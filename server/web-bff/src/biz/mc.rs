use crate::model::mc::{AdvancementConfig, AdvancementData};
use crate::model::Model;
use crate::service::mc;
use anyhow::Result;
use web_db::mc_config::get_all_advancement;
use web_db::{begin_tx, create_connection, RDS};

pub type GetPlayerAdvancementResp = Vec<AdvancementData>;
pub async fn get_player_advancement(name: &str) -> Result<Model<GetPlayerAdvancementResp>> {
    let uuid = mc::get_player_uuid(name).await?;
    let advancements = mc::get_player_advancement(&uuid).await?;

    let mut resp_data = vec![];
    for (id, advancement) in advancements {
        let mut done_criteria = vec![];
        for (criteria, _) in advancement.criteria {
            done_criteria.push(criteria);
        }
        resp_data.push(AdvancementData {
            id,
            done_criteria,
            done: advancement.done,
        })
    }

    Ok(Model::from_success(resp_data))
}

pub type GetAdvancementResp = Vec<AdvancementConfig>;
pub async fn get_advancement_config() -> Result<Model<GetAdvancementResp>> {
    let mut conn = create_connection(RDS::McConfig).await?;
    let mut tx = begin_tx(&mut conn).await?;

    let advancements = get_all_advancement(&mut tx).await?;

    let mut res = vec![];
    for advancement in advancements {
        res.push(AdvancementConfig {
            id: advancement.id.clone(),
            title: advancement.title.clone(),
            description: advancement.description.clone(),
            icon: advancement.icon.clone(),
            frame: advancement.frame.clone(),
            parent: advancement.parent.clone(),
            requirements: serde_json::from_str(&advancement.requirements)?,
        })
    }

    Ok(Model::from_success(res))
}
