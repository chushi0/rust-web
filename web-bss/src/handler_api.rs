use crate::biz;
use crate::model::Model;
use rocket::serde::json::Json;

#[get("/home/events")]
pub async fn home_events() -> Json<Model<biz::home::GetEventsResp>> {
    Json(biz::home::get_events().await.unwrap_or_else(|e| {
        log::error!("handle home_events error: {e}");
        Model::new_error()
    }))
}

#[post("/user/new")]
pub async fn user_new() -> Json<Model<biz::user::NewUserResp>> {
    Json(biz::user::new_user().await.unwrap_or_else(|e| {
        log::error!("handle user_new error: {e}");
        Model::new_error()
    }))
}

#[get("/console/mc/playerdata/advancement?<name>")]
async fn mc_playerdata_advancement(name: &str) -> Json<Model<biz::mc::GetPlayerAdvancementResp>> {
    Json(
        biz::mc::get_player_advancement(name)
            .await
            .unwrap_or_else(|e| {
                log::error!("handle error: {e}");
                Model::new_error()
            }),
    )
}

#[get("/console/mc/globaldata/advancement")]
async fn mc_globaldata_advancement() -> Json<Model<biz::mc::GetAdvancementResp>> {
    Json(biz::mc::get_advancement_config().await.unwrap_or_else(|e| {
        log::error!("handle error: {e}");
        Model::new_error()
    }))
}