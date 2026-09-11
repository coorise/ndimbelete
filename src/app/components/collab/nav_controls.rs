//! Collaboration toolbar shortcuts in the dashboard navbar.

use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

use crate::app::components::ui::{Button, ButtonVariant, Input, Modal};
use crate::app::hooks::use_auth;
use crate::app::i18n::use_i18n;
use crate::app::lib::api;
use crate::app::lib::types::CollabStatus;

#[component]
pub fn CollabNavControls() -> impl IntoView {
    let i18n = use_i18n();
    let auth = use_auth();
    let status = RwSignal::new(Option::<CollabStatus>::None);
    let connect_open = RwSignal::new(false);
    let login_step = RwSignal::new(false);
    let push_open = RwSignal::new(false);
    let pull_open = RwSignal::new(false);
    let uri = RwSignal::new(String::new());
    let remote_user = RwSignal::new(String::new());
    let remote_pass = RwSignal::new(String::new());
    let push_msg = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);
    let remote_alert = RwSignal::new(Option::<String>::None);

    Effect::new(move |_| {
        spawn_local(async move {
            if let Ok(s) = api::collab_status().await {
                status.set(Some(s));
            }
        });
        if let Some(w) = web_sys::window() {
            let cb = Closure::wrap(Box::new(move || {
                spawn_local(async move {
                    if let Ok(s) = api::collab_status().await {
                        status.set(Some(s));
                    }
                });
            }) as Box<dyn FnMut()>);
            let _ = w.set_interval_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                30_000,
            );
            cb.forget();
        }
    });

    Effect::new(move |_| {
        let Some(window) = web_sys::window() else {
            return;
        };
        let Ok(tauri) = js_sys::Reflect::get(&window, &JsValue::from_str("__TAURI__")) else {
            return;
        };
        if tauri.is_undefined() {
            return;
        }
        let Ok(event_ns) = js_sys::Reflect::get(&tauri, &JsValue::from_str("event")) else {
            return;
        };
        let Ok(listen) = js_sys::Reflect::get(&event_ns, &JsValue::from_str("listen")) else {
            return;
        };
        let Ok(listen_fn) = listen.dyn_into::<js_sys::Function>() else {
            return;
        };
        let handler = Closure::wrap(Box::new(move |_ev: JsValue| {
            remote_alert.set(Some(
                "Nouvelles données distantes — cliquez sur Recevoir.".into(),
            ));
            spawn_local(async move {
                if let Ok(s) = api::collab_status().await {
                    status.set(Some(s));
                }
            });
        }) as Box<dyn FnMut(JsValue)>);
        let _ = listen_fn.call2(
            &event_ns,
            &JsValue::from_str("collab://remote-push"),
            handler.as_ref().unchecked_ref(),
        );
        handler.forget();
    });

    let run_connect = move |with_credentials: bool| {
        let u = uri.get_untracked().trim().to_string();
        if u.is_empty() {
            error.set(Some("URI obligatoire".into()));
            return;
        }
        let user = if with_credentials {
            Some(remote_user.get_untracked())
        } else {
            None
        };
        let pass = if with_credentials {
            Some(remote_pass.get_untracked())
        } else {
            None
        };
        if with_credentials
            && (user.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true)
                || pass.as_ref().map(|s| s.is_empty()).unwrap_or(true))
        {
            error.set(Some("Identifiant et mot de passe obligatoires".into()));
            return;
        }
        busy.set(true);
        error.set(None);
        spawn_local(async move {
            let user_ref = user.as_deref().filter(|s| !s.trim().is_empty());
            let pass_ref = pass.as_deref().filter(|s| !s.is_empty());
            match api::collab_connect(&u, user_ref, pass_ref).await {
                Ok(res) => {
                    if res.needs_remote_login {
                        login_step.set(true);
                    } else {
                        if let Some(session) = res.session {
                            auth.set_session(Some(session));
                        }
                        status.set(Some(res.status));
                        connect_open.set(false);
                        login_step.set(false);
                        remote_user.set(String::new());
                        remote_pass.set(String::new());
                    }
                }
                Err(e) => error.set(Some(e)),
            }
            busy.set(false);
        });
    };

    view! {
        <div class="flex items-center gap-1.5 sm:gap-2">
            <Show when=move || remote_alert.get().is_some()>
                <button
                    type="button"
                    class="hidden max-w-[12rem] truncate rounded-lg bg-[color-mix(in_srgb,var(--brand)_18%,transparent)] px-2 py-1 text-xs font-semibold text-[var(--brand)] md:inline"
                    title=move || remote_alert.get().unwrap_or_default()
                    on:click=move |_| remote_alert.set(None)
                >
                    {move || remote_alert.get().unwrap_or_default()}
                </button>
            </Show>

            <Show when=move || status.get().map(|s| !s.connected).unwrap_or(true)>
                <button
                    type="button"
                    class="tap-target rounded-xl border border-sky-500 bg-sky-600 px-2.5 py-2 text-sm font-semibold text-white hover:bg-sky-700"
                    title=move || i18n.t("collab.connect")
                    on:click=move |_| {
                        error.set(None);
                        login_step.set(false);
                        spawn_local(async move {
                            if let Ok(Some(u)) = api::collab_get_default_uri().await {
                                uri.set(u);
                            }
                            connect_open.set(true);
                        });
                    }
                >
                    <span class="inline-flex items-center gap-1">
                        <span aria-hidden="true">"☁"</span>
                        <span class="hidden sm:inline">{move || i18n.t("collab.connect")}</span>
                    </span>
                </button>
            </Show>

            <Show when=move || status.get().map(|s| s.connected).unwrap_or(false)>
                <Show when=move || status.get().map(|s| s.can_push).unwrap_or(true)>
                    <button
                        type="button"
                        class=move || {
                            let dirty = status.get().map(|s| s.dirty && s.local_change_count > 0).unwrap_or(false);
                            if dirty {
                                "relative tap-target inline-flex items-center gap-1 rounded-xl border border-[var(--brand)] bg-[var(--brand)] px-2.5 py-2 text-sm font-semibold text-white animate-pulse"
                            } else {
                                "relative tap-target inline-flex items-center gap-1 rounded-xl border border-[var(--brand)] bg-[var(--brand)] px-2.5 py-2 text-sm font-semibold text-white hover:brightness-110"
                            }
                        }
                        title=move || i18n.t("collab.push")
                        on:click=move |_| {
                            push_msg.set(String::new());
                            error.set(None);
                            push_open.set(true);
                        }
                    >
                        <span aria-hidden="true">"↑"</span>
                        <span class="hidden sm:inline">{move || i18n.t("collab.push")}</span>
                        <Show when=move || {
                            status.get().map(|s| s.dirty && s.local_change_count > 0).unwrap_or(false)
                        }>
                            <span class="absolute -right-1.5 -top-1.5 flex h-5 min-w-5 items-center justify-center rounded-full bg-[var(--brand-red)] px-1 text-[10px] font-bold text-white">
                                {move || status.get().map(|s| s.local_change_count).unwrap_or(0).to_string()}
                            </span>
                        </Show>
                    </button>
                </Show>
                <button
                    type="button"
                    class=move || {
                        let pending = status.get().map(|s| s.remote_ahead_count > 0).unwrap_or(false);
                        if pending {
                            "btn-collab-pull relative tap-target inline-flex items-center gap-1 rounded-xl px-2.5 py-2 text-sm font-semibold animate-pulse"
                        } else {
                            "btn-collab-pull relative tap-target inline-flex items-center gap-1 rounded-xl px-2.5 py-2 text-sm font-semibold"
                        }
                    }
                    style="background-color:#e6b800;border:1px solid #c99a00;color:#ffffff"
                    title=move || i18n.t("collab.pull")
                    on:click=move |_| {
                        error.set(None);
                        pull_open.set(true);
                    }
                >
                    <span aria-hidden="true">"↓"</span>
                    <span class="hidden sm:inline">{move || i18n.t("collab.pull")}</span>
                    <Show when=move || {
                        status.get().map(|s| s.remote_ahead_count > 0).unwrap_or(false)
                    }>
                        <span class="absolute -right-1.5 -top-1.5 flex h-5 min-w-5 items-center justify-center rounded-full bg-[var(--brand-red)] px-1 text-[10px] font-bold text-white">
                            {move || status.get().map(|s| s.remote_ahead_count).unwrap_or(0).to_string()}
                        </span>
                    </Show>
                </button>
                <button
                    type="button"
                    class="tap-target rounded-xl border border-[var(--brand-red)] bg-[var(--brand-red)] px-2.5 py-2 text-sm font-semibold text-white hover:brightness-110"
                    title=move || i18n.t("collab.disconnect")
                    on:click=move |_| {
                        spawn_local(async move {
                            let _ = api::collab_disconnect().await;
                            if let Ok(s) = api::collab_status().await {
                                status.set(Some(s));
                            }
                        });
                    }
                >
                    <span class="hidden lg:inline">{move || i18n.t("collab.disconnect")}</span>
                    <span class="lg:hidden">"✕"</span>
                </button>
            </Show>
        </div>

        <Modal
            open=connect_open.into()
            on_close=Callback::new(move |_| {
                connect_open.set(false);
                login_step.set(false);
            })
            title_signal=Signal::derive(move || {
                if login_step.get() {
                    i18n.t("collab.remote_login_title")
                } else {
                    i18n.t("collab.connect_title")
                }
            })
        >
            <div class="flex flex-col gap-3">
                <Show when=move || !login_step.get()>
                    <p class="text-sm text-[var(--muted)]">{move || i18n.t("collab.connect_help")}</p>
                    <Input
                        label_key="collab.uri"
                        value=uri.into()
                        on_input=Callback::new(move |v| uri.set(v))
                        placeholder="postgres://user:pass@host:port/db?sslmode=require"
                    />
                </Show>
                <Show when=move || login_step.get()>
                    <p class="text-sm text-[var(--muted)]">{move || i18n.t("collab.remote_login_help")}</p>
                    <Input
                        label_key="collab.remote_username"
                        value=remote_user.into()
                        on_input=Callback::new(move |v| remote_user.set(v))
                        required=true
                    />
                    <Input
                        label_key="collab.remote_password"
                        r#type="password"
                        value=remote_pass.into()
                        on_input=Callback::new(move |v| remote_pass.set(v))
                        required=true
                    />
                </Show>
                <Show when=move || error.get().is_some()>
                    <p class="text-sm text-[var(--brand-red)]" role="alert">
                        {move || error.get().unwrap_or_default()}
                    </p>
                </Show>
                <div class="flex justify-end gap-2">
                    <Button
                        variant=ButtonVariant::Secondary
                        on_click=Callback::new(move |_| {
                            connect_open.set(false);
                            login_step.set(false);
                        })
                    >
                        {move || i18n.t("common.cancel")}
                    </Button>
                    <Button on_click=Callback::new(move |_| {
                        run_connect(login_step.get_untracked());
                    })>
                        {move || {
                            if busy.get() {
                                i18n.t("common.loading")
                            } else {
                                i18n.t("collab.connect")
                            }
                        }}
                    </Button>
                </div>
            </div>
        </Modal>

        <Modal
            open=push_open.into()
            on_close=Callback::new(move |_| push_open.set(false))
            title_signal=Signal::derive(move || i18n.t("collab.push_title"))
        >
            <div class="flex flex-col gap-3">
                <p class="text-sm text-[var(--muted)]">{move || i18n.t("collab.push_help")}</p>
                <Input
                    label_key="collab.version_note"
                    value=push_msg.into()
                    on_input=Callback::new(move |v| push_msg.set(v))
                    placeholder="Ex. Cotisations juillet — optionnel"
                />
                <Show when=move || error.get().is_some()>
                    <p class="text-sm text-[var(--brand-red)]" role="alert">
                        {move || error.get().unwrap_or_default()}
                    </p>
                </Show>
                <div class="flex justify-end gap-2">
                    <Button
                        variant=ButtonVariant::Secondary
                        on_click=Callback::new(move |_| push_open.set(false))
                    >
                        {move || i18n.t("common.cancel")}
                    </Button>
                    <Button on_click=Callback::new(move |_| {
                        busy.set(true);
                        error.set(None);
                        let msg = push_msg.get_untracked();
                        spawn_local(async move {
                            let arg = if msg.trim().is_empty() {
                                None
                            } else {
                                Some(msg.as_str())
                            };
                            match api::collab_push(arg).await {
                                Ok(_) => {
                                    push_open.set(false);
                                    if let Ok(s) = api::collab_status().await {
                                        status.set(Some(s));
                                    }
                                }
                                Err(e) => error.set(Some(e)),
                            }
                            busy.set(false);
                        });
                    })>
                        {move || {
                            if busy.get() {
                                i18n.t("common.loading")
                            } else {
                                i18n.t("collab.push")
                            }
                        }}
                    </Button>
                </div>
            </div>
        </Modal>

        <Modal
            open=pull_open.into()
            on_close=Callback::new(move |_| pull_open.set(false))
            title_signal=Signal::derive(move || i18n.t("collab.pull"))
        >
            <div class="flex flex-col gap-3">
                <p class="text-sm text-[var(--muted)]">
                    "Recevoir écrasera les données locales avec la version distante. Continuer ?"
                </p>
                <Show when=move || error.get().is_some()>
                    <p class="text-sm text-[var(--brand-red)]" role="alert">
                        {move || error.get().unwrap_or_default()}
                    </p>
                </Show>
                <div class="flex justify-end gap-2">
                    <Button
                        variant=ButtonVariant::Secondary
                        on_click=Callback::new(move |_| pull_open.set(false))
                    >
                        {move || i18n.t("common.cancel")}
                    </Button>
                    <Button on_click=Callback::new(move |_| {
                        busy.set(true);
                        error.set(None);
                        spawn_local(async move {
                            match api::collab_pull().await {
                                Ok(_) => {
                                    pull_open.set(false);
                                    let _ = api::restart_app().await;
                                }
                                Err(e) => {
                                    error.set(Some(e));
                                    busy.set(false);
                                }
                            }
                        });
                    })>
                        {move || {
                            if busy.get() {
                                i18n.t("common.loading")
                            } else {
                                i18n.t("collab.pull")
                            }
                        }}
                    </Button>
                </div>
            </div>
        </Modal>
    }
}
