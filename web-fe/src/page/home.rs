use crate::component::*;
use crate::model::home::*;
use crate::model::*;
use gloo_net::Error;
use std::rc::Rc;
use time::format_description;
use yew::prelude::*;
use yew::suspense::*;

#[derive(PartialEq, Properties)]
struct NewsCardProps {
    #[prop_or_default]
    time: AttrValue,
    #[prop_or_default]
    title: AttrValue,
    #[prop_or_default]
    msg: AttrValue,
    #[prop_or_default]
    link: AttrValue,

    #[prop_or_default]
    loading: bool,
}

#[function_component]
fn EventCard(props: &NewsCardProps) -> Html {
    if props.loading {
        return html! {
            <div class="row">
                <div class="col-md-2 placeholder-glow" style="text-align: right; border-right: solid silver;">
                    <span class="placeholder">{"8888-88-88"}</span>
                </div>
                <div class="col" style="padding-bottom: 1.5em">
                    <div class="card">
                        <div class="card-body">
                            <h5 class="placeholder-glow">
                                <span class="placeholder col-6"></span>
                            </h5>
                            <p class="card-text placeholder-glow">
                                <span class="placeholder col-7"></span>
                                <span class="placeholder col-4"></span>
                                <span class="placeholder col-4"></span>
                                <span class="placeholder col-6"></span>
                                <span class="placeholder col-8"></span>
                            </p>
                            <a class="card-link placeholder-glow disable">
                                <span class="placeholder">{"XXXXXXXX"}</span>
                            </a>
                        </div>
                    </div>
                </div>
            </div>
        };
    }

    html! {
        <div class="row">
            <div class="col-md-2" style="text-align: right; border-right: solid silver;">
                {props.time.clone()}
            </div>
            <div class="col" style="padding-bottom: 1.5em">
                <div class="card">
                    <div class="card-body">
                        <h5>{props.title.clone()}</h5>
                        <p class="card-text">{props.msg.clone()}</p>
                        <a class="card-link" href={props.link.clone()} target="_blank">
                            {"查看详情→"}
                        </a>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[function_component]
fn EventDetails() -> HtmlResult {
    let news = use_events()?;
    let time_format = format_description::parse("[year]-[month]-[day]").unwrap();

    Ok(html! {
        <>
            {news.iter().map(|data| html!{
                <EventCard
                    time={data.time.clone().date().format(&time_format).unwrap_or("".to_string())}
                    title={data.title.clone()}
                    msg={data.msg.clone()}
                    link={data.link.clone()} />
            }).collect::<Html>()}
            <hr/>
            <p class="text-center text-muted">{"更早的动态已不可见"}</p>
        </>
    })
}

#[hook]
fn use_events() -> SuspensionResult<Rc<Vec<EventData>>> {
    let data: UseStateHandle<Option<Rc<Vec<EventData>>>> = use_state(|| None);
    // save the handle to prevent refresh component
    let state_handle = use_mut_ref(|| None);

    match &*data {
        Some(v) => Ok(v.clone()),
        None => {
            let (s, handle) = Suspension::new();
            *state_handle.borrow_mut() = Some(handle);

            let data = data.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let news = load_events().await;
                match news {
                    Ok(fetch_data) => {
                        data.set(Some(Rc::new(fetch_data)));
                        *state_handle.borrow_mut() = None;
                    }
                    Err(e) => {
                        log::error!("load events error: {}", e);
                    }
                }
            });

            Err(s)
        }
    }
}

async fn load_events() -> Result<Vec<EventData>, Error> {
    Ok(gloo_net::http::Request::get("/api/home/events")
        .send()
        .await?
        .json::<Model<Vec<EventData>>>()
        .await?
        .data
        .ok_or(Error::GlooError("empty data".to_string()))?)
}

#[function_component]
pub fn HomePage() -> Html {
    let loading = html! {
        <>
            <EventCard loading=true />
            <EventCard loading=true />
            <EventCard loading=true />
            <EventCard loading=true />
            <EventCard loading=true />
        </>
    };

    html! {
        <>
            <Title title="动态" />
            <NavBar active="home"/>
            <div class="container-sm" style="padding-top: 1em; padding-bottom: 1em;">
                <Suspense fallback={loading}>
                    <EventDetails />
                </Suspense>
            </div>
        </>
    }
}
