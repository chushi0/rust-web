pub(crate) mod component;
pub(crate) mod model;
pub(crate) mod page;

use crate::page::*;
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
    yew::Renderer::<App>::new().render();
}
