use yew::prelude::*;
use yew_router::prelude::Redirect;

#[function_component]
pub fn NotFoundPage() -> Html {
    html! {
        <Redirect<crate::Route> to={crate::Route::Home} />
    }
}
