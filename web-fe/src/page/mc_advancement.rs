use crate::{
    component::*,
    model::{
        mc::{AdvancementConfig, AdvancementData},
        Model,
    },
};
use gloo_net::{http::Request, Error};
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    rc::{Rc, Weak},
    vec,
};
use web_sys::HtmlInputElement;
use yew::{html::Scope, prelude::*};

#[derive(Debug, Default)]
pub struct AdvancementConfigTree {
    all_nodes: Vec<Rc<AdvancementConfigTreeNode>>,
    roots: Vec<Weak<AdvancementConfigTreeNode>>,
}

#[derive(Debug)]
pub struct AdvancementConfigTreeNode {
    advancement: AdvancementConfig,
    children: RefCell<Vec<Weak<AdvancementConfigTreeNode>>>,
}

impl PartialEq for AdvancementConfigTreeNode {
    fn eq(&self, other: &Self) -> bool {
        self.advancement == other.advancement
    }
}

pub struct McAdvancementPage {
    global_data: AdvancementConfigTree,
    player_data: Rc<HashMap<String, AdvancementData>>,

    input_ref: NodeRef,
}

pub enum McAdvancementPageMsg {
    SetGlobalData(AdvancementConfigTree),
    SetPlayerData(HashMap<String, AdvancementData>),
    FetchDataFail,
}

impl Component for McAdvancementPage {
    type Message = McAdvancementPageMsg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        fetch_global_data(ctx.link().clone());

        McAdvancementPage {
            global_data: AdvancementConfigTree::default(),
            player_data: Rc::new(HashMap::new()),
            input_ref: NodeRef::default(),
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            McAdvancementPageMsg::SetGlobalData(global_data) => self.global_data = global_data,
            McAdvancementPageMsg::SetPlayerData(player_data) => {
                self.player_data = Rc::new(player_data)
            }
            McAdvancementPageMsg::FetchDataFail => {}
        }

        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_search_click = {
            let link = ctx.link().clone();
            let input_ref = self.input_ref.clone();
            Callback::from(move |_| {
                let input = input_ref
                    .cast::<HtmlInputElement>()
                    .expect("input_ref should attached to input element");

                fetch_player_data(link.clone(), input.value())
            })
        };

        let advancements = self.global_data.roots.iter().map(|root| {
            html! {
                <AdvancementComponent root={Weak::upgrade(root)} player_data={self.player_data.clone()}/>
            }
        });

        html! {
            <>
                <Title title="MC进度查看" />
                <NavBar active="mc-advancement"/>
                <div class="container-sm" style="padding-top: 1em; padding-bottom: 1em;">
                    <div class="input-group mb-3">
                        <span class="input-group-text">{"查询玩家成就:"}</span>
                        <input type="text" class="form-control" ref={self.input_ref.clone()} />
                        <button class="btn btn-outline-secondary" type="button" onclick={on_search_click}>{"查询"}</button>
                    </div>

                    { for advancements }
                </div>
            </>
        }
    }
}

#[derive(PartialEq, Properties)]
pub struct AdvancementComponentProps {
    root: Option<Rc<AdvancementConfigTreeNode>>,
    player_data: Rc<HashMap<String, AdvancementData>>,
}

