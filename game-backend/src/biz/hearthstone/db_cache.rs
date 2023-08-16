use anyhow::Result;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use web_db::hearthstone::{Card, CardInfo};

#[derive(Debug)]
pub struct DbCardCache {
    pub card: Card,
    pub card_info: CardInfo,
}

lazy_static::lazy_static! {
    static ref DB_CACHE: RwLock<HashMap<String, Arc<DbCardCache>>> = RwLock::new(HashMap::new());
}

pub async fn get_cache_card(code: String) -> Result<Arc<DbCardCache>> {
    {
        let read = DB_CACHE.read().await;
        let data = read.get(&code);
        if let Some(data) = data {
            return Ok(data.clone());
        }
    }

    let mut conn = web_db::create_connection(web_db::RDS::Hearthstone).await?;
    let mut tx = web_db::begin_tx(&mut conn).await?;
    let card = web_db::hearthstone::get_card_by_code(&mut tx, &code).await?;
    let card_info = serde_json::from_str(&card.card_info)?;
    let card = Arc::new(DbCardCache { card, card_info });

    let mut write = DB_CACHE.write().await;
    let data = write.get(&code);
    if let Some(data) = data {
        return Ok(data.clone());
    }
    write.insert(code, card.clone());

    Ok(card)
}
