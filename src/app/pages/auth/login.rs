use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;

use crate::app::components::layout::PublicNavbar;
use crate::app::components::ui::{Button, ButtonVariant, Input};
use crate::app::hooks::use_auth;
use crate::app::i18n::use_i18n;
use crate::app::lib::{api, ResetPasswordWithHintsInput};

#[component]
pub fn LoginPage() -> impl IntoView {
    let auth = use_auth();
    let i18n = use_i18n();
    let navigate = use_navigate();
    let username = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let error = RwSignal::new(Option::<String>::None);
    let busy = RwSignal::new(false);
    let checking = RwSignal::new(true);

    // forgot password flow: 0 = login, 1 = enter username, 2 = answers + new password
    let forgot_step = RwSignal::new(0u8);
    let school_answer = RwSignal::new(String::new());
    let color_answer = RwSignal::new(String::new());
    let new_password = RwSignal::new(String::new());
    let new_password_confirm = RwSignal::new(String::new());
    let recover_success = RwSignal::new(false);

    Effect::new({
        let navigate = navigate.clone();
        move |_| {
            if auth.session.get().is_some() {
                navigate("/app", Default::default());
                return;
            }
            let navigate = navigate.clone();
            spawn_local(async move {
                match api::needs_setup().await {
                    Ok(true) => navigate("/setup", Default::default()),
                    Ok(false) => checking.set(false),
                    Err(e) => {
                        error.set(Some(e));
                        checking.set(false);
                    }
                }
            });
        }
    });

    view! {
        <div class="min-h-screen">
            <PublicNavbar />
            <div class="mx-auto flex max-w-md flex-col justify-center px-5 py-12">
                <p
                    class="text-[var(--muted)]"
                    class:hidden=move || !checking.get()
                >
                    {move || i18n.t("login.checking")}
                </p>

                <div class:hidden=move || checking.get()>
                    <div class:hidden=move || forgot_step.get() != 0>
                        <h1 class="font-display text-3xl font-semibold">{move || i18n.t("login.title")}</h1>
                        <p class="mt-2 text-[var(--muted)]">
                            {move || i18n.t("login.subtitle")}
                        </p>

                        <form
                            class="mt-8 flex flex-col gap-4"
                            on:submit={
                                let navigate = navigate.clone();
                                move |ev: SubmitEvent| {
                                    ev.prevent_default();
                                    error.set(None);
                                    busy.set(true);
                                    let u = username.get_untracked();
                                    let p = password.get_untracked();
                                    let navigate = navigate.clone();
                                    spawn_local(async move {
                                        match api::login(u, p).await {
                                            Ok(session) => {
                                                auth.set_session(Some(session));
                                                navigate("/app", Default::default());
                                            }
                                            Err(e) => error.set(Some(e)),
                                        }
                                        busy.set(false);
                                    });
                                }
                            }
                        >
                            <Input
                                label_key="login.username"
                                value=username.into()
                                on_input=Callback::new(move |v| username.set(v))
                                required=true
                            />
                            <Input
                                label_key="login.password"
                                r#type="password"
                                value=password.into()
                                on_input=Callback::new(move |v| password.set(v))
                                required=true
                            />
                            <Show when=move || error.get().is_some()>
                                <p class="rounded-xl bg-[color-mix(in_srgb,var(--brand-red)_12%,transparent)] px-3 py-2 text-[var(--brand-red)]" role="alert">
                                    {move || error.get().unwrap_or_default()}
                                </p>
                            </Show>
                            <Button r#type="submit">
                                {move || {
                                    if busy.get() {
                                        i18n.t("login.busy")
                                    } else {
                                        i18n.t("login.submit")
                                    }
                                }}
                            </Button>
                            <button
                                type="button"
                                class="text-left text-sm font-semibold text-[var(--brand)] underline"
                                on:click=move |_| {
                                    error.set(None);
                                    recover_success.set(false);
                                    forgot_step.set(1);
                                }
                            >
                                {move || i18n.t("login.forgot")}
                            </button>
                        </form>
                    </div>

                    <div class:hidden=move || forgot_step.get() != 1>
                        <h1 class="font-display text-3xl font-semibold">{move || i18n.t("login.forgot_title")}</h1>
                        <p class="mt-2 text-[var(--muted)]">{move || i18n.t("login.forgot_step1")}</p>
                        <form
                            class="mt-8 flex flex-col gap-4"
                            on:submit=move |ev: SubmitEvent| {
                                ev.prevent_default();
                                error.set(None);
                                let u = username.get_untracked().trim().to_string();
                                if u.is_empty() {
                                    error.set(Some(i18n.t_static("login.username_required")));
                                    return;
                                }
                                busy.set(true);
                                spawn_local(async move {
                                    match api::get_recovery_status(u).await {
                                        Ok(status) if status.has_hints => {
                                            forgot_step.set(2);
                                        }
                                        Ok(_) => {
                                            error.set(Some(i18n.t_static("login.no_hints")));
                                        }
                                        Err(e) => error.set(Some(e)),
                                    }
                                    busy.set(false);
                                });
                            }
                        >
                            <Input
                                label_key="login.username"
                                value=username.into()
                                on_input=Callback::new(move |v| username.set(v))
                                required=true
                            />
                            <Show when=move || error.get().is_some()>
                                <p class="rounded-xl bg-[color-mix(in_srgb,var(--brand-red)_12%,transparent)] px-3 py-2 text-[var(--brand-red)]" role="alert">
                                    {move || error.get().unwrap_or_default()}
                                </p>
                            </Show>
                            <Button r#type="submit">
                                {move || {
                                    if busy.get() {
                                        i18n.t("common.loading")
                                    } else {
                                        i18n.t("login.forgot_continue")
                                    }
                                }}
                            </Button>
                            <Button
                                variant=ButtonVariant::Ghost
                                r#type="button"
                                on_click=Callback::new(move |_| {
                                    error.set(None);
                                    forgot_step.set(0);
                                })
                            >
                                {move || i18n.t("common.cancel")}
                            </Button>
                        </form>
                    </div>

                    <div class:hidden=move || forgot_step.get() != 2>
                        <h1 class="font-display text-3xl font-semibold">{move || i18n.t("login.forgot_title")}</h1>
                        <p class="mt-2 text-[var(--muted)]">{move || i18n.t("login.forgot_step2")}</p>
                        <Show when=move || recover_success.get()>
                            <p class="mt-4 rounded-xl bg-[color-mix(in_srgb,var(--brand)_12%,transparent)] px-3 py-2 text-[var(--brand)]">
                                {move || i18n.t("login.recover_success")}
                            </p>
                        </Show>
                        <form
                            class="mt-8 flex flex-col gap-4"
                            class:hidden=move || recover_success.get()
                            on:submit=move |ev: SubmitEvent| {
                                ev.prevent_default();
                                error.set(None);
                                let pass = new_password.get_untracked();
                                let confirm = new_password_confirm.get_untracked();
                                if pass.len() < 8 {
                                    error.set(Some(i18n.t_static("login.password_short")));
                                    return;
                                }
                                if pass != confirm {
                                    error.set(Some(i18n.t_static("login.password_mismatch")));
                                    return;
                                }
                                busy.set(true);
                                let input = ResetPasswordWithHintsInput {
                                    username: username.get_untracked(),
                                    school_answer: school_answer.get_untracked(),
                                    color_answer: color_answer.get_untracked(),
                                    new_password: pass,
                                };
                                spawn_local(async move {
                                    match api::reset_password_with_hints(input).await {
                                        Ok(()) => {
                                            recover_success.set(true);
                                            password.set(String::new());
                                            school_answer.set(String::new());
                                            color_answer.set(String::new());
                                            new_password.set(String::new());
                                            new_password_confirm.set(String::new());
                                        }
                                        Err(e) => error.set(Some(e)),
                                    }
                                    busy.set(false);
                                });
                            }
                        >
                            <Input
                                label_key="login.recovery_school"
                                r#type="password"
                                value=school_answer.into()
                                on_input=Callback::new(move |v| school_answer.set(v))
                                required=true
                            />
                            <Input
                                label_key="login.recovery_color"
                                r#type="password"
                                value=color_answer.into()
                                on_input=Callback::new(move |v| color_answer.set(v))
                                required=true
                            />
                            <Input
                                label_key="login.new_password"
                                r#type="password"
                                value=new_password.into()
                                on_input=Callback::new(move |v| new_password.set(v))
                                required=true
                            />
                            <Input
                                label_key="login.new_password_confirm"
                                r#type="password"
                                value=new_password_confirm.into()
                                on_input=Callback::new(move |v| new_password_confirm.set(v))
                                required=true
                            />
                            <Show when=move || error.get().is_some()>
                                <p class="rounded-xl bg-[color-mix(in_srgb,var(--brand-red)_12%,transparent)] px-3 py-2 text-[var(--brand-red)]" role="alert">
                                    {move || error.get().unwrap_or_default()}
                                </p>
                            </Show>
                            <Button r#type="submit">
                                {move || {
                                    if busy.get() {
                                        i18n.t("common.loading")
                                    } else {
                                        i18n.t("login.reset_password")
                                    }
                                }}
                            </Button>
                            <Button
                                variant=ButtonVariant::Ghost
                                r#type="button"
                                on_click=Callback::new(move |_| {
                                    error.set(None);
                                    forgot_step.set(0);
                                })
                            >
                                {move || i18n.t("login.back_to_login")}
                            </Button>
                        </form>
                        <div class:hidden=move || !recover_success.get() class="mt-6">
                            <Button on_click=Callback::new(move |_| {
                                recover_success.set(false);
                                forgot_step.set(0);
                            })>
                                {move || i18n.t("login.back_to_login")}
                            </Button>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}