#[function_component]
pub fn AdvancementComponent(props: &AdvancementComponentProps) -> Html {
    let showing_node = use_state(|| None);

    match &props.root {
        Some(node) => {
            let mut node_map = HashMap::new();
            node.get_node_map(&mut node_map);
            node_map.insert(node.advancement.id.clone(), node.clone());

            // 计算树的宽高
            let width = node.get_width();
            let height = node.get_height();

            // 计算svg的宽高
            let svg_width = height * 64 + 100;
            let svg_height = width * 64 + 100;

            // 计算每个node的位置
            let mut node_pos = BTreeMap::new();
            calc_node_position(node.clone(), &mut node_pos, 0, 0);

            let node_on_click = {
                let showing_node = showing_node.clone();
                Callback::from(move |node| showing_node.set(Some(node)))
            };

            // 生成每个node
            let mut node_html = vec![];
            let mut svg_node_html = vec![];
            for (node_id, (x, y)) in &node_pos {
                let node = node_map.get(node_id).expect("node should exist");
                node_html.push(html! {
                    <AdvancementNodeComponent
                        root={node.clone()} x={x} y={y} onclick={node_on_click.clone()} player_data={props.player_data.clone()} />
                });

                let start_pos = (x * 64 + 16 + 32, y * 64 + 8 + 32);
                for child in &*node.children.borrow() {
                    let child_pos = node_pos
                        .get(
                            &child
                                .upgrade()
                                .expect("child should has strong ref")
                                .advancement
                                .id,
                        )
                        .expect("child pos should exist");
                    let child_pos = (child_pos.0 * 64 + 32, child_pos.1 * 64 + 8 + 32);

                    let mid_x = (start_pos.0 + child_pos.0) / 2;

                    svg_node_html.push(html! {
                        <>
                            <line x1={start_pos.0.to_string()} y1={start_pos.1.to_string()}
                                x2={mid_x.to_string()} y2={start_pos.1.to_string()}
                                style="stroke: rgb(255,255,255); stroke-width: 2;" />
                            <line x1={mid_x.to_string()} y1={start_pos.1.to_string()}
                                x2={mid_x.to_string()} y2={child_pos.1.to_string()}
                                style="stroke: rgb(255,255,255); stroke-width: 2;" />
                            <line x1={mid_x.to_string()} y1={child_pos.1.to_string()}
                                x2={child_pos.0.to_string()} y2={child_pos.1.to_string()}
                                style="stroke: rgb(255,255,255); stroke-width: 2;" />
                        </>
                    })
                }
            }

            let close_dialog = {
                let showing_node = showing_node.clone();
                Callback::from(move |_| showing_node.set(None))
            };

            // 成就进度
            let mut process = vec![];
            if let Some(node) = (*showing_node).clone() {
                for requirement in &node.advancement.requirements {
                    if requirement.len() == 1 {
                        let color = if has_finish(&props.player_data, &node, &requirement[0]) {
                            "green"
                        } else {
                            "black"
                        };

                        process.push(html!{
                        <li><span style={format!("color: {color};")}>{requirement[0].clone()}</span></li>
                    });
                    } else {
                        let mut list = vec![];

                        for req in requirement {
                            let color = if has_finish(&props.player_data, &node, req) {
                                "green"
                            } else {
                                "black"
                            };

                            list.push(html! {
                            <li><span style={format!("color: {color};")}>{req.clone()}</span></li>
                        });
                        }

                        process.push(html! {
                            <li>
                                <i>{"完成下面任意一项"}</i>
                                <ul>{for list}</ul>
                            </li>
                        });
                    }
                }
            }

            html! {
                <div class="container" style="margin-top: 16px;">
                    <h3>{node.advancement.title.clone()}</h3>

                    <div style={format!("overflow: auto; max-height: 500px; position: relative; height: {}px; background-color: gray; margin-left: auto; margin-right: auto;", svg_height)}>
                        <svg width={svg_width.to_string()} height={svg_height.to_string()} style="position: absolute; left: 0px; top: 0px;">
                            {for svg_node_html}
                        </svg>
                        { for node_html }
                    </div>

                    if let Some(showing_node) = (*showing_node).clone() {
                        <div class="modal" style="display: block;" tabindex="-1">
                            <div class="modal-dialog modal-dialog-scrollable">
                                <div class="modal-content">
                                    <div class="modal-header">
                                        <h5 class="modal-title">{showing_node.advancement.title.clone()}</h5>
                                        <button type="button" class="btn-close" aria-label="Close" onclick={close_dialog.clone()}></button>
                                    </div>
                                    <div class="modal-body">
                                        <p>{showing_node.advancement.description.clone()}</p>
                                        <p>{"成就进度："}</p>
                                        <ul>{for process}</ul>
                                    </div>
                                    <div class="modal-footer">
                                        <button type="button" class="btn btn-primary" onclick={close_dialog.clone()}>{"关闭"}</button>
                                    </div>
                                </div>
                            </div>
                        </div>
                    }
                </div>
            }
        }
        None => html! { <></> },
    }
}

#[derive(PartialEq, Properties)]
pub struct AdvancementNodeComponentProps {
    root: Rc<AdvancementConfigTreeNode>,
    x: i32,
    y: i32,
    player_data: Rc<HashMap<String, AdvancementData>>,
    onclick: Callback<Rc<AdvancementConfigTreeNode>>,
}

