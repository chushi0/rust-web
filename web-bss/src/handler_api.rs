use crate::biz;
use crate::model::Model;
use rocket::serde::json::Json;

#[get("/home/events")]
pub async fn home_events() -> Json<Model<biz::home::GetEventsResp>> {
    Json(biz::home::get_events().await.unwrap_or_else(|e| {
        log::error!("handle error: {e}");
        Model::new_error()
    }))
}
