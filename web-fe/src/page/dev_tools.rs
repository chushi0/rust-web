use crate::component::{NavBar, Title};
use std::collections::{HashMap, HashSet};
use wasm_bindgen::prelude::*;
use web_sys::HtmlTextAreaElement;
use yew::prelude::*;

pub struct DevToolsPage {
    operation_items: Vec<OperationItem>,
    dataset_node: Vec<NodeRef>,
    timeout_handle: Option<i32>,

    timeout_func: Closure<dyn FnMut()>,
}

pub enum DevToolsPageMsg {
    Input,
    Calc,
}

impl Component for DevToolsPage {
    type Message = DevToolsPageMsg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let operation_items = vec![
            OperationItem::create(Distinct {}),
            OperationItem::create(Extraction {}),
            OperationItem::create(Conjunction {}),
            OperationItem::create(Minus {}),
            OperationItem::create(DistinctExtraction {}),
            OperationItem::create(DistinctConjunction {}),
            OperationItem::create(DistinctMinus {}),
            OperationItem::create(Sort {}),
        ];
        let dataset_node = vec![NodeRef::default(), NodeRef::default()];

        let timeout_func = {
            let link = ctx.link().clone();
            Closure::wrap(
                Box::new(move || link.send_message(DevToolsPageMsg::Calc)) as Box<dyn FnMut()>
            )
        };

        Self {
            operation_items,
            dataset_node,
            timeout_handle: None,
            timeout_func,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            DevToolsPageMsg::Input => {
                let mut refresh = false;
                for item in &mut self.operation_items {
                    if item.cache_result.is_some() {
                        refresh = true;
                        item.cache_result = None;
                    }
                }

                if let Some(handle) = self.timeout_handle {
                    web_sys::window()
                        .expect("window not found")
                        .clear_timeout_with_handle(handle)
                }

                self.timeout_handle = Some(
                    web_sys::window()
                        .expect("window not found")
                        .set_timeout_with_callback_and_timeout_and_arguments_0(
                            self.timeout_func.as_ref().unchecked_ref(),
                            1000,
                        )
                        .expect("set timeout"),
                );

                refresh
            }
            DevToolsPageMsg::Calc => {
                let parse_dataset = |node: &NodeRef| -> Vec<String> {
                    let node: HtmlTextAreaElement = node.cast().expect("text_area_node");
                    let content = node.value();
                    content
                        .trim()
                        .split('\n')
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string())
                        .collect()
                };

                let dataset_a = parse_dataset(&self.dataset_node[0]);
                let dataset_b = parse_dataset(&self.dataset_node[1]);

                for item in &mut self.operation_items {
                    let res = item.op.calc(&dataset_a, &dataset_b);
                    item.cache_result = Some((res.len(), res.join("\n")))
                }

                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_input = {
            let link = ctx.link().clone();
            Callback::from(move |_| link.send_message(DevToolsPageMsg::Input))
        };

        html! {
            <>
                <Title title="Dev-Tools"/>
                <NavBar active="dev-tools" />

                <div class="container-sm">
                    <table style="width: 100%;">
                        <tr>
                            <td>
                                <h3>{"操作数据集A"}</h3>
                            </td>
                            <td>
                                <h3>{"操作数据据B"}</h3>
                            </td>
                        </tr>
                        <tr>
                            <td style="vertical-align: top;">
                                <textarea placeholder="每行一条数据" style="width: 100%;" ref={self.dataset_node[0].clone()} oninput={on_input.clone()} />
                            </td>
                            <td style="vertical-align: top;">
                                <textarea placeholder="每行一条数据" style="width: 100%;" ref={self.dataset_node[1].clone()} oninput={on_input.clone()} />
                            </td>
                        </tr>
                    </table>

                    <hr/>

                    <table class="table">
                        <thead>
                            <tr>
                                <td>{"运算"}</td>
                                <td>{"结果数"}</td>
                                <td>{"结果"}</td>
                            </tr>
                        </thead>
                        <tbody>
                            {self.operation_items.iter().map(|item|html!{
                                <tr>
                                    <td>{item.op.name()}</td>
                                    {
                                        match &item.cache_result {
                                            Some((c, res)) => html!{
                                                <>
                                                    <td>{c}</td>
                                                    <td>
                                                        <pre style="max-height: 120px; padding: 8px;"><code>{res}</code></pre>
                                                    </td>
                                                </>
                                            },
                                            None => html!{
                                                <>
                                                    <td>{"???"}</td>
                                                    <td>
                                                        <pre style="max-height: 120px; padding: 8px;"><code></code></pre>
                                                    </td>
                                                </>
                                            }
                                        }
                                    }
                                </tr>
                            }).collect::<Html>()}
                        </tbody>
                    </table>
                </div>
            </>
        }
    }
}

