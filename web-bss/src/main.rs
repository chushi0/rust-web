use crate::model::Model;
use rocket::serde::json::Json;

#[macro_use]
extern crate rocket;

mod biz;
pub(crate) mod model;
pub(crate) mod service;

#[get("/home/events")]
async fn home_events() -> Json<Model<biz::home::GetEventsResp>> {
    Json(biz::home::get_events().await.unwrap_or_else(|e| {
        log::error!("handle error: {e}");
        Model::new_error()
    }))
}

#[get("/console/mc/playerdata?<name>")]
async fn mc_playerdata(name: &str) -> Json<Model<biz::mc::GetPlayerAdvancementResp>> {
    Json(
        biz::mc::get_player_advancement(name)
            .await
            .unwrap_or_else(|e| {
                log::error!("handle error: {e}");
                Model::new_error()
            }),
    )
}

#[launch]
fn rocket() -> _ {
    let routes = routes![home_events, mc_playerdata];
    rocket::build().mount("/api/", routes)
}
