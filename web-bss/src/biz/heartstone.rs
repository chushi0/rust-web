use crate::model::Model;
use anyhow::Result;
use rocket::serde::Serialize;
use web_db::hearthstone::{get_all_cards, get_all_resources};
use web_db::{begin_tx, create_connection, RDS};

#[derive(Debug, Clone, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct AssetFile {
    pub uri: String,
    pub md5: String,
    pub sha1: String,
    pub size: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct HeartstoneCard {
    pub code: String,
    pub name: String,
    pub card_type: i32,
    pub mana_cost: i32,
    pub derive: bool,
    pub need_select_target: bool,
    pub card_info: String,
    pub description: String,
    pub resources: String,
}

const CARD_RESOURCE_PREFIX: &str = "game_assets/heartstone/";

pub type AssetsListResp = Vec<AssetFile>;
pub async fn heartstone_assets_list() -> Result<Model<AssetsListResp>> {
    let mut conn = create_connection(RDS::Hearthstone).await?;
    let mut tx = begin_tx(&mut conn).await?;

    let resources = get_all_resources(&mut tx).await?;
    let data = resources
        .into_iter()
        .map(|resources| AssetFile {
            uri: format!("{CARD_RESOURCE_PREFIX}{}", resources.uri),
            md5: resources.md5,
            sha1: resources.sha1,
            size: resources.size,
        })
        .collect();

    Ok(Model::from_success(data))
}

pub type GetHeartstoneCardsResp = Vec<HeartstoneCard>;
pub async fn get_heartstone_cards() -> Result<Model<GetHeartstoneCardsResp>> {
    let mut conn = create_connection(RDS::Hearthstone).await?;
    let mut tx = begin_tx(&mut conn).await?;

    let cards = get_all_cards(&mut tx).await?;
    let data = cards
        .into_iter()
        .map(|card| HeartstoneCard {
            code: card.code,
            name: card.name,
            card_type: card.card_type,
            mana_cost: card.mana_cost,
            derive: card.derive,
            need_select_target: card.need_select_target,
            card_info: card.card_info,
            description: card.description,
            resources: format!("{CARD_RESOURCE_PREFIX}{}", card.resources),
        })
        .collect();

    Ok(Model::from_success(data))
}
