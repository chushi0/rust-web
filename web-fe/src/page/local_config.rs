use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::{component::*, config::secret::SecretConfig};

#[derive(Debug, Default, Clone)]
struct AppConfig {
    secret: SecretConfig,
}

#[function_component]
pub fn LocalConfigPage() -> Html {
    let config = use_state(AppConfig::default);

    {
        let config = config.clone();
        use_effect_with((), move |_| {
            config.set(AppConfig {
                secret: SecretConfig::load_from_localstorage(),
            });
        });
    }

    let on_edit_auth_key = {
        let config = config.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let mut new_config = (*config).clone();
            new_config.secret.auth_key = Some(input.value());
            config.set(new_config);
        })
    };

    let on_submit = {
        let config = config.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            config.secret.save_to_localstorage();
            _ = js_sys::eval(
                r#"
                    new bootstrap.Modal(document.getElementById("save-config-modal")).show()
                "#,
            );
        })
    };

    html! {
        <>
            <Title title="设置" />
            <NavBar active="config" />

            <div class="container-sm">
                <h3>{"本地配置管理"}</h3>

                <form onsubmit={on_submit}>
                    <div class="row mb-3">
                        <label for="auth_key" class="col-sm-2 col-form-label">
                            {"身份验证密钥"}
                        </label>
                        <div class="col-sm-10">
                            <input type="password" class="form-control" id="auth_key"
                                value={config.secret.auth_key.clone().unwrap_or_default()} oninput={on_edit_auth_key} />
                            <i style="color: gray;">{"当进行敏感操作时，需要此信息确认您的管理员身份。如果您不是管理员，请忽略此信息。"}</i>
                        </div>
                    </div>

                    <button type="submit" class="btn btn-primary">
                        {"保存配置"}
                    </button>
                </form>

                <div class="modal fade" tabindex="-1" id="save-config-modal">
                    <div class="modal-dialog">
                        <div class="modal-content">
                            <div class="modal-header">
                                <h5 class="modal-title">{"保存配置"}</h5>
                                <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close"></button>
                            </div>
                            <div class="modal-body">
                                {"配置已保存"}
                            </div>
                            <div class="modal-footer">
                                <button type="button" data-bs-dismiss="modal" class="btn btn-primary">{"确定"}</button>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </>
    }
}
