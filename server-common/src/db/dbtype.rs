use sqlx::{Database, Decode, Encode, Type};
use std::convert::TryFrom;
use thiserror::Error;

pub trait DBType<DB>: TryFrom<Self::Alias>
where
    DB: Database,
    <Self as TryFrom<Self::Alias>>::Error: std::fmt::Display,
{
    type Alias: Type<DB> + for<'q> Encode<'q, DB> + for<'q> Decode<'q, DB> + for<'r> From<&'r Self>;
}

#[derive(Debug, Error)]
pub enum DBTypeConvertError {
    #[error("{0}")]
    Anyhow(#[source] anyhow::Error),
    #[error("{0}")]
    BoxStd(#[source] Box<dyn std::error::Error + Send + Sync>),
}

#[macro_export]
macro_rules! impl_sqlx_type {
    ($t:ty, $a:ty) => {
        impl<DB: sqlx::Database> $crate::db::dbtype::DBType<DB> for $t
        where
            $a: sqlx::Type<DB>
                + for<'q> sqlx::Encode<'q, DB>
                + for<'q> sqlx::Decode<'q, DB>
                + for<'r> From<&'r Self>,
        {
            type Alias = $a;
        }

        impl<DB: sqlx::Database> sqlx::Type<DB> for $t
        where
            $a: sqlx::Type<DB>,
        {
            fn type_info() -> DB::TypeInfo {
                <$a as sqlx::Type<DB>>::type_info()
            }

            fn compatible(ty: &DB::TypeInfo) -> bool {
                <$a as sqlx::Type<DB>>::compatible(ty)
            }
        }

        impl<'q, DB: sqlx::Database> sqlx::Encode<'q, DB> for $t
        where
            $a: sqlx::Encode<'q, DB> + sqlx::Type<DB> + for<'r> From<&'r $t>,
        {
            fn encode_by_ref(
                &self,
                buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
            ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
                let alias: $a = self.into();
                alias.encode_by_ref(buf)
            }
        }

        impl<'q, DB: sqlx::Database> sqlx::Decode<'q, DB> for $t
        where
            $a: sqlx::Decode<'q, DB> + sqlx::Type<DB>,
            $t: TryFrom<$a>,
            <$t as TryFrom<$a>>::Error: std::fmt::Display + Send + Sync + 'static,
        {
            fn decode(
                value: <DB as sqlx::Database>::ValueRef<'q>,
            ) -> Result<Self, sqlx::error::BoxDynError> {
                let alias_val = <$a as sqlx::Decode<'q, DB>>::decode(value)?;
                <$t>::try_from(alias_val).map_err(|e| Box::new(e) as sqlx::error::BoxDynError)
            }
        }
    };
}
