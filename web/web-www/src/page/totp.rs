use crate::component::*;
use anyhow::{anyhow, Result};
use totp_lite::Sha1;
use web_sys::HtmlInputElement;
use yew::prelude::*;

fn totp(secret: &str) -> Result<String> {
    let secret_bytes = base32::decode(base32::Alphabet::Rfc4648 { padding: false }, secret)
        .ok_or(anyhow!("invalid secret key"))?;
    let current_time = js_sys::Date::new_0().get_time() as u64;
    let code = totp_lite::totp_custom::<Sha1>(30000, 6, &secret_bytes, current_time);
    Ok(code)
}

pub struct TotpPage {
    input_key: NodeRef,
    generated_password: Option<String>,
    alert_msg: Option<String>,
}

pub enum TotpPageMsg {
    PasswordGenerated { pwd: String },
    GenFailed { msg: String },
}

impl Component for TotpPage {
    type Message = TotpPageMsg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            input_key: NodeRef::default(),
            generated_password: None,
            alert_msg: None,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            TotpPageMsg::PasswordGenerated { pwd } => {
                self.generated_password = Some(pwd);
                self.alert_msg = None;
                true
            }
            TotpPageMsg::GenFailed { msg } => {
                self.generated_password = None;
                self.alert_msg = Some(msg);
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let totp_click = {
            let link = ctx.link().clone();
            let input = self.input_key.clone();
            Callback::from(move |_| {
                let key = input
                    .cast::<HtmlInputElement>()
                    .expect("this is input element")
                    .value();
                let pwd = totp(&key);
                match pwd {
                    Ok(pwd) => link.send_message(TotpPageMsg::PasswordGenerated { pwd }),
                    Err(err) => link.send_message(TotpPageMsg::GenFailed {
                        msg: err.to_string(),
                    }),
                }
            })
        };

        html! {
            <>
                <Title title="一次性密码 TOTP" />
                <NavBar active="totp"/>
                <div class="container-sm" style="padding-top: 1em; padding-bottom: 1em;">

                    {
                        self.alert_msg.clone().map(|msg| html!{
                            <div class="alert alert-warning" role="alert">
                                {msg}
                            </div>
                        })
                    }

                    <table>
                        <tr>
                            <td>
                                {"Secret Key:"}
                            </td>
                            <td>
                                <input type="password" ref={self.input_key.clone()} />
                            </td>
                        </tr>
                        {
                            self.generated_password.clone().map(|pwd| html! {
                                <tr>
                                    <td>
                                        {"Password:"}
                                    </td>
                                    <td>
                                        <strong>{pwd}</strong>
                                    </td>
                                </tr>
                            })
                        }
                        <tr>
                            <td></td>
                            <td>
                                <input type="button" value="TOTP!" onclick={totp_click} />
                            </td>
                        </tr>
                    </table>
                </div>
            </>
        }
    }
}
