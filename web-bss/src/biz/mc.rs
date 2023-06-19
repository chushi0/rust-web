use crate::model::mc::AdvancementData;
use crate::model::Model;
use crate::service::mc;
use anyhow::Result;

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
