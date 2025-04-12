use std::{cell::RefCell, collections::HashMap, fmt::Debug, rc::Rc, str::FromStr};

use crate::component::*;
use js_sys::Math;
use web_sys::{window, HtmlInputElement};
use yew::prelude::*;

pub struct WuwaGachaPage {
    draw_context: Rc<RefCell<DrawContext>>,
}

pub enum WuwaGachaPageMsg {
    UpdateContext,
    StartSimulate,
}

const FOUR_STAR_CHARACTER_COUNT: usize = 11;
const FOUR_STAR_CHARACTER: [&str; FOUR_STAR_CHARACTER_COUNT] = [
    "灯灯",
    "釉瑚",
    "渊武",
    "丹瑾",
    "散华",
    "莫特斐",
    "桃祁",
    "秋水",
    "白芷",
    "秧秧",
    "炽霞",
];

const FIVE_STAR_CHARACTER_COUNT: usize = 5;
const FIVE_STAR_CHARACTER: [&str; FIVE_STAR_CHARACTER_COUNT] =
    ["卡卡罗", "凌阳", "鉴心", "维里奈", "安可"];

const FOUR_STAR_WEAPON_COUNT: usize = 20;

impl Component for WuwaGachaPage {
    type Message = WuwaGachaPageMsg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            draw_context: Rc::new(RefCell::new(DrawContext::default())),
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            WuwaGachaPageMsg::UpdateContext => true,
            WuwaGachaPageMsg::StartSimulate => {
                log::info!("{:?}", self.draw_context);
                let result = self.draw_context.borrow().batch_simulate();
                log::info!("{:?}", result);

                let mut s = format!("模拟 {} 次的结果：", result.total);
                s += &format!(
                    "\n啥都没抽到：{:.02}%",
                    result
                        .result_set
                        .get(&(0, 0))
                        .map_or(0.0, |v| *v as f64 / result.total as f64 * 100.0)
                );
                for (c, w) in [
                    (1, 0),
                    (1, 1),
                    (2, 1),
                    (3, 1),
                    (4, 1),
                    (5, 1),
                    (6, 1),
                    (7, 1),
                    (7, 2),
                    (7, 3),
                    (7, 4),
                    (7, 5),
                ] {
                    s += &format!(
                        "\n{}+{}：{:.02}%",
                        c - 1,
                        w,
                        result
                            .result_set
                            .get(&(c, w))
                            .map_or(0.0, |v| *v as f64 / result.total as f64 * 100.0)
                    );
                }

                let _ = window()
                    .expect("window should be exist")
                    .alert_with_message(&s);
                return false;
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let start_simulate = {
            let link = ctx.link().clone();
            Callback::from(move |_| {
                link.send_message(WuwaGachaPageMsg::StartSimulate);
            })
        };

        html! {
            <>
                <Title title="鸣潮抽卡模拟" />
                <NavBar active="wuwa-gacha"/>

                <div class="container">
                    <h3>{"鸣潮抽卡模拟"}</h3>
                    <h5>{"基础货币"}</h5>

                    <div class="row mb-3">
                        <label for="input-star" class="col-sm-2 col-form-label">{"星声"}</label>
                        <div class="col-sm-4">
                            <input type="number" id="input-star" class="form-control" required=true
                                value={self.draw_context.borrow().money_star.to_string()}
                                oninput={self.process_input_draw_context(ctx, |ctx, val| ctx.money_star = val)} />
                        </div>
                        <label for="input-month" class="col-sm-2 col-form-label">{"月相"}</label>
                        <div class="col-sm-4">
                            <input type="number" id="input-month" class="form-control" required=true
                                value={self.draw_context.borrow().money_month.to_string()}
                                oninput={self.process_input_draw_context(ctx, |ctx, val| ctx.money_month = val)}  />
                        </div>
                    </div>

                    <div class="row mb-3">
                        <label for="input-item-character" class="col-sm-2 col-form-label">{"浮金波纹（角色球）"}</label>
                        <div class="col-sm-4">
                            <input type="number" id="input-item-character" class="form-control" required=true
                                value={self.draw_context.borrow().ball_character.to_string()}
                                oninput={self.process_input_draw_context(ctx, |ctx, val| ctx.ball_character = val)} />
                        </div>
                        <label for="input-item-weapon" class="col-sm-2 col-form-label">{"铸潮波纹（武器球）"}</label>
                        <div class="col-sm-4">
                            <input type="number" id="input-item-weapon" class="form-control" required=true
                                value={self.draw_context.borrow().ball_weapon.to_string()}
                                oninput={self.process_input_draw_context(ctx, |ctx, val| ctx.ball_weapon = val)} />
                        </div>
                    </div>

                    <div class="row mb-3">
                        <label for="input-draw-reward" class="col-sm-2 col-form-label">{"余波珊瑚"}</label>
                        <div class="col-sm-4">
                            <input type="number" id="input-draw-reward" class="form-control" required=true
                                value={self.draw_context.borrow().reward_gold.to_string()}
                                oninput={self.process_input_draw_context(ctx, |ctx, val| ctx.reward_gold = val)} />
                        </div>
                    </div>

                    <h5>{"卡池配置"}</h5>

                    <div class="row mb-3">
                        <label  class="col-sm-2 col-form-label">{"角色池已垫抽数"}</label>
                        <div class="col-sm-4">
                            <input type="number" class="form-control" required=true
                                value={self.draw_context.borrow().paid_character.to_string()}
                                oninput={self.process_input_draw_context(ctx, |ctx, val| ctx.paid_character = val)} />
                        </div>
                        <label class="col-sm-2 col-form-label">{"武器池已垫抽数"}</label>
                        <div class="col-sm-4">
                            <input type="number" class="form-control" required=true
                                value={self.draw_context.borrow().paid_weapon.to_string()}
                                oninput={self.process_input_draw_context(ctx, |ctx, val| ctx.paid_weapon = val)}  />
                        </div>
                    </div>
                    <div class="row mb-3">
                        <div class="form-check col-sm-12">
                            <input class="form-check-input" type="checkbox"
                                checked={self.draw_context.borrow().guarantee_character}
                                onchange={self.process_input_draw_context_bool(ctx, |ctx, val| ctx.guarantee_character = val)} />
                            <label class="form-check-label">{"角色池大保底"}</label>
                        </div>
                    </div>

                    <h5>{"角色命座"}</h5>

                    {
                        FIVE_STAR_CHARACTER.chunks(2).enumerate().map(|(i, s)| {
                            html! {
                                <div class="row mb-3">
                                    {
                                        s.iter().enumerate().map(move |(j, s)| {
                                            let index = i * 2 + j;
                                            html! {
                                                <>
                                                    <label class="col-sm-2 col-form-label">{s}</label>
                                                    <div class="col-sm-4">
                                                        <input type="number" class="form-control" required=true
                                                            value={self.draw_context.borrow().chain_five[index].to_string()}
                                                            oninput={self.process_input_draw_context(ctx, move |ctx, val| ctx.chain_five[index]=val)} />
                                                    </div>
                                                </>
                                            }
                                        }).collect::<Html>()
                                    }
                                </div>
                            }
                        }).collect::<Html>()
                    }

                    {
                        FOUR_STAR_CHARACTER.chunks(2).enumerate().map(|(i, s)| {
                            html! {
                                <div class="row mb-3">
                                    {
                                        s.iter().enumerate().map(move |(j, s)| {
                                            let index = i * 2 + j;
                                            html! {
                                                <>
                                                    <label class="col-sm-2 col-form-label">{s}</label>
                                                    <div class="col-sm-4">
                                                        <input type="number" class="form-control" required=true
                                                            value={self.draw_context.borrow().chain_four[index].to_string()}
                                                            oninput={self.process_input_draw_context(ctx, move |ctx, val| ctx.chain_four[index]=val)}  />
                                                    </div>
                                                </>
                                            }
                                        }).collect::<Html>()
                                    }
                                </div>
                            }
                        }).collect::<Html>()
                    }

                    <button onclick={start_simulate}>{"开始模拟"}</button>
                </div>
            </>
        }
    }
}

