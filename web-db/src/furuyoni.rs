use anyhow::{anyhow, Result};
use futures::TryStreamExt;
use sqlx::{FromRow, Row};

#[derive(Debug, Clone, FromRow)]
pub struct Character {
    pub rowid: i64,
    pub name: String,
    pub description: String,
    pub image_uri: String,
    pub attack_distance_json: String,
    pub special_effect_json: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct Card {
    pub rowid: i64,
    pub name: String,
    pub derivative: bool, // 衍生物
    pub tag: String,
    pub ref_character_id: i64,
    pub card_info_json: String,
    pub special_effect_json: String,
}

pub async fn get_all_characters(db: &mut super::Transaction<'_>) -> Result<Vec<Character>> {
    let mut iter = sqlx::query_as("select * from character").fetch(&mut db.tx);
    let mut res = Vec::new();
    while let Some(row) = iter.try_next().await? {
        res.push(row);
    }
    Ok(res)
}

pub async fn get_characters_by_ids(
    db: &mut super::Transaction<'_>,
    id: i64,
) -> Result<Option<Character>> {
    let character: Result<Character, sqlx::Error> =
        sqlx::query_as("select * from character where rowid = ?")
            .bind(id)
            .fetch_one(&mut db.tx)
            .await;

    match character {
        Ok(character) => Ok(Some(character)),
        Err(error) => {
            if let sqlx::Error::RowNotFound = error {
                Ok(None)
            } else {
                Err(anyhow!("{error}"))
            }
        }
    }
}

pub async fn get_cards_by_character(
    db: &mut super::Transaction<'_>,
    character_id: i64,
) -> Result<Vec<Card>> {
    let mut iter = sqlx::query_as("select * from card where ref_character_id = ?")
        .bind(character_id)
        .fetch(&mut db.tx);
    let mut res = Vec::new();
    while let Some(row) = iter.try_next().await? {
        res.push(row);
    }
    Ok(res)
}
