use anyhow::*;
use web_sys::window;
use yew::prelude::*;

#[derive(PartialEq, Properties)]
pub struct TitleProps {
    pub title: AttrValue,
}

#[function_component]
pub fn Title(props: &TitleProps) -> Html {
    if let Err(e) = set_title(props.title.clone()) {
        log::error!("set title error: {e}");
    }

    html! {
        <></>
    }
}

fn set_title(title: AttrValue) -> Result<(), Error> {
    window()
        .ok_or(anyhow!("window not found"))?
        .document()
        .ok_or(anyhow!("document not found"))?
        .set_title(title.as_str());
    Ok(())
}
