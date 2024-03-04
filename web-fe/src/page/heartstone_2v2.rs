use crate::component::*;
use yew::prelude::*;

pub struct Heartstone2V2Page;

pub enum Heartstone2V2Msg {}

impl Component for Heartstone2V2Page {
    type Message = Heartstone2V2Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        Self
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <>
                <Title title="炉石传说 2V2" />
                <NavBar active="heartstone-2v2" />
                <div class="container-sm">
                    <h3>{"炉石传说 2V2"}</h3>
                </div>
            </>
        }
    }
}
