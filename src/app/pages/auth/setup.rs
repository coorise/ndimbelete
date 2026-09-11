use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;

use crate::app::components::layout::PublicNavbar;
use crate::app::components::ui::{Button, ButtonVariant, Input, TabItem, Tabs};
use crate::app::hooks::use_auth;
use crate::app::i18n::use_i18n;
use crate::app::lib::{api, mask_postgres_uri};

fn force_setup_demo() -> bool {
    web_sys::window()
        .and_then(|w| w.location().search().ok())
        .map(|s| s.contains("force_setup=1"))
        .unwrap_or(false)
}

/// First-run screen: remote connect (default), restore backup, OR use locally.
#[component]
pub fn SetupPage() -> impl IntoView {
    let auth = use_auth();
    let i18n = use_i18n();
    let navigate = use_navigate();
    let navigate_guard = navigate.clone();
    let navigate_submit = navigate.clone();

    let tab = RwSignal::new("remote");
    let last_name = RwSignal::new(String::new());
    let first_name = RwSignal::new(String::new());
    let username = RwSignal::new(String::new());
    let phone = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let password_confirm = RwSignal::new(String::new());
    let recovery_school = RwSignal::new(String::new());
    let recovery_color = RwSignal::new(String::new());
    let error = RwSignal::new(Option::<String>::None);
    let busy = RwSignal::new(false);
    let checking = RwSignal::new(true);
    let backup_path = RwSignal::new(Option::<String>::None);
    let remote_uri = RwSignal::new(String::new());
    let remote_user = RwSignal::new(String::new());
    let remote_pass = RwSignal::new(String::new());

    Effect::new(move |_| {
        if auth.session.get().is_some() && !force_setup_demo() {
            navigate_guard("/app", Default::default());
            return;
        }
        let navigate_guard = navigate_guard.clone();
        spawn_local(async move {
            match api::needs_setup().await {
                Ok(true) => {
                    if let Ok(Some(u)) = api::collab_get_default_uri().await {
                        remote_uri.set(u);
                    }
                    checking.set(false);
                }
                Ok(false) if force_setup_demo() => {
                    if let Ok(Some(u)) = api::collab_get_default_uri().await {
                        remote_uri.set(u);
                    }
                    checking.set(false);
                }
                Ok(false) => navigate_guard("/login", Default::default()),
                Err(e) => {
                    error.set(Some(e));
                    checking.set(false);
                }
            }
        });
    });

    let tab_items = Signal::derive(move || {
        vec![
            TabItem {
                id: "remote",
                label: i18n.t("setup.tab_remote"),
            },
            TabItem {
                id: "backup",
                label: i18n.t("setup.tab_backup"),
            },
            TabItem {
                id: "admin",
                label: i18n.t("setup.tab_admin"),
            },
        ]
    });

    let remote_uri_display = Signal::derive(move || mask_postgres_uri(&remote_uri.get()));

    view! {
        <div class="min-h-screen">
            <PublicNavbar />
            <div class="mx-auto flex max-w-xl flex-col justify-center px-5 py-12">
                <p
                    class="text-[var(--muted)]"
                    class:hidden=move || !checking.get()
                >
                    {move || i18n.t("setup.preparing")}
                </p>

                <div class:hidden=move || checking.get()>
                    <h1 class="font-display text-3xl font-semibold">{move || i18n.t("setup.title")}</h1>
                    <p class="mt-2 text-[var(--muted)]">
                        {move || i18n.t("setup.subtitle")}
                    </p>

                    <div class="mt-6">
                        <Tabs items=tab_items active=tab />
                    </div>

                    <div
                        class="mt-8 flex flex-col gap-4"
                        class:hidden=move || tab.get() != "remote"
                    >
                        <p class="text-[var(--muted)]">{move || i18n.t("setup.remote_help")}</p>
                        <Input
                            label_key="collab.uri"
                            value=remote_uri_display.into()
                            on_input=Callback::new(move |v: String| {
                                // Keep real URI; ignore edits of the masked display.
                                if v.contains(":***") {
                                    return;
                                }
                                remote_uri.set(v);
                            })
                            placeholder="postgres://user:pass@host:port/db?sslmode=require"
                        />
                        <Input
                            label_key="collab.remote_username"
                            value=remote_user.into()
                            on_input=Callback::new(move |v| remote_user.set(v))
                            placeholder="ex. commissaire"
                        />
                        <Input
                            label_key="collab.remote_password"
                            r#type="password"
                            value=remote_pass.into()
                            on_input=Callback::new(move |v| remote_pass.set(v))
                        />
                        <Show when=move || error.get().is_some() && tab.get() == "remote">
                            <p
                                class="rounded-xl bg-[color-mix(in_srgb,var(--brand-red)_12%,transparent)] px-3 py-2 text-[var(--brand-red)]"
                                role="alert"
                            >
                                {move || error.get().unwrap_or_default()}
                            </p>
                        </Show>
                        <Button on_click=Callback::new({
                            let navigate = navigate.clone();
                            move |_| {
                                let u = remote_uri.get_untracked().trim().to_string();
                                if u.is_empty() {
                                    error.set(Some("URI obligatoire".into()));
                                    return;
                                }
                                let user = remote_user.get_untracked();
                                let pass = remote_pass.get_untracked();
                                let has_creds =
                                    !user.trim().is_empty() && !pass.is_empty();
                                busy.set(true);
                                error.set(None);
                                let navigate = navigate.clone();
                                spawn_local(async move {
                                    let user_ref =
                                        if has_creds { Some(user.as_str()) } else { None };
                                    let pass_ref =
                                        if has_creds { Some(pass.as_str()) } else { None };
                                    match api::collab_connect(&u, user_ref, pass_ref).await {
                                        Ok(res) => {
                                            if res.needs_remote_login {
                                                error.set(Some(
                                                    i18n.t_static("setup.remote_creds_required"),
                                                ));
                                                busy.set(false);
                                                return;
                                            }
                                            if let Some(session) = res.session {
                                                auth.set_session(Some(session));
                                                navigate("/app", Default::default());
                                                busy.set(false);
                                                return;
                                            }
                                            // Empty remote (or connected without session):
                                            // no root yet → create admin locally.
                                            match api::needs_setup().await {
                                                Ok(true) => {
                                                    info_or_tab_admin(
                                                        &error,
                                                        &tab,
                                                        &i18n.t_static("setup.remote_empty"),
                                                    );
                                                    busy.set(false);
                                                }
                                                Ok(false) => {
                                                    navigate("/login", Default::default());
                                                    busy.set(false);
                                                }
                                                Err(e) => {
                                                    error.set(Some(e));
                                                    busy.set(false);
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            error.set(Some(e));
                                            busy.set(false);
                                        }
                                    }
                                });
                            }
                        })>
                            {move || {
                                if busy.get() {
                                    i18n.t("setup.remote_connecting")
                                } else {
                                    i18n.t("setup.remote_connect")
                                }
                            }}
                        </Button>
                    </div>

                    <div
                        class="mt-8 flex flex-col gap-4"
                        class:hidden=move || tab.get() != "backup"
                    >
                        <p class="text-[var(--muted)]">{move || i18n.t("setup.backup_help")}</p>
                        <Button
                            variant=ButtonVariant::Secondary
                            on_click=Callback::new(move |_| {
                                spawn_local(async move {
                                    if let Some(path) = api::pick_backup_file().await
                                    {
                                        backup_path.set(Some(path));
                                        error.set(None);
                                    }
                                });
                            })
                        >
                            {move || i18n.t("setup.pick_backup")}
                        </Button>
                        <p class="text-sm text-[var(--muted)]">
                            {move || {
                                backup_path
                                    .get()
                                    .unwrap_or_else(|| i18n.t("setup.no_file"))
                            }}
                        </p>
                        <Show when=move || error.get().is_some() && tab.get() == "backup">
                            <p
                                class="rounded-xl bg-[color-mix(in_srgb,var(--brand-red)_12%,transparent)] px-3 py-2 text-[var(--brand-red)]"
                                role="alert"
                            >
                                {move || error.get().unwrap_or_default()}
                            </p>
                        </Show>
                        <Button on_click=Callback::new(move |_| {
                            let Some(path) = backup_path.get_untracked() else {
                                error.set(Some(i18n.t_static("setup.no_file")));
                                return;
                            };
                            busy.set(true);
                            error.set(None);
                            spawn_local(async move {
                                match api::import_backup(&path).await {
                                    Ok(()) => {
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
                                    i18n.t("setup.restoring")
                                } else {
                                    i18n.t("setup.restore")
                                }
                            }}
                        </Button>
                    </div>

                    <form
                        class="mt-8 flex flex-col gap-4"
                        class:hidden=move || tab.get() != "admin"
                        on:submit=move |ev: SubmitEvent| {
                            ev.prevent_default();
                            error.set(None);

                            let last = last_name.get_untracked().trim().to_string();
                            let first = first_name.get_untracked().trim().to_string();
                            let user = username.get_untracked().trim().to_string();
                            let phone_v = phone.get_untracked().trim().to_string();
                            let pass = password.get_untracked();
                            let confirm = password_confirm.get_untracked();
                            let school = recovery_school.get_untracked();
                            let color = recovery_color.get_untracked();

                            if last.is_empty() || first.is_empty() {
                                error.set(Some("Le nom et le prénom sont obligatoires.".into()));
                                return;
                            }
                            if user.len() < 3 {
                                error.set(Some(i18n.t_static("setup.err_username")));
                                return;
                            }
                            if pass.len() < 8 {
                                error.set(Some(i18n.t_static("setup.err_password")));
                                return;
                            }
                            if pass != confirm {
                                error.set(Some(i18n.t_static("setup.err_mismatch")));
                                return;
                            }

                            busy.set(true);
                            let navigate_submit = navigate_submit.clone();
                            spawn_local(async move {
                                match api::setup_admin(
                                    first, last, user, pass, phone_v, school, color,
                                )
                                .await
                                {
                                    Ok(session) => {
                                        auth.set_session(Some(session));
                                        navigate_submit("/app", Default::default());
                                    }
                                    Err(e) => error.set(Some(e)),
                                }
                                busy.set(false);
                            });
                        }
                    >
                        <p class="text-[var(--muted)]">{move || i18n.t("setup.local_help")}</p>
                        <Input
                            label="Nom"
                            value=last_name.into()
                            on_input=Callback::new(move |v| last_name.set(v))
                            placeholder="Ex. Niang"
                            required=true
                        />
                        <Input
                            label="Prénom"
                            value=first_name.into()
                            on_input=Callback::new(move |v| first_name.set(v))
                            placeholder="Ex. Assane"
                            required=true
                        />
                        <Input
                            label_key="setup.username"
                            value=username.into()
                            on_input=Callback::new(move |v| username.set(v))
                            placeholder="Ex. commissaire"
                            required=true
                        />
                        <Input
                            label_key="setup.phone"
                            value=phone.into()
                            on_input=Callback::new(move |v| phone.set(v))
                            placeholder="+33 6 …"
                        />
                        <Input
                            label_key="setup.password"
                            r#type="password"
                            value=password.into()
                            on_input=Callback::new(move |v| password.set(v))
                            required=true
                        />
                        <Input
                            label_key="setup.password_confirm"
                            r#type="password"
                            value=password_confirm.into()
                            on_input=Callback::new(move |v| password_confirm.set(v))
                            required=true
                        />
                        <p class="text-sm text-[var(--muted)]">{move || i18n.t("setup.recovery_hint")}</p>
                        <Input
                            label_key="setup.recovery_school"
                            r#type="password"
                            value=recovery_school.into()
                            on_input=Callback::new(move |v| recovery_school.set(v))
                        />
                        <Input
                            label_key="setup.recovery_color"
                            r#type="password"
                            value=recovery_color.into()
                            on_input=Callback::new(move |v| recovery_color.set(v))
                        />
                        <Show when=move || error.get().is_some() && tab.get() == "admin">
                            <p
                                class="rounded-xl bg-[color-mix(in_srgb,var(--brand-red)_12%,transparent)] px-3 py-2 text-[var(--brand-red)]"
                                role="alert"
                            >
                                {move || error.get().unwrap_or_default()}
                            </p>
                        </Show>
                        <Button r#type="submit">
                            {move || {
                                if busy.get() {
                                    i18n.t("setup.creating")
                                } else {
                                    i18n.t("setup.create_admin")
                                }
                            }}
                        </Button>
                    </form>
                </div>
            </div>
        </div>
    }
}

fn info_or_tab_admin(error: &RwSignal<Option<String>>, tab: &RwSignal<&'static str>, msg: &str) {
    error.set(Some(msg.into()));
    tab.set("admin");
}