impl WuwaGachaPage {
    fn process_input_draw_context<T, F>(
        &self,
        ctx: &Context<Self>,
        f: F,
    ) -> Callback<InputEvent, ()>
    where
        T: FromStr,
        <T as FromStr>::Err: Debug,
        F: Fn(&mut DrawContext, T) + 'static,
    {
        let link = ctx.link().clone();
        let context = self.draw_context.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            if let Ok(val) = input.value().parse() {
                f(&mut *context.borrow_mut(), val);
                link.send_message(WuwaGachaPageMsg::UpdateContext);
            }
        })
    }

    fn process_input_draw_context_bool<F>(&self, ctx: &Context<Self>, f: F) -> Callback<Event, ()>
    where
        F: Fn(&mut DrawContext, bool) + 'static,
    {
        let link = ctx.link().clone();
        let context = self.draw_context.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            f(&mut *context.borrow_mut(), input.checked());
            link.send_message(WuwaGachaPageMsg::UpdateContext);
        })
    }
}

#[derive(Debug, Default, Clone)]
struct DrawContext {
    money_star: u32,                              // 星声
    money_month: u32,                             // 月相
    ball_character: u32,                          // 浮金波纹
    ball_weapon: u32,                             // 铸潮波纹
    reward_gold: u32,                             // 余波珊瑚
    paid_character: u32,                          // 角色池垫
    paid_weapon: u32,                             // 武器池垫
    guarantee_character: bool,                    // 角色池大保底
    chain_five: [i32; FIVE_STAR_CHARACTER_COUNT], // 五星角色命座
    chain_four: [i32; FOUR_STAR_CHARACTER_COUNT], // 四星角色命座
}

#[derive(Debug)]
struct SimulateResult {
    total: u32,
    result_set: HashMap<(u32, u32), u32>,
}

impl DrawContext {
    fn batch_simulate(&self) -> SimulateResult {
        const MAX: u32 = 10_0000;
        let mut results: HashMap<(u32, u32), u32> = HashMap::new();
        for _ in 0..MAX {
            let result = self.clone().simulate();
            *results.entry(result).or_default() += 1;
        }

        SimulateResult {
            total: MAX,
            result_set: results,
        }
    }

