pub(crate) mod component;
pub(crate) mod config;
pub(crate) mod model;
pub(crate) mod page;
pub(crate) mod sys;

use crate::page::*;
use web_sys::window;
use yew::prelude::*;
use yew_router::prelude::*;

#[derive(Clone, Copy, Routable, PartialEq)]
enum Route {
    #[at("/")]
    Home,
    #[at("/game/number-tower")]
    GameNumberTower,
    #[at("/dev-tools")]
    DevTools,
    #[at("/totp")]
    Totp,
    #[at("/manage/mc-server")]
    ManageMcServer,
    #[at("/config")]
    LocalConfig,
    #[not_found]
    #[at("/404")]
    NotFound,
}

fn switch(routes: Route) -> Html {
    match routes {
        Route::Home => html! { <HomePage /> },
        Route::NotFound => html! { <NotFoundPage /> },
        Route::GameNumberTower => html! { <GameNumberTowerPage/> },
        Route::DevTools => html! { <DevToolsPage /> },
        Route::Totp => html! { <TotpPage /> },
        Route::ManageMcServer => html! { <McServerManagePage /> },
        Route::LocalConfig => html! { <LocalConfigPage /> },
    }
}

#[function_component]
fn App() -> Html {
    html! {
        <BrowserRouter>
            <Switch<Route> render={switch} />
        </BrowserRouter>
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    let root = window()
        .expect("window is not initialized")
        .document()
        .expect("document is not initialized")
        .get_element_by_id("app")
        .expect("root element is not initialized");
    yew::Renderer::<App>::with_root(root).render();
}
