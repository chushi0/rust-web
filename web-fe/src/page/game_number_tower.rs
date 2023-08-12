use crate::component::*;
use js_sys::Math::random;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;
use yew::prelude::*;

pub struct GameNumberTowerPage {
    data: Rc<RefCell<GameData>>,
    gameover: bool,
}

pub enum GameNumberTowerPageMsg {
    TowerClear,
    Die,
    Restart,
}

struct GameData {
    towers: Vec<Tower>,
    player_atk: i64,
    player_tower_count: i64,
    best_atk: i64,
}

type Tower = Vec<TowerLayerInfo>;

#[derive(Debug, Clone, Copy)]
enum TowerLayerInfo {
    Clear,
    Enemy { atk: i64 },
    ItemBoost { add: i64, mul: i64 },
    ItemDamage { sub: i64, divide: i64 },
}

impl Component for GameNumberTowerPage {
    type Message = GameNumberTowerPageMsg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        let mut data = GameData {
            towers: Vec::new(),
            player_atk: 5,
            player_tower_count: 0,
            best_atk: 5,
        };
        for _ in 0..3 {
            let (next_atk, tower) = generate_next_tower(data.best_atk);
            data.best_atk = next_atk;
            data.towers.push(tower);
        }
        Self {
            data: Rc::new(RefCell::new(data)),
            gameover: false,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            GameNumberTowerPageMsg::Die => self.gameover = true,
            GameNumberTowerPageMsg::Restart => {
                self.gameover = false;
                let mut data = self.data.borrow_mut();
                data.towers.clear();
                data.player_atk = 5;
                data.player_tower_count = 0;
                data.best_atk = 5;
                for _ in 0..3 {
                    let (next_atk, tower) = generate_next_tower(data.best_atk);
                    data.best_atk = next_atk;
                    data.towers.push(tower);
                }
            }
            GameNumberTowerPageMsg::TowerClear => {}
        }
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link().clone();

        let init_callback = Callback::from(|ctx: CanvasContext| {
            let canvas: CanvasRenderingContext2d =
                ctx.context.dyn_into().expect("should be 2d context");
            let width = ctx.width as f64;
            let height = ctx.height as f64;
            canvas.set_fill_style(&JsValue::from("white"));
            canvas.fill_rect(0.0, 0.0, width, height);
            canvas.set_line_width(1.0);
            canvas.set_stroke_style(&JsValue::from("black"));
            canvas.begin_path();
            canvas.rect(0.0, 0.0, width, height);
            canvas.stroke();
        });
        let render_callback = {
            let data = self.data.clone();
            Callback::from(move |ctx: CanvasContext| {
                let data = data.borrow();

                let canvas: CanvasRenderingContext2d =
                    ctx.context.dyn_into().expect("should be 2d context");
                let width = ctx.width as f64;
                let height = ctx.height as f64;
                canvas.set_fill_style(&JsValue::from("white"));
                canvas.fill_rect(0.0, 0.0, width, height);
                canvas.set_line_width(1.0);
                canvas.set_stroke_style(&JsValue::from("black"));
                canvas.begin_path();
                canvas.rect(0.0, 0.0, width, height);
                canvas.stroke();

                let mut x = 300.0;
                let room_height = 200.0;
                let tower_space = 450.0;

                // player tower
                draw_player(&canvas, x, height, data.player_atk);
                draw_floor(&canvas, x, height);
                draw_tower_top(&canvas, x, height - room_height);

                for tower in &data.towers {
                    x += tower_space;
                    let mut y = height;
                    for layer in tower {
                        draw_floor(&canvas, x, y);
                        match layer {
                            TowerLayerInfo::Clear => {}
                            TowerLayerInfo::Enemy { atk } => draw_enemy(&canvas, x, y, *atk),
                            TowerLayerInfo::ItemBoost { add, mul } => {
                                if *mul == 1 {
                                    draw_item(&canvas, x, y, format!("+{add}"))
                                } else {
                                    draw_item(&canvas, x, y, format!("×{mul}"))
                                }
                            }
                            TowerLayerInfo::ItemDamage { sub, divide } => {
                                if *divide == 1 {
                                    draw_item(&canvas, x, y, format!("-{sub}"))
                                } else {
                                    draw_item(&canvas, x, y, format!("÷{divide}"))
                                }
                            }
                        }
                        y -= room_height;
                    }
                    draw_tower_top(&canvas, x, y);
                }

                // draw player
            })
        };
        let onmousedown = Callback::from(|_event: CanvasMouseEvent| {});

