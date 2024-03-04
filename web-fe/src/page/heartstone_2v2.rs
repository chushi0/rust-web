use crate::component::*;
use yew::prelude::*;

pub struct Heartstone2V2Page {
    console_buffer: Vec<AttrValue>,
}

pub enum Heartstone2V2Msg {
    NewConsoleMsg(AttrValue),
}

impl Component for Heartstone2V2Page {
    type Message = Heartstone2V2Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            console_buffer: Vec::new(),
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Heartstone2V2Msg::NewConsoleMsg(msg) => {
                self.console_buffer.push(msg);
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_click = {
            let link = ctx.link().clone();
            Callback::from(move |_| {
                link.send_message(Heartstone2V2Msg::NewConsoleMsg("info".into()))
            })
        };

        html! {
            <>
                <Title title="炉石传说 2V2" />
                <NavBar active="heartstone-2v2" />
                <div class="container-sm">
                    <h3>{"炉石传说 2V2"}</h3>

                    <div style="border-color: black; border-width: thin; border-style: solid; border-radius: 0.5em; padding: 0.5em; width: 100%; height: 400px; overflow-y: scroll">
                        {self.console_buffer.iter().map(|info| html!{
                            <p style="margin-top: 0; margin-bottom: 0">{info.clone()}</p>
                        }).collect::<Html>()}
                    </div>
                    <button onclick={on_click}>{"Debug"}</button>
                </div>
            </>
        }
    }
}
