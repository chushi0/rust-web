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

#[launch]
fn rocket() -> _ {
    let routes = routes![home_events];
    rocket::build().mount("/api/", routes)
}
