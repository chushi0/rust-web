use std::fmt::Debug;

use futures::future::BoxFuture;
use sqlx::{Connection, Database, Executor, Pool, Transaction};

#[derive(Debug)]
pub enum Context<'a, DB: Database> {
    Pool(Pool<DB>),
    PoolRef(&'a Pool<DB>),
    Connection(DB::Connection),
    Transaction(Transaction<'a, DB>),
}

pub type ContextRef<'a, 'b, DB> = &'a mut Context<'b, DB>;

impl<'a, DB: Database> Context<'a, DB> {
    pub async fn begin<'b>(&'b mut self) -> Result<Context<'b, DB>, sqlx::Error> {
        Ok(Context::Transaction(match self {
            Context::Pool(pool) => pool.begin().await?,
            Context::PoolRef(pool) => pool.begin().await?,
            Context::Connection(connection) => connection.begin().await?,
            Context::Transaction(transaction) => transaction.begin().await?,
        }))
    }

    pub async fn commit(self) -> Result<(), sqlx::Error> {
        if let Context::Transaction(transaction) = self {
            transaction.commit().await?;
        }

        Ok(())
    }

    pub async fn rollback(self) -> Result<(), sqlx::Error> {
        if let Context::Transaction(transaction) = self {
            transaction.rollback().await?;
        }

        Ok(())
    }
}

impl<'a, 'b, DB> Executor<'a> for ContextRef<'a, 'b, DB>
where
    DB: Database,
    DB::Connection: Debug,
    for<'c> &'c Pool<DB>: Executor<'c, Database = DB>,
    for<'c> &'c mut DB::Connection: Executor<'c, Database = DB>,
    'b: 'a,
{
    type Database = DB;

    fn execute<'e, 'q: 'e, E>(
        self,
        query: E,
    ) -> BoxFuture<'e, Result<<Self::Database as Database>::QueryResult, sqlx::Error>>
    where
        'a: 'e,
        E: 'q + sqlx::Execute<'q, Self::Database>,
    {
        match self {
            Context::Pool(pool) => pool.execute(query),
            Context::PoolRef(pool) => pool.execute(query),
            Context::Connection(connection) => connection.execute(query),
            Context::Transaction(transaction) => transaction.execute(query),
        }
    }

    fn fetch_many<'e, 'q: 'e, E>(
        self,
        query: E,
    ) -> futures::stream::BoxStream<
        'e,
        Result<
            sqlx::Either<
                <Self::Database as Database>::QueryResult,
                <Self::Database as Database>::Row,
            >,
            sqlx::Error,
        >,
    >
    where
        'a: 'e,
        E: 'q + sqlx::Execute<'q, Self::Database>,
    {
        match self {
            Context::Pool(pool) => pool.fetch_many(query),
            Context::PoolRef(pool) => pool.fetch_many(query),
            Context::Connection(connection) => connection.fetch_many(query),
            Context::Transaction(transaction) => transaction.fetch_many(query),
        }
    }

    fn fetch_optional<'e, 'q: 'e, E>(
        self,
        query: E,
    ) -> BoxFuture<'e, Result<Option<<Self::Database as Database>::Row>, sqlx::Error>>
    where
        'a: 'e,
        E: 'q + sqlx::Execute<'q, Self::Database>,
    {
        match self {
            Context::Pool(pool) => pool.fetch_optional(query),
            Context::PoolRef(pool) => pool.fetch_optional(query),
            Context::Connection(connection) => connection.fetch_optional(query),
            Context::Transaction(transaction) => transaction.fetch_optional(query),
        }
    }

    fn prepare_with<'e, 'q: 'e>(
        self,
        sql: &'q str,
        parameters: &'e [<Self::Database as Database>::TypeInfo],
    ) -> BoxFuture<'e, Result<<Self::Database as Database>::Statement<'q>, sqlx::Error>>
    where
        'a: 'e,
    {
        match self {
            Context::Pool(pool) => pool.prepare_with(sql, parameters),
            Context::PoolRef(pool) => pool.prepare_with(sql, parameters),
            Context::Connection(connection) => connection.prepare_with(sql, parameters),
            Context::Transaction(transaction) => transaction.prepare_with(sql, parameters),
        }
    }

    fn describe<'e, 'q: 'e>(
        self,
        sql: &'q str,
    ) -> BoxFuture<'e, Result<sqlx::Describe<Self::Database>, sqlx::Error>>
    where
        'a: 'e,
    {
        match self {
            Context::Pool(pool) => pool.describe(sql),
            Context::PoolRef(pool) => pool.describe(sql),
            Context::Connection(connection) => connection.describe(sql),
            Context::Transaction(transaction) => transaction.describe(sql),
        }
    }
}