struct OperationItem {
    op: Box<dyn Operation>,
    cache_result: Option<(usize, String)>,
}

impl OperationItem {
    fn create<T: Operation + 'static>(op: T) -> Self {
        Self {
            op: Box::new(op),
            cache_result: None,
        }
    }
}

trait Operation {
    fn calc(&self, a: &[String], b: &[String]) -> Vec<String>;
    fn name(&self) -> &'static str;
}

struct Distinct;
struct Extraction;
struct Conjunction;
struct Minus;
struct DistinctExtraction;
struct DistinctConjunction;
struct DistinctMinus;
struct Sort;

impl Operation for Distinct {
    fn calc(&self, a: &[String], _b: &[String]) -> Vec<String> {
        let mut set = HashSet::new();
        let mut res = Vec::new();

        for item in a {
            if set.contains(&item) {
                continue;
            }
            res.push(item.clone());
            set.insert(item);
        }

        res
    }

    fn name(&self) -> &'static str {
        "DISTINCT A"
    }
}

impl Operation for Extraction {
    fn calc(&self, a: &[String], b: &[String]) -> Vec<String> {
        let mut res = Vec::new();
        let mut count = HashMap::new();

        for item in a {
            res.push(item.clone());
            let c = *count.get(item).unwrap_or(&0);
            count.insert(item.clone(), c + 1);
        }

        for item in b {
            let c = *count.get(item).unwrap_or(&0);
            if c > 0 {
                count.insert(item.clone(), c - 1);
                continue;
            }
            res.push(item.clone());
        }

        res
    }

    fn name(&self) -> &'static str {
        "A ∨ B"
    }
}

impl Operation for Conjunction {
    fn calc(&self, a: &[String], b: &[String]) -> Vec<String> {
        let mut res = Vec::new();
        let mut count = HashMap::new();

        for item in a {
            let c = *count.get(item).unwrap_or(&0);
            count.insert(item.clone(), c + 1);
        }

        for item in b {
            let c = *count.get(item).unwrap_or(&0);
            if c > 0 {
                res.push(item.clone());
                count.insert(item.clone(), c - 1);
            }
        }

        res
    }

    fn name(&self) -> &'static str {
        "A ∧ B"
    }
}

impl Operation for Minus {
    fn calc(&self, a: &[String], b: &[String]) -> Vec<String> {
        let mut res = Vec::new();
        let mut count = HashMap::new();

        for item in a {
            let c = *count.get(item).unwrap_or(&0);
            count.insert(item.clone(), c + 1);
        }

        for item in b {
            let c = *count.get(item).unwrap_or(&0);
            if c > 0 {
                count.insert(item.clone(), c - 1);
            }
        }

        for (item, c) in count {
            for _ in 0..c {
                res.push(item.clone());
            }
        }

        res
    }

    fn name(&self) -> &'static str {
        "A - B"
    }
}

impl Operation for DistinctExtraction {
    fn calc(&self, a: &[String], b: &[String]) -> Vec<String> {
        Distinct {}.calc(&(Extraction {}.calc(a, b)), &Vec::new())
    }

    fn name(&self) -> &'static str {
        "DISTINCT(A ∨ B)"
    }
}

impl Operation for DistinctConjunction {
    fn calc(&self, a: &[String], b: &[String]) -> Vec<String> {
        Distinct {}.calc(&(Conjunction {}.calc(a, b)), &Vec::new())
    }

    fn name(&self) -> &'static str {
        "DISTINCT(A ∧ B)"
    }
}

impl Operation for DistinctMinus {
    fn calc(&self, a: &[String], b: &[String]) -> Vec<String> {
        Distinct {}.calc(&(Minus {}.calc(a, b)), &Vec::new())
    }

    fn name(&self) -> &'static str {
        "DISTINCT(A - B)"
    }
}

impl Operation for Sort {
    fn calc(&self, a: &[String], _b: &[String]) -> Vec<String> {
        let mut res = a.to_vec();
        res.sort();
        res
    }

    fn name(&self) -> &'static str {
        "Sort"
    }
}