#[function_component]
pub fn AdvancementNodeComponent(props: &AdvancementNodeComponentProps) -> Html {
    let hover = use_state(|| false);

    let on_click = {
        let node = props.root.clone();
        let callback = props.onclick.clone();
        Callback::from(move |_| callback.emit(node.clone()))
    };

    let player_data_node = props.player_data.get(&props.root.advancement.id);

    let player_gain = match player_data_node {
        Some(player_data_node) => player_data_node.done,
        None => false,
    };

    let img_url = format!(
        "/static/images/mc/Advancement-{}-{}.webp",
        if props.root.advancement.frame == "goal" {
            "oval"
        } else if props.root.advancement.frame == "challenge" {
            "fancy"
        } else {
            "plain"
        },
        if player_gain { "worn" } else { "raw" }
    );
    let icon_img_url = format!(
        "/static/images/mc/items/{}.png",
        props
            .root
            .advancement
            .icon
            .clone()
            .unwrap_or("placeholder".to_string())
            .trim_start_matches("minecraft:")
    );

    if *hover {
        let description_color = if props.root.advancement.frame == "challenge" {
            "purple"
        } else {
            "green"
        };

        let bar_color = if player_gain {
            "rgb(185,143,44)"
        } else {
            "rgb(3,106,150)"
        };

        let on_mouse_out = {
            let hover = hover.clone();
            Callback::from(move |_| hover.set(false))
        };

        html! {
            <div style={format!("position: absolute; left: {}px; top: {}px; width: 48px; height: 48px; z-index: 1", props.x*64+16, props.y*64+8)} onmouseout={on_mouse_out}>
                <div style={format!("position: absolute; left: -8px; background-color: {bar_color}; border-radius: 8px")}>
                    <span style="white-space: nowrap;" onclick={on_click}>
                        <img style="margin: 8px; vertical-align: center; z-index: 1; display: inline-block;" src={img_url} width="48" height="48" />
                        <img style="margin: 8px; vertical-align: center; z-index: 2; position: absolute; left: 8px; top: 8px;" src={icon_img_url} width="32" height="32" />
                        <span style="vertical-align: center; z-index: 1; white-space: nowrap; margin-right: 12px; display: inline-block; color: white;">{props.root.advancement.title.clone()}</span>
                    </span>
                    <br />
                    <div style="padding: 16px; background-color: rgb(33,33,33); border-radius: 8px;">
                        <span style={format!("z-index: 1; display: inline-block; color: {description_color};")}>{props.root.advancement.description.clone()}</span>
                    </div>
                </div>
            </div>
        }
    } else {
        let on_mouse_over = {
            let hover = hover.clone();
            Callback::from(move |_| hover.set(true))
        };

        html! {
            <div style={format!("position: absolute; left: {}px; top: {}px; width: 48px; height: 48px;", props.x*64+16, props.y*64+8)} onmouseover={on_mouse_over}>
                <div style="position: absolute; left: -8px;" onclick={on_click}>
                    <img style="margin: 8px; vertical-align: center; display: inline-block;" src={img_url} width="48" height="48" />
                    <img style="margin: 8px; vertical-align: center; position: absolute; left: 8px; top: 8px;" src={icon_img_url} width="32" height="32" />
                </div>
            </div>
        }
    }
}

fn fetch_global_data(link: Scope<McAdvancementPage>) {
    wasm_bindgen_futures::spawn_local(async move {
        let global_data = load_global_data().await;
        match global_data {
            Ok(global_data) => {
                link.send_message(McAdvancementPageMsg::SetGlobalData(global_data));
            }
            Err(e) => {
                log::error!("load data error: {}", e);
                link.send_message(McAdvancementPageMsg::FetchDataFail);
            }
        }
    })
}

async fn load_global_data() -> Result<AdvancementConfigTree, Error> {
    let raw_data = Request::get("/api/console/mc/globaldata/advancement")
        .send()
        .await?
        .json::<Model<Vec<AdvancementConfig>>>()
        .await?
        .data
        .ok_or(Error::GlooError("empty data".to_string()))?;
    let translate_data = translate_global_data(raw_data)?;
    Ok(translate_data)
}

