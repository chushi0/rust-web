use std::net::ToSocketAddrs;

use crate::biz;
use crate::model::Model;
use rocket::serde::json::Json;

#[get("/home/events")]
pub async fn home_events() -> Json<Model<biz::home::GetEventsResp>> {
    Json(biz::home::get_events().await.unwrap_or_else(|e| {
        log::error!("handle home_events error: {e} {}", e.backtrace());
        Model::new_error()
    }))
}

#[get("/testdns?<host>")]
pub async fn testdns(host: &str) -> String {
    format!("{:#?}", host.to_socket_addrs())
}

#[post("/user/new")]
pub async fn user_new() -> Json<Model<biz::user::NewUserResp>> {
    Json(biz::user::new_user().await.unwrap_or_else(|e| {
        log::error!("handle user_new error: {e} {}", e.backtrace());
        Model::new_error()
    }))
}

#[get("/console/mc/playerdata/advancement?<name>")]
pub async fn mc_playerdata_advancement(
    name: &str,
) -> Json<Model<biz::mc::GetPlayerAdvancementResp>> {
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
pub async fn mc_globaldata_advancement() -> Json<Model<biz::mc::GetAdvancementResp>> {
    Json(biz::mc::get_advancement_config().await.unwrap_or_else(|e| {
        log::error!("handle error: {e}");
        Model::new_error()
    }))
}

#[get("/game/heartstone/assets_list")]
pub async fn heartstone_assets_list() -> Json<Model<biz::heartstone::AssetsListResp>> {
    Json(
        biz::heartstone::heartstone_assets_list()
            .await
            .unwrap_or_else(|e| {
                log::error!("handle error: {e}");
                Model::new_error()
            }),
    )
}

#[get("/game/heartstone/cards")]
pub async fn get_heartstone_cards() -> Json<Model<biz::heartstone::GetHeartstoneCardsResp>> {
    Json(
        biz::heartstone::get_heartstone_cards()
            .await
            .unwrap_or_else(|e| {
                log::error!("handle error: {e}");
                Model::new_error()
            }),
    )
}

#[get("/oss-file-obtain?<uri>")]
pub async fn oss_file_obtain(uri: String) -> Json<Model<biz::oss::OssFileObtainResp>> {
    Json(biz::oss::oss_file_obtain(uri).await.unwrap_or_else(|e| {
        log::error!("handle error: {e}");
        Model::new_error()
    }))
}