    fn simulate(mut self) -> (u32, u32) {
        let mut character = 0;
        let mut weapon = 0;

        // 抽取顺序：00 - 01 - 61 - 65
        while character < 7 || weapon < 5 {
            if character == 0 || (weapon == 1 && character < 7) {
                match self.draw_character() {
                    None => break,
                    Some(false) => continue,
                    Some(true) => {
                        // 首次获得15余波珊瑚
                        self.reward_gold += 15;
                        character += 1;
                        continue;
                    }
                }
            } else {
                match self.draw_weapon() {
                    None => break,
                    Some(false) => continue,
                    Some(true) => {
                        weapon += 1;
                        continue;
                    }
                }
            }
        }

        (character, weapon)
    }

    fn draw_character(&mut self) -> Option<bool> {
        if self.ball_character > 0 {
            self.ball_character -= 1;
        } else if !self.cost() {
            return None;
        }

        // 抽卡次数 +1
        self.paid_character += 1;

        // 抽卡随机数
        let random_value = Math::random();
        // 出金概率：0.8%，或80抽保底
        if random_value <= 0.008 || self.paid_character == 80 {
            // 是否出UP五星
            let draw_up;
            // 50%概率出UP五星，或当前为大保底
            if Math::random() <= 0.5 || self.guarantee_character {
                draw_up = true;
                self.guarantee_character = false;
            } else {
                draw_up = false;
                self.guarantee_character = true;
                // 歪常驻，随机常驻角色加命座
                let index = (Math::random() * FIVE_STAR_CHARACTER_COUNT as f64) as usize;
                // 满命，额外40余波珊瑚，否则，额外15余波珊瑚
                if self.chain_five[index] == 6 {
                    self.reward_gold += 40;
                } else {
                    self.reward_gold += 15;
                }
                self.chain_five[index] += 1;
                // 歪了还会额外给30
                self.reward_gold += 30;
            }
            // 清除角色抽卡记数
            self.paid_character = 0;
            return Some(draw_up);
        }
        // 出紫概率6%，或10抽保底
        if random_value <= 0.068 || self.paid_character % 10 == 9 {
            // 50% UP四星，50%其他四星，每个非UP四星概率均等
            let is_weapon = Math::random() < 0.5
                && Math::random()
                    * ((FOUR_STAR_CHARACTER_COUNT + FOUR_STAR_WEAPON_COUNT - 3) as f64)
                    < FOUR_STAR_WEAPON_COUNT as f64;
            if is_weapon {
                // 四星武器额外获得3余波珊瑚
                self.reward_gold += 3;
            } else {
                // 随机常驻角色加命座
                let index = (Math::random() * FOUR_STAR_CHARACTER_COUNT as f64) as usize;
                // 满命，额外8余波珊瑚，否则，额外3余波珊瑚
                if self.chain_four[index] == 6 {
                    self.reward_gold += 8;
                } else {
                    self.reward_gold += 3;
                }
                self.chain_four[index] += 1;
            }
        }
        Some(false)
    }

    fn draw_weapon(&mut self) -> Option<bool> {
        if self.ball_weapon > 0 {
            self.ball_weapon -= 1;
        } else if !self.cost() {
            return None;
        }

        // 抽卡次数 +1
        self.paid_weapon += 1;

        // 抽卡随机数
        let random_value = Math::random();
        // 出金概率：0.8%，或80抽保底
        if random_value <= 0.008 || self.paid_weapon == 80 {
            // 清除武器抽卡记数
            self.paid_weapon = 0;
            return Some(true);
        }
        // 出紫概率6%，或10抽保底
        if random_value <= 0.068 || self.paid_weapon % 10 == 9 {
            // 50% UP四星，50%其他四星，每个非UP四星概率均等
            let is_weapon = Math::random() < 0.5
                && Math::random()
                    * ((FOUR_STAR_CHARACTER_COUNT + FOUR_STAR_WEAPON_COUNT - 3) as f64)
                    < (FOUR_STAR_WEAPON_COUNT - 3) as f64;
            if is_weapon {
                // 四星武器额外获得3余波珊瑚
                self.reward_gold += 3;
            } else {
                // 随机常驻角色加命座
                let index = (Math::random() * FOUR_STAR_CHARACTER_COUNT as f64) as usize;
                // 满命，额外8余波珊瑚，否则，额外3余波珊瑚
                if self.chain_four[index] == 6 {
                    self.reward_gold += 8;
                } else {
                    self.reward_gold += 3;
                }
                self.chain_four[index] += 1;
            }
        }
        Some(false)
    }

    fn cost(&mut self) -> bool {
        if self.money_star >= 160 {
            self.money_star -= 160;
            return true;
        }

        if self.money_star + self.money_month >= 160 {
            self.money_month -= 160 - self.money_star;
            self.money_star = 0;
            return true;
        }

        if self.reward_gold >= 8 {
            self.reward_gold -= 8;
            return true;
        }

        false
    }
}