        let onmouseup = {
            let data = self.data.clone();
            let link = link.clone();

            Callback::from(move |event: CanvasMouseEvent| {
                let mut data = data.borrow_mut();
                let Some(layer) = get_click_layer(&data.towers[0], event.x, event.y) else {
                    return;
                };
                data.player_atk = data.towers[0][layer].apply(data.player_atk);
                if data.player_atk < 0 {
                    log::info!("You die");
                    link.send_message(GameNumberTowerPageMsg::Die);
                    return;
                }
                data.towers[0][layer] = TowerLayerInfo::Clear;
                let mut all_clear = true;
                for layer in &data.towers[0] {
                    if let TowerLayerInfo::Clear = layer {
                        continue;
                    }
                    all_clear = false;
                    break;
                }
                if all_clear {
                    data.player_tower_count += 1;
                    data.towers.remove(0);
                    let (next_atk, tower) = generate_next_tower(data.best_atk);
                    data.best_atk = next_atk;
                    data.towers.push(tower);
                    link.send_message(GameNumberTowerPageMsg::TowerClear);
                }
            })
        };

        let close_dialog =
            Callback::from(move |_| link.send_message(GameNumberTowerPageMsg::Restart));

        html! {
            <>
                <Title title="数字爬塔游戏"/>
                <NavBar active="game-number-tower" />
                <div class="container-sm">
                    <h3>{"数字爬塔游戏介绍"}</h3>
                    <p>{"玩家和敌人头上都有一个数字，代表战斗力。当玩家战斗力超过敌人时，可以战胜敌人，并抢夺敌人的战斗力。"}</p>
                    <p>{"当探索完前方塔上每一层后，才可以继续探索向下一坐塔。"}</p>
                    <p>{"有些楼层上会存有道具，会令玩家战斗力进行变化。"}</p>
                    <p>{"游戏目标是尽可能探索更多的塔。你能推进到第几坐塔？"}</p>
                    <p>{"（如果你使用移动设备访问本页，建议在横屏下游玩）"}</p>

                    <hr/>
                    <p>{format!("当前已探索 {} 坐塔", self.data.borrow().player_tower_count)}</p>

                    <Canvas id="game-number-tower-main-canvas"
                            width={1920} height={1080}
                            contexttype={CanvasContextType::Type2D}
                            oninit={init_callback}
                            onrender={render_callback}
                            onmousedown={onmousedown}
                            onmouseup={onmouseup} />


                    if self.gameover {
                        <div class="modal" style="display: block;" tabindex="-1">
                            <div class="modal-dialog modal-dialog-scrollable">
                                <div class="modal-content">
                                    <div class="modal-header">
                                        <h5 class="modal-title">{"游戏结束"}</h5>
                                        <button type="button" class="btn-close" aria-label="Close" onclick={close_dialog.clone()}></button>
                                    </div>
                                    <div class="modal-body">
                                        <p>{format!("您探索了 {} 坐塔", self.data.borrow().player_tower_count)}</p>
                                    </div>
                                    <div class="modal-footer">
                                        <button type="button" class="btn btn-primary" onclick={close_dialog.clone()}>{"重新开始"}</button>
                                    </div>
                                </div>
                            </div>
                        </div>
                    }
                </div>
            </>
        }
    }
}

fn get_click_layer(tower: &Tower, x: f64, y: f64) -> Option<usize> {
    let x_center = 300.0 + 450.0;
    if x < x_center - 150.0 || x > x_center + 150.0 {
        return None;
    }

    let height = ((1080.0 - y) / 200.0) as usize;
    if height > tower.len() - 1 {
        return None;
    }

    Some(height)
}

fn draw_actor(canvas: &CanvasRenderingContext2d, x: f64, y: f64, atk: i64, atk_color: &str) {
    let y = y - 80.0;
    canvas.set_line_width(3.0);
    canvas.set_stroke_style(&JsValue::from("black"));
    canvas.begin_path();
    canvas
        .arc(x, y, 15.0, 0.0, f64::to_radians(360.0))
        .expect("canvas.arc");
    canvas.move_to(x, y + 15.0);
    canvas.line_to(x, y + 50.0);
    canvas.move_to(x, y + 50.0);
    canvas.line_to(x - 10.0, y + 70.0);
    canvas.move_to(x, y + 50.0);
    canvas.line_to(x + 10.0, y + 70.0);
    canvas.move_to(x - 20.0, y + 35.0);
    canvas.line_to(x, y + 30.0);
    canvas.move_to(x + 20.0, y + 35.0);
    canvas.line_to(x, y + 30.0);
    canvas.stroke();

    canvas.set_font("2.5em Arial");
    canvas.set_line_width(1.0);
    canvas.set_fill_style(&JsValue::from(atk_color));
    let atk_text = atk.to_string();
    let measure = canvas.measure_text(&atk_text).expect("measure_text");
    canvas
        .fill_text(
            &atk_text,
            x - measure.width() / 2.0,
            y - measure.font_bounding_box_descent() - measure.font_bounding_box_ascent(),
        )
        .expect("fill_text");
}

