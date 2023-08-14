use instant::Instant;
use wasm_bindgen::prelude::*;
use web_sys::{window, HtmlCanvasElement};
use yew::{html::Scope, prelude::*};

pub struct Canvas {
    canvas_node: NodeRef,
    cb: Closure<dyn FnMut()>,
    last_refresh_time: Instant,
}

#[derive(PartialEq, Properties)]
pub struct CanvasProps {
    pub id: AttrValue,
    pub width: usize,
    pub height: usize,
    pub contexttype: CanvasContextType,
    pub autorefresh: bool,
    pub oninit: Callback<CanvasContext>,
    pub onrender: Callback<CanvasContext>,
    pub onmousedown: Callback<CanvasMouseEvent>,
    pub onmouseup: Callback<CanvasMouseEvent>,
}

pub struct CanvasContext {
    pub canvas_handler: CanvasHandler,
    pub context: js_sys::Object,
    pub width: usize,
    pub height: usize,
    pub delay: f64,
}

pub struct CanvasMouseEvent {
    pub canvas_handler: CanvasHandler,
    pub x: f64,
    pub y: f64,
    pub width: usize,
    pub height: usize,
}

#[derive(Clone)]
pub struct CanvasHandler {
    link: Scope<Canvas>,
}

#[derive(Debug, PartialEq)]
pub enum CanvasContextType {
    Type2D,
    // TypeWebGL,
    // TypeWebGL2,
}

pub enum CanvasMsg {
    Init,
    Render,
}

impl Component for Canvas {
    type Message = CanvasMsg;
    type Properties = CanvasProps;

    fn create(ctx: &Context<Self>) -> Self {
        let link = ctx.link().clone();
        let cb = Closure::wrap(
            Box::new(move || link.send_message(CanvasMsg::Render)) as Box<dyn FnMut()>
        );

        ctx.link().send_message(CanvasMsg::Init);

        Self {
            canvas_node: NodeRef::default(),
            cb,
            last_refresh_time: Instant::now(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            CanvasMsg::Init => self.on_init(ctx),
            CanvasMsg::Render => self.on_render(ctx),
        }
        false
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let onmousedown = {
            let link = ctx.link().clone();
            let raw_onmousedown = ctx.props().onmousedown.clone();
            let canvas_node = self.canvas_node.clone();
            let width = ctx.props().width;
            let height = ctx.props().height;
            Callback::from(move |raw_event: MouseEvent| {
                let canvas: HtmlCanvasElement = canvas_node.cast().expect("canvas element");
                let dom_width = canvas.offset_width() as usize;
                let dom_height = canvas.offset_height() as usize;
                let x = raw_event.offset_x() as f64 / dom_width as f64 * width as f64;
                let y = raw_event.offset_y() as f64 / dom_height as f64 * height as f64;
                raw_onmousedown.emit(CanvasMouseEvent {
                    canvas_handler: CanvasHandler { link: link.clone() },
                    x,
                    y,
                    width,
                    height,
                })
            })
        };
        let onmouseup = {
            let link = ctx.link().clone();
            let raw_onmouseup = ctx.props().onmouseup.clone();
            let canvas_node = self.canvas_node.clone();
            let width = ctx.props().width;
            let height = ctx.props().height;
            Callback::from(move |raw_event: MouseEvent| {
                let canvas: HtmlCanvasElement = canvas_node.cast().expect("canvas element");
                let dom_width = canvas.offset_width() as usize;
                let dom_height = canvas.offset_height() as usize;
                let x = raw_event.offset_x() as f64 / dom_width as f64 * width as f64;
                let y = raw_event.offset_y() as f64 / dom_height as f64 * height as f64;
                raw_onmouseup.emit(CanvasMouseEvent {
                    canvas_handler: CanvasHandler { link: link.clone() },
                    x,
                    y,
                    width,
                    height,
                })
            })
        };

        html! {
            <canvas id={ctx.props().id.clone()}
                style={"width: 100%"}
                width={ctx.props().width.to_string()}
                height={ctx.props().height.to_string()}
                ref={self.canvas_node.clone()}
                onmousedown={onmousedown}
                onmouseup={onmouseup} />
        }
    }
}

impl Canvas {
    fn on_init(&mut self, ctx: &Context<Self>) {
        let canvas: HtmlCanvasElement = self
            .canvas_node
            .cast()
            .expect("canvas_node should used as <canvas>");
        let width = canvas.width() as usize;
        let height = canvas.height() as usize;
        let canvas_context = canvas
            .get_context(ctx.props().contexttype.as_js_param())
            .expect("should succes when get_context")
            .expect("should not null when get_context");

        self.last_refresh_time = Instant::now();

        ctx.props().oninit.emit(CanvasContext {
            canvas_handler: CanvasHandler {
                link: ctx.link().clone(),
            },
            context: canvas_context,
            width,
            height,
            delay: 0.0,
        });
        if ctx.props().autorefresh {
            ctx.link().send_message(CanvasMsg::Render);
        }
    }

    fn on_render(&mut self, ctx: &Context<Self>) {
        let canvas: HtmlCanvasElement = self
            .canvas_node
            .cast()
            .expect("canvas_node should used as <canvas>");
        let width = canvas.width() as usize;
        let height = canvas.height() as usize;
        let canvas_context = canvas
            .get_context(ctx.props().contexttype.as_js_param())
            .expect("should succes when get_context")
            .expect("should not null when get_context");

        let current_time = Instant::now();
        let delay = current_time
            .duration_since(self.last_refresh_time)
            .as_secs_f64();
        self.last_refresh_time = current_time;

        ctx.props().onrender.emit(CanvasContext {
            canvas_handler: CanvasHandler {
                link: ctx.link().clone(),
            },
            context: canvas_context,
            width,
            height,
            delay,
        });
        if ctx.props().autorefresh {
            window()
                .expect("window should not be null")
                .request_animation_frame(self.cb.as_ref().unchecked_ref())
                .expect("auto refresh canvas fail");
        }
    }
}

impl CanvasContextType {
    fn as_js_param(&self) -> &'static str {
        match self {
            CanvasContextType::Type2D => "2d",
            // CanvasContextType::TypeWebGL => "webgl",
            // CanvasContextType::TypeWebGL2 => "webgl2",
        }
    }
}

impl CanvasHandler {
    pub fn refresh(&self) {
        self.link.send_message(CanvasMsg::Render)
    }
}