fn fetch_player_data(link: Scope<McAdvancementPage>, name: String) {
    wasm_bindgen_futures::spawn_local(async move {
        let player_data = load_player_data(name).await;
        match player_data {
            Ok(player_data) => {
                link.send_message(McAdvancementPageMsg::SetPlayerData(player_data));
            }
            Err(e) => {
                log::error!("load data error: {}", e);
                link.send_message(McAdvancementPageMsg::FetchDataFail);
            }
        }
    })
}

async fn load_player_data(name: String) -> Result<HashMap<String, AdvancementData>, Error> {
    let raw_data = Request::new("/api/console/mc/playerdata/advancement")
        .query(vec![("name", name)])
        .method(gloo_net::http::Method::GET)
        .send()
        .await?
        .json::<Model<Vec<AdvancementData>>>()
        .await?
        .data
        .ok_or(Error::GlooError("empty data".to_string()))?;
    let translate_data = translate_player_data(raw_data);
    Ok(translate_data)
}

fn translate_global_data(
    global_data: Vec<AdvancementConfig>,
) -> Result<AdvancementConfigTree, Error> {
    let mut tree = AdvancementConfigTree::default();
    let mut nodes = HashMap::new();

    for advancement_config in global_data {
        let node = Rc::new(AdvancementConfigTreeNode {
            advancement: advancement_config,
            children: RefCell::new(vec![]),
        });

        nodes.insert(node.advancement.id.clone(), node.clone());
        tree.all_nodes.push(node);
    }

    for node in nodes.values() {
        match &node.advancement.parent {
            Some(parent_id) => nodes
                .get(&format!("{parent_id}.json"))
                .ok_or(Error::GlooError("parent_id not found".to_string()))?
                .children
                .borrow_mut()
                .push(Rc::downgrade(node)),
            None => tree.roots.push(Rc::downgrade(node)),
        }
    }

    tree.roots.sort_by_cached_key(|node| {
        node.upgrade()
            .expect("should has strong refer")
            .advancement
            .id
            .clone()
    });

    Ok(tree)
}

fn translate_player_data(player_data: Vec<AdvancementData>) -> HashMap<String, AdvancementData> {
    let mut map = HashMap::new();

    for data in player_data {
        map.insert(format!("{}.json", data.id), data);
    }

    map
}

fn has_finish(
    player_data: &Rc<HashMap<String, AdvancementData>>,
    node: &Rc<AdvancementConfigTreeNode>,
    criteria: &str,
) -> bool {
    match player_data.get(&node.advancement.id) {
        Some(player_data) => {
            for done_criteria in &player_data.done_criteria {
                if done_criteria == criteria {
                    return true;
                }
            }
            false
        }
        None => false,
    }
}

fn calc_node_position(
    node: Rc<AdvancementConfigTreeNode>,
    node_pos: &mut BTreeMap<String, (i32, i32)>,
    height: i32,
    width: i32,
) -> i32 {
    node_pos.insert(node.advancement.id.clone(), (height, width));

    let mut child_width = width;
    for child_node in &*node.children.borrow() {
        child_width += calc_node_position(
            child_node.upgrade().expect("should has strong ref"),
            node_pos,
            height + 1,
            child_width,
        )
    }

    if child_width == width {
        1
    } else {
        child_width - width
    }
}

impl AdvancementConfigTreeNode {
    fn get_width(&self) -> i32 {
        let mut res = 0;
        self.children.borrow().iter().for_each(|children| {
            res += children
                .upgrade()
                .expect("should has strong ref")
                .get_width();
        });

        if res == 0 {
            res = 1;
        }

        res
    }

    fn get_height(&self) -> i32 {
        let mut res = 0;
        self.children.borrow().iter().for_each(|children| {
            let child_res = children
                .upgrade()
                .expect("should has strong ref")
                .get_height();
            if child_res > res {
                res = child_res;
            }
        });

        res + 1
    }

    fn get_node_map(&self, map: &mut HashMap<String, Rc<AdvancementConfigTreeNode>>) {
        for child_node in &*self.children.borrow() {
            let child_node = child_node.upgrade().expect("should has strong ref");
            map.insert(child_node.advancement.id.clone(), child_node.clone());
            child_node.get_node_map(map);
        }
    }
}