fn draw_item(canvas: &CanvasRenderingContext2d, x: f64, y: f64, item_str: String) {
    let y = y - 35.0;
    canvas.set_font("4em Arial");
    canvas.set_line_width(1.0);
    canvas.set_fill_style(&JsValue::from("blue"));
    let measure = canvas.measure_text(&item_str).expect("measure_text");
    canvas
        .fill_text(
            &item_str,
            x - measure.width() / 2.0,
            y - measure.font_bounding_box_descent() - measure.font_bounding_box_ascent(),
        )
        .expect("fill_text");
}

#[inline]
fn draw_player(canvas: &CanvasRenderingContext2d, x: f64, y: f64, atk: i64) {
    draw_actor(canvas, x, y, atk, "green")
}

#[inline]
fn draw_enemy(canvas: &CanvasRenderingContext2d, x: f64, y: f64, atk: i64) {
    draw_actor(canvas, x, y, atk, "red")
}

fn draw_floor(canvas: &CanvasRenderingContext2d, x: f64, y: f64) {
    canvas.set_line_width(3.0);
    canvas.set_stroke_style(&JsValue::from("black"));
    canvas.begin_path();
    canvas.rect(x - 350.0 / 2.0, y - 200.0, 350.0, 200.0);
    canvas.stroke();
}

fn draw_tower_top(canvas: &CanvasRenderingContext2d, x: f64, y: f64) {
    canvas.set_line_width(3.0);
    canvas.set_stroke_style(&JsValue::from("black"));
    canvas.begin_path();
    canvas.move_to(x - 350.0 / 2.0, y);
    canvas.line_to(x, y - 170.0);
    canvas.line_to(x + 350.0 / 2.0, y);
    canvas.stroke();
}

fn generate_next_tower(start_atk: i64) -> (i64, Tower) {
    log::info!("gen tower: start={start_atk}");
    let mut tower = Vec::new();

    // 随机高度，范围 2 ~ 5
    let height = (random() * 4.0) as usize + 2;

    let mut atk = start_atk;
    while tower.len() < height {
        // 随机生成种类
        let type_rand = random();
        if type_rand < 0.1 {
            tower.push(TowerLayerInfo::ItemBoost {
                add: ((atk as f64 * random()) as i64).max(1),
                mul: 1,
            })
        } else if type_rand < 0.15 || (type_rand < 0.25 && atk < 100) {
            tower.push(TowerLayerInfo::ItemBoost { add: 0, mul: 2 })
        } else if type_rand < 0.4 && atk > 100 {
            tower.push(TowerLayerInfo::ItemDamage {
                sub: 0,
                divide: (((atk as f64).log10() * random()) as i64).max(2),
            })
        } else if type_rand < 0.5 && atk > 100 {
            tower.push(TowerLayerInfo::ItemDamage {
                sub: ((atk as f64 * random() * 0.8) as i64).max(1),
                divide: 1,
            })
        } else {
            let enemy_atk = (((random() / 5.0 + 0.8) * atk as f64) as i64).max(1);
            tower.push(TowerLayerInfo::Enemy { atk: enemy_atk });
        }

        // 重新计算最佳路线
        atk = calc_best_atk(start_atk, &tower, &mut vec![true; tower.len()]);
        log::info!("tower: {tower:?}, best_atk: {atk:?}");
    }

    // 随机化
    for i in 0..tower.len() {
        let i = tower.len() - i - 1;
        let j = (random() * i as f64) as usize;
        (tower[i], tower[j]) = (tower[j], tower[i]);
    }

    (atk, tower)
}

fn calc_best_atk(start_atk: i64, tower: &Tower, accessible: &mut Vec<bool>) -> i64 {
    if start_atk <= 0 {
        return start_atk;
    }

    let mut best_atk = 0;
    let mut has_tower = false;

    for i in 0..tower.len() {
        if !accessible[i] {
            continue;
        }
        has_tower = true;
        let atk = tower[i].apply(start_atk);
        accessible[i] = false;
        let atk = calc_best_atk(atk, tower, accessible);
        accessible[i] = true;
        if atk > best_atk {
            best_atk = atk;
        }
    }

    if !has_tower {
        return start_atk;
    }
    best_atk
}

impl TowerLayerInfo {
    fn apply(&self, start_atk: i64) -> i64 {
        match self {
            TowerLayerInfo::Clear => start_atk,
            TowerLayerInfo::Enemy { atk } => {
                if start_atk > *atk {
                    start_atk + *atk
                } else {
                    start_atk - *atk
                }
            }
            TowerLayerInfo::ItemBoost { add, mul } => start_atk * *mul + *add,
            TowerLayerInfo::ItemDamage { sub, divide } => start_atk / *divide - sub,
        }
    }
}
