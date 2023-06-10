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
