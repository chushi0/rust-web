use crate::component::Canvas;
use std::{cell::RefCell, rc::Rc};
use yew::prelude::*;

pub struct SceneDelegate {
    imp: Rc<RefCell<dyn Scene>>,
}

pub struct ObjectDelegate {
    imp: Rc<RefCell<dyn Object>>,
}

pub trait Scene {
    fn on_create(&mut self, delegate: SceneDelegate) {}
    fn on_destroy(&mut self, delegate: SceneDelegate) {}
}

pub trait Object {
    fn draw(&mut self, delegate: ObjectDelegate);
}

pub struct GameCanvas;

pub enum GameCanvasMsg {}

impl Component for GameCanvas {
    type Message = GameCanvasMsg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        Self
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <Canvas />
        }
    }
}
