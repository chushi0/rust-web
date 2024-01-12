use crate::ws::{game, WsBiz, WsBizFactory};

pub struct GameBizFactory;

impl WsBizFactory for GameBizFactory {
    fn create_if_match(
        &self,
        request: &crate::util::http_decode::HttpRequest,
        con: crate::ws::WsCon,
    ) -> Option<Box<dyn WsBiz + Send>> {
        if request.path == "/ws/game" {
            Some(Box::new(game::GameBiz::create(con)))
        } else {
            None
        }
    }
}
