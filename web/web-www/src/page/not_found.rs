use yew::prelude::*;

use crate::component::{NavBar, Title};

#[function_component]
pub fn NotFoundPage() -> Html {
    html! {
        <>
            <Title title="404 - Not Found" />
            <NavBar active=""/>

            <div class="container-sm" style="padding-top: 2em; padding-bottom: 2em; text-align: center">
                <img src="https://http.cat/404.jpg" class="img-fluid" alt="http.cat/404" />
            </div>
        </>
    }
}
