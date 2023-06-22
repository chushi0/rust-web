use anyhow::Result;
use futures::TryStreamExt;
use sqlx::Row;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Advancement {
    pub rowid: i64,
    pub id: String,
    pub title: String,
    pub description: String,
    pub icon: Option<String>,
    pub frame: String,
    pub parent: Option<String>,
    pub requirements: String,
}

pub async fn get_all_advancement(db: &mut super::Transaction<'_>) -> Result<Vec<Advancement>> {
    let mut iter = sqlx::query_as("select rowid, * from advancement").fetch(&mut db.tx);
    let mut res = Vec::new();
    while let Some(row) = iter.try_next().await? {
        res.push(row)
    }
    Ok(res)
}

pub async fn delete_all_advancement(db: &mut super::Transaction<'_>) -> Result<()> {
    sqlx::query("delete from advancement")
        .execute(&mut db.tx)
        .await?;

    Ok(())
}

pub async fn insert_advancement(
    db: &mut super::Transaction<'_>,
    advancement: &mut Advancement,
) -> Result<()> {
    sqlx::query("insert into advancement (id, title, description, icon, frame, parent, requirements) values (?, ?, ?, ?, ?, ?, ?)")
    .bind(&advancement.id)
    .bind(&advancement.title)
    .bind(&advancement.description)
    .bind(&advancement.icon)
    .bind(&advancement.frame)
    .bind(&advancement.parent)
    .bind(&advancement.requirements)
    .execute(&mut db.tx)
    .await?;

    let id: i64 = sqlx::query("select last_insert_rowid()")
        .fetch_one(&mut db.tx)
        .await?
        .get(0);

    advancement.rowid = id;

    Ok(())
}
