use std::fmt::Display;

use sqlx::{Database, Encode, QueryBuilder, Type};

pub trait BoundSeparatedHelper<'args, DB>
where
    DB: Database,
{
    fn bound_separated<'qb, Begin, End, Sep>(
        &'qb mut self,
        begin: Begin,
        end: End,
        separator: Sep,
    ) -> BoundSeparated<'qb, 'args, DB, Begin, End, Sep>
    where
        'args: 'qb,
        Begin: Display,
        End: Display,
        Sep: Display;
}

pub struct BoundSeparated<'qb, 'args: 'qb, DB, Begin, End, Sep>
where
    DB: Database,
    Begin: Display,
    End: Display,
    Sep: Display,
{
    query_builder: &'qb mut QueryBuilder<'args, DB>,
    begin: Begin,
    end: End,
    separator: Sep,
    push_separator: bool,
}

impl<'args, DB> BoundSeparatedHelper<'args, DB> for QueryBuilder<'args, DB>
where
    DB: Database,
{
    fn bound_separated<'qb, Begin, End, Sep>(
        &'qb mut self,
        begin: Begin,
        end: End,
        separator: Sep,
    ) -> BoundSeparated<'qb, 'args, DB, Begin, End, Sep>
    where
        'args: 'qb,
        Begin: Display,
        End: Display,
        Sep: Display,
    {
        BoundSeparated {
            query_builder: self,
            begin,
            end,
            separator,
            push_separator: false,
        }
    }
}

impl<'qb, 'args: 'qb, DB, Begin, End, Sep> BoundSeparated<'qb, 'args, DB, Begin, End, Sep>
where
    DB: Database,
    Begin: Display,
    End: Display,
    Sep: Display,
{
    pub fn push(&mut self, sql: impl Display) -> &mut Self {
        if self.push_separator {
            self.query_builder
                .push(format_args!("{}{}", self.separator, sql));
        } else {
            self.query_builder
                .push(format_args!("{}{}", self.begin, sql));
            self.push_separator = true;
        }

        self
    }

    pub fn push_unseparated(&mut self, sql: impl Display) -> &mut Self {
        self.query_builder.push(sql);
        self
    }

    pub fn push_bind<T>(&mut self, value: T) -> &mut Self
    where
        T: 'args + Encode<'args, DB> + Type<DB>,
    {
        if self.push_separator {
            self.query_builder.push(&self.separator);
        } else {
            self.query_builder.push(&self.begin);
        }

        self.query_builder.push_bind(value);
        self.push_separator = true;

        self
    }

    pub fn push_bind_unseparated<T>(&mut self, value: T) -> &mut Self
    where
        T: 'args + Encode<'args, DB> + Type<DB>,
    {
        self.query_builder.push_bind(value);
        self
    }
}

impl<'qb, 'args, DB, Begin, End, Sep> Drop for BoundSeparated<'qb, 'args, DB, Begin, End, Sep>
where
    DB: Database,
    Begin: Display,
    End: Display,
    Sep: Display,
{
    fn drop(&mut self) {
        if self.push_separator {
            self.query_builder.push(&self.end);
        }
    }
}
