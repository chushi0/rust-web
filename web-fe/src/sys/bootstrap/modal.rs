use wasm_bindgen::prelude::*;
use web_sys::Element;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "bootstrap.Modal")]
    #[derive(Clone, Debug)]
    pub type Modal;

    #[wasm_bindgen(constructor, js_class = "bootstrap.Modal")]
    pub fn new_with_element(element: &Element) -> Modal;

    #[wasm_bindgen(method)]
    pub fn show(this: &Modal);

    #[wasm_bindgen(method)]
    pub fn hide(this: &Modal);
}
