use sqlx::{ColumnIndex, Decode, FromRow, Row, Type};

#[derive(Debug, Clone, Default)]
pub struct Counter {
    pub count: i64,
}

impl<'r, R> FromRow<'r, R> for Counter
where
    R: Row,
    usize: ColumnIndex<R>,
    i64: Decode<'r, R::Database> + Type<R::Database>,
{
    fn from_row(row: &'r R) -> Result<Self, sqlx::Error> {
        Ok(Counter {
            count: row.try_get(0)?,
        })
    }
}
