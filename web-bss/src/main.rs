use std::collections::HashMap;

use crate::model::Model;
use rocket::{serde::json::Json, Request};

#[macro_use]
extern crate rocket;

mod biz;
pub mod model;
pub mod service;

#[get("/home/events")]
async fn home_events() -> Json<Model<biz::home::GetEventsResp>> {
    Json(biz::home::get_events().await.unwrap_or_else(|e| {
        log::error!("handle error: {e}");
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

#[catch(500)]
async fn generate_error(req: &Request<'_>) {
    let warn_msg = {
        let method = req.method();
        let path = req.uri().path();
        let uri = req.uri();

        format!("[ Method ] {method}\n[ Path ] {path}\n [ Uri ] {uri}")
    };

    let notification_params = {
        let mut map = HashMap::new();
        map.insert("warn_title".to_string(), "HTTP接口500报警".to_string());
        map.insert("warn_msg".to_string(), warn_msg);
        map
    };
    let result = feishu_api::sdk::send_card_message("ctp_AAwmLGzY0vLz", notification_params).await;
    if let Err(err) = result {
        log::error!("send notification error! err={err}")
    }
}

#[launch]
fn rocket() -> _ {
    let routes = routes![
        home_events,
        mc_playerdata_advancement,
        mc_globaldata_advancement
    ];
    rocket::build()
        .register("/", catchers![generate_error])
        .mount("/api/", routes)
}
