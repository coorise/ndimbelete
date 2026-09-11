//! Collaboration hub for platform data sync (not source code).

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::app::components::ui::{Button, ButtonVariant, Input, Modal, TabItem, Tabs};
use crate::app::hooks::use_auth;
use crate::app::i18n::use_i18n;
use crate::app::lib::api;
use crate::app::lib::types::{ActivityEntry, CollabCommitInfo, CollabStatus, Role, Staff};

#[component]
pub fn CollaborationPage() -> impl IntoView {
    let i18n = use_i18n();
    let auth = use_auth();
    let tab = RwSignal::new("data_versions");
    let status = RwSignal::new(Option::<CollabStatus>::None);
    let versions = RwSignal::new(Vec::<CollabCommitInfo>::new());
    let activities = RwSignal::new(Vec::<ActivityEntry>::new());
    let roles = RwSignal::new(Vec::<Role>::new());
    let staff = RwSignal::new(Vec::<Staff>::new());
    let keep = RwSignal::new("50".to_string());
    let uri = RwSignal::new(String::new());
    let push_msg = RwSignal::new(String::new());
    let error = RwSignal::new(Option::<String>::None);
    let info = RwSignal::new(Option::<String>::None);
    let busy = RwSignal::new(false);
    let selected_role_ids = RwSignal::new(Vec::<String>::new());
    let selected_staff_ids = RwSignal::new(Vec::<String>::new());
    let role_query = RwSignal::new(String::new());
    let user_query = RwSignal::new(String::new());
    let remote_login_open = RwSignal::new(false);
    let remote_user = RwSignal::new(String::new());
    let remote_pass = RwSignal::new(String::new());
    let pull_confirm_open = RwSignal::new(false);
    let restore_confirm_open = RwSignal::new(false);
    let restore_commit_id = RwSignal::new(Option::<String>::None);

    let ensure_root_in_acl = move |staff_ids: &mut Vec<String>, root: Option<&str>| {
        if let Some(rid) = root {
            if !staff_ids.iter().any(|id| id == rid) {
                staff_ids.push(rid.to_string());
            }
        }
    };

    let load = move || {
        spawn_local(async move {
            if let Ok(Some(u)) = api::collab_get_default_uri().await {
                if uri.get_untracked().trim().is_empty() {
                    uri.set(u);
                }
            }
            match api::collab_status().await {
                Ok(s) => {
                    keep.set(s.keep_commits.to_string());
                    let root = s.root_staff_id.clone();
                    let mut roles_sel = s.push_role_ids.clone();
                    let mut staff_sel = s.push_staff_ids.clone();
                    ensure_root_in_acl(&mut staff_sel, root.as_deref());
                    let _ = &mut roles_sel;
                    selected_role_ids.set(s.push_role_ids.clone());
                    selected_staff_ids.set(staff_sel);
                    status.set(Some(s));
                }
                Err(e) => error.set(Some(e)),
            }
            match api::collab_list_commits().await {
                Ok(c) => versions.set(c),
                Err(e) => error.set(Some(e)),
            }
            match api::collab_list_activities(Some(200)).await {
                Ok(a) => activities.set(a),
                Err(e) => error.set(Some(e)),
            }
            if let Ok(r) = api::list_roles().await {
                roles.set(r);
            }
            if let Ok(s) = api::list_staff().await {
                staff.set(s);
            }
        });
    };

    Effect::new(move |_| {
        load();
    });

    let can_manage_settings = Signal::derive(move || {
        auth.session.get().map(|s| {
            s.staff.is_founder
                || s.permissions.iter().any(|p| {
                    p == "*" || p == "staff:write" || p == "settings:write" || p == "collab:manage"
                })
        })
        .unwrap_or(false)
    });

    let is_root = Signal::derive(move || {
        auth.session
            .get()
            .map(|s| s.staff.is_founder)
            .unwrap_or(false)
    });

    let tab_items = Signal::derive(move || {
        let mut items = vec![
            TabItem {
                id: "data_versions",
                label: i18n.t("collab.tab_data_versions"),
            },
            TabItem {
                id: "commits",
                label: i18n.t("collab.tab_commits"),
            },
        ];
        if can_manage_settings.get() {
            items.push(TabItem {
                id: "settings",
                label: i18n.t("collab.tab_settings"),
            });
        }
        items
    });

    Effect::new(move |_| {
        if !can_manage_settings.get() && tab.get() == "settings" {
            tab.set("data_versions");
        }
    });

    let finish_connect = move |u: String, username: Option<String>, password: Option<String>| {
        busy.set(true);
        error.set(None);
        info.set(None);
        spawn_local(async move {
            match api::collab_probe(&u).await {
                Ok(probe) => {
                    info.set(Some(probe.message.clone()));
                    let need_login = probe.has_head
                        && (username.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true)
                            || password.as_ref().map(|s| s.is_empty()).unwrap_or(true));
                    if need_login {
                        remote_login_open.set(true);
                        busy.set(false);
                        return;
                    }
                    let user_ref = username.as_deref().filter(|s| !s.trim().is_empty());
                    let pass_ref = password.as_deref().filter(|s| !s.is_empty());
                    match api::collab_connect(&u, user_ref, pass_ref).await {
                        Ok(res) => {
                            if res.needs_remote_login {
                                remote_login_open.set(true);
                            } else {
                                if let Some(session) = res.session {
                                    auth.set_session(Some(session));
                                }
                                status.set(Some(res.status));
                                remote_login_open.set(false);
                                remote_user.set(String::new());
                                remote_pass.set(String::new());
                                info.set(Some(i18n.t_static("collab.connect_ok")));
                                load();
                            }
                        }
                        Err(e) => error.set(Some(e)),
                    }
                }
                Err(e) => error.set(Some(e)),
            }
            busy.set(false);
        });
    };

    let do_push = move || {
        if !status.get_untracked().map(|s| s.can_push).unwrap_or(false) {
            error.set(Some(i18n.t_static("collab.err_cannot_push")));
            return;
        }
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
                    info.set(Some(i18n.t_static("collab.push_ok")));
                    push_msg.set(String::new());
                    load();
                }
                Err(e) => error.set(Some(e)),
            }
            busy.set(false);
        });
    };

    let do_pull = move || {
        pull_confirm_open.set(true);
    };

    let confirm_pull = move || {
        busy.set(true);
        pull_confirm_open.set(false);
        spawn_local(async move {
            match api::collab_pull().await {
                Ok(_) => {
                    let _ = api::restart_app().await;
                }
                Err(e) => {
                    error.set(Some(e));
                    busy.set(false);
                }
            }
        });
    };

    let do_connect = move || {
        let u = uri.get_untracked().trim().to_string();
        if u.is_empty() {
            error.set(Some("URI obligatoire".into()));
            return;
        }
        finish_connect(u, None, None);
    };

    let do_disconnect = move || {
        busy.set(true);
        spawn_local(async move {
            match api::collab_disconnect().await {
                Ok(()) => {
                    info.set(Some(i18n.t_static("collab.disconnect_ok")));
                    load();
                }
                Err(e) => error.set(Some(e)),
            }
            busy.set(false);
        });
    };

    let root_id = Signal::derive(move || {
        status
            .get()
            .and_then(|s| s.root_staff_id)
            .or_else(|| {
                staff
                    .get()
                    .into_iter()
                    .find(|p| p.is_founder)
                    .map(|p| p.id)
            })
    });

    view! {
        <div class="mx-auto w-full max-w-5xl space-y-6 pb-10">
            <header>
                <h1 class="font-display text-3xl font-semibold">{move || i18n.t("collab.title")}</h1>
                <p class="mt-1 text-[var(--muted)]">{move || i18n.t("collab.subtitle")}</p>
            </header>

            // Row 1: URI + Connect XOR Disconnect
            <div class="flex flex-col gap-3 sm:flex-row sm:items-end">
                <div class="min-w-0 flex-1">
                    <Input
                        label_key="collab.uri"
                        value=Signal::derive(move || {
                            if status.get().map(|s| s.connected).unwrap_or(false) {
                                status
                                    .get()
                                    .and_then(|s| s.uri_masked)
                                    .unwrap_or_else(|| crate::app::lib::mask_postgres_uri(&uri.get()))
                            } else {
                                crate::app::lib::mask_postgres_uri(&uri.get())
                            }
                        }).into()
                        on_input=Callback::new(move |v: String| {
                            if status.get_untracked().map(|s| s.connected).unwrap_or(false) {
                                return;
                            }
                            // Ignore edits of the already-masked display; accept a fresh pasted URI.
                            if v.contains(":***") {
                                return;
                            }
                            uri.set(v);
                        })
                        placeholder="postgres://user:pass@host:port/db?sslmode=require"
                        disabled_signal=Signal::derive(move || {
                            status.get().map(|s| s.connected).unwrap_or(false)
                        })
                    />
                </div>
                <div class="flex shrink-0 flex-wrap gap-2">
                    <Show when=move || !status.get().map(|s| s.connected).unwrap_or(false)>
                        <button
                            type="button"
                            class="tap-target rounded-xl border border-sky-500 bg-sky-600 px-3 py-2.5 text-sm font-semibold text-white hover:bg-sky-700 disabled:opacity-50"
                            disabled=move || busy.get()
                            on:click=move |_| do_connect()
                        >
                            {move || {
                                if busy.get() {
                                    i18n.t("common.loading")
                                } else {
                                    i18n.t("collab.connect")
                                }
                            }}
                        </button>
                    </Show>
                    <Show when=move || status.get().map(|s| s.connected).unwrap_or(false)>
                        <button
                            type="button"
                            class="tap-target rounded-xl border border-[var(--brand-red)] bg-[var(--brand-red)] px-3 py-2.5 text-sm font-semibold text-white hover:brightness-110 disabled:opacity-50"
                            disabled=move || busy.get()
                            on:click=move |_| do_disconnect()
                        >
                            {move || i18n.t("collab.disconnect")}
                        </button>
                    </Show>
                </div>
            </div>

            <p class="text-sm text-[var(--muted)]">
                {move || {
                    let s = status.get();
                    if s.as_ref().map(|x| x.connected).unwrap_or(false) {
                        let local_n = s.as_ref().map(|x| x.local_change_count).unwrap_or(0);
                        let remote_n = s.as_ref().map(|x| x.remote_ahead_count).unwrap_or(0);
                        if local_n > 0 {
                            format!(
                                "{} · {} ({})",
                                s.as_ref().and_then(|x| x.remote_commit_count).unwrap_or(0),
                                i18n.t("collab.dirty"),
                                local_n
                            )
                        } else if remote_n > 0 {
                            format!(
                                "{} · {} ({})",
                                s.as_ref().and_then(|x| x.remote_commit_count).unwrap_or(0),
                                i18n.t("collab.remote_pending"),
                                remote_n
                            )
                        } else {
                            format!(
                                "{} · {}",
                                s.as_ref().and_then(|x| x.remote_commit_count).unwrap_or(0),
                                i18n.t("collab.clean")
                            )
                        }
                    } else {
                        i18n.t("collab.not_connected")
                    }
                }}
            </p>

            // Row 2: Push / Pull
            <div class="flex flex-wrap items-center gap-2">
                <button
                    type="button"
                    class=move || {
                        let dirty = status.get().map(|s| s.dirty && s.local_change_count > 0).unwrap_or(false);
                        let can = status.get().map(|s| s.connected && s.can_push).unwrap_or(false);
                        if !can {
                            "relative tap-target inline-flex items-center gap-1 rounded-xl border border-[var(--border)] px-3 py-2.5 text-sm font-semibold opacity-50"
                        } else if dirty {
                            "relative tap-target inline-flex items-center gap-1 rounded-xl border border-[var(--brand)] bg-[var(--brand)] px-3 py-2.5 text-sm font-semibold text-white animate-pulse"
                        } else {
                            "relative tap-target inline-flex items-center gap-1 rounded-xl border border-[var(--brand)] bg-[var(--brand)] px-3 py-2.5 text-sm font-semibold text-white"
                        }
                    }
                    disabled=move || {
                        busy.get()
                            || !status.get().map(|s| s.connected && s.can_push).unwrap_or(false)
                    }
                    on:click=move |_| do_push()
                >
                    <span aria-hidden="true">"↑"</span>
                    {move || i18n.t("collab.push")}
                    <Show when=move || {
                        status.get().map(|s| s.dirty && s.local_change_count > 0).unwrap_or(false)
                    }>
                        <span class="absolute -right-1.5 -top-1.5 flex h-5 min-w-5 items-center justify-center rounded-full bg-[var(--brand-red)] px-1 text-[10px] font-bold text-white">
                            {move || {
                                status
                                    .get()
                                    .map(|s| s.local_change_count)
                                    .unwrap_or(0)
                                    .to_string()
                            }}
                        </span>
                    </Show>
                </button>
                <button
                    type="button"
                    class=move || {
                        let pending = status.get().map(|s| s.remote_ahead_count > 0).unwrap_or(false);
                        let connected = status.get().map(|s| s.connected).unwrap_or(false);
                        if !connected {
                            "btn-collab-pull relative tap-target inline-flex items-center gap-1 rounded-xl px-3 py-2.5 text-sm font-semibold opacity-50"
                        } else if pending {
                            "btn-collab-pull relative tap-target inline-flex items-center gap-1 rounded-xl px-3 py-2.5 text-sm font-semibold animate-pulse"
                        } else {
                            "btn-collab-pull relative tap-target inline-flex items-center gap-1 rounded-xl px-3 py-2.5 text-sm font-semibold"
                        }
                    }
                    style="background-color:#e6b800;border:1px solid #c99a00;color:#ffffff"
                    disabled=move || {
                        busy.get() || !status.get().map(|s| s.connected).unwrap_or(false)
                    }
                    on:click=move |_| do_pull()
                >
                    <span aria-hidden="true">"↓"</span>
                    {move || i18n.t("collab.pull")}
                    <Show when=move || {
                        status.get().map(|s| s.remote_ahead_count > 0).unwrap_or(false)
                    }>
                        <span class="absolute -right-1.5 -top-1.5 flex h-5 min-w-5 items-center justify-center rounded-full bg-[var(--brand-red)] px-1 text-[10px] font-bold text-white">
                            {move || {
                                status
                                    .get()
                                    .map(|s| s.remote_ahead_count)
                                    .unwrap_or(0)
                                    .to_string()
                            }}
                        </span>
                    </Show>
                </button>
            </div>

            <Show when=move || status.get().map(|s| s.connected && s.can_push).unwrap_or(false)>
                <Input
                    label_key="collab.version_note"
                    value=push_msg.into()
                    on_input=Callback::new(move |v| push_msg.set(v))
                    placeholder="Note optionnelle pour cette version de données"
                />
            </Show>

            <Tabs items=tab_items active=tab />

            <Show when=move || error.get().is_some()>
                <p class="text-[var(--brand-red)]" role="alert">{move || error.get().unwrap_or_default()}</p>
            </Show>
            <Show when=move || info.get().is_some()>
                <p class="text-[var(--brand)]">{move || info.get().unwrap_or_default()}</p>
            </Show>

            // —— Data versions ——
            <div class:hidden=move || tab.get() != "data_versions">
                <p class="mb-3 text-sm text-[var(--muted)]">{move || i18n.t("collab.data_versions_help")}</p>
                <ul class="divide-y divide-[var(--border)] rounded-2xl border border-[var(--border)] bg-[var(--bg-elevated)]">
                    <For
                        each=move || versions.get()
                        key=|c| c.id.clone()
                        children=move |c| {
                            let id = c.id.clone();
                            let id_roll = c.id.clone();
                            view! {
                                <li class="flex flex-col gap-2 px-4 py-3 sm:flex-row sm:items-center sm:justify-between">
                                    <div class="min-w-0">
                                        <div class="flex flex-wrap items-center gap-2">
                                            <p class="font-semibold">{c.message.clone()}</p>
                                            <Show when=move || c.is_head>
                                                <span class="rounded-md bg-[var(--brand)] px-1.5 py-0.5 text-[10px] font-bold uppercase text-white">
                                                    {move || i18n.t("collab.current")}
                                                </span>
                                            </Show>
                                        </div>
                                        <p class="truncate text-xs text-[var(--muted)]">
                                            {format!(
                                                "{} · {}",
                                                c.author_name.clone().unwrap_or_else(|| "—".into()),
                                                c.created_at.clone()
                                            )}
                                        </p>
                                    </div>
                                    <Button
                                        variant=ButtonVariant::Secondary
                                        on_click=Callback::new(move |_| {
                                            restore_commit_id.set(Some(id_roll.clone()));
                                            restore_confirm_open.set(true);
                                            let _ = id;
                                        })
                                    >
                                        {move || i18n.t("collab.rollback")}
                                    </Button>
                                </li>
                            }
                        }
                    />
                </ul>
                <Show when=move || versions.get().is_empty()>
                    <p class="mt-3 text-sm text-[var(--muted)]">{move || i18n.t("collab.no_versions")}</p>
                </Show>
            </div>

            // —— Commits ——
            <div class:hidden=move || tab.get() != "commits">
                <p class="mb-3 text-sm text-[var(--muted)]">{move || i18n.t("collab.commits_help")}</p>
                <ul class="divide-y divide-[var(--border)] rounded-2xl border border-[var(--border)] bg-[var(--bg-elevated)]">
                    <For
                        each=move || activities.get()
                        key=|a| a.id.clone()
                        children=move |a| {
                            view! {
                                <li class="px-4 py-3">
                                    <div class="flex flex-wrap items-center gap-2">
                                        <span class="rounded-md bg-[color-mix(in_srgb,var(--fg)_8%,transparent)] px-2 py-0.5 text-xs font-semibold uppercase">
                                            {a.area.clone()}
                                        </span>
                                        <span class="text-xs text-[var(--muted)]">{a.action.clone()}</span>
                                    </div>
                                    <p class="mt-1 font-medium">{a.summary.clone()}</p>
                                    <p class="mt-0.5 text-xs text-[var(--muted)]">
                                        {format!(
                                            "{} · {}",
                                            a.staff_name.clone().unwrap_or_else(|| "—".into()),
                                            a.created_at.clone()
                                        )}
                                    </p>
                                </li>
                            }
                        }
                    />
                </ul>
                <Show when=move || activities.get().is_empty()>
                    <p class="mt-3 text-sm text-[var(--muted)]">{move || i18n.t("collab.no_activities")}</p>
                </Show>
            </div>

            // —— Settings (root / elevated only) ——
            <Show when=move || can_manage_settings.get()>
                <div class="flex flex-col gap-6" class:hidden=move || tab.get() != "settings">
                    <section class="space-y-3">
                        <h2 class="font-display text-xl font-semibold">{move || i18n.t("collab.retention_title")}</h2>
                        <p class="text-sm text-[var(--muted)]">{move || i18n.t("collab.settings_help")}</p>
                        <Input
                            label_key="collab.keep_commits"
                            value=keep.into()
                            on_input=Callback::new(move |v| keep.set(v))
                        />
                        <div class="flex flex-wrap gap-2">
                            <Button on_click=Callback::new(move |_| {
                                let k: u32 = keep.get_untracked().parse().unwrap_or(50).max(1);
                                busy.set(true);
                                spawn_local(async move {
                                    match api::collab_set_keep_commits(k).await {
                                        Ok(()) => {
                                            match api::collab_cleanup(Some(k)).await {
                                                Ok(n) => {
                                                    info.set(Some(format!(
                                                        "Conservé {k} version(s) — {n} ancienne(s) supprimée(s)."
                                                    )));
                                                    load();
                                                }
                                                Err(e) => error.set(Some(e)),
                                            }
                                        }
                                        Err(e) => error.set(Some(e)),
                                    }
                                    busy.set(false);
                                });
                            })>
                                {move || i18n.t("collab.save_cleanup")}
                            </Button>
                            <Button
                                variant=ButtonVariant::Secondary
                                on_click=Callback::new(move |_| {
                                    let k: u32 = keep.get_untracked().parse().unwrap_or(50).max(1);
                                    spawn_local(async move {
                                        match api::collab_cleanup(Some(k)).await {
                                            Ok(n) => {
                                                info.set(Some(format!("{n} ancienne(s) version(s) libérée(s).")));
                                                load();
                                            }
                                            Err(e) => error.set(Some(e)),
                                        }
                                    });
                                })
                            >
                                {move || i18n.t("collab.manual_cleanup")}
                            </Button>
                            <Show when=move || is_root.get()>
                                <Button
                                    variant=ButtonVariant::Secondary
                                    on_click=Callback::new(move |_| {
                                        if !web_sys::window()
                                            .map(|w| {
                                                w.confirm_with_message(
                                                    "Effacer TOUTES les versions distantes ? Irréversible.",
                                                )
                                                .unwrap_or(false)
                                            })
                                            .unwrap_or(false)
                                        {
                                            return;
                                        }
                                        spawn_local(async move {
                                            match api::collab_clear_remote().await {
                                                Ok(()) => {
                                                    info.set(Some("Versions distantes vidées.".into()));
                                                    load();
                                                }
                                                Err(e) => error.set(Some(e)),
                                            }
                                        });
                                    })
                                >
                                    {move || i18n.t("collab.clear_remote")}
                                </Button>
                            </Show>
                        </div>
                    </section>

                    <section class="space-y-4">
                        <h2 class="font-display text-xl font-semibold">{move || i18n.t("collab.acl_title")}</h2>
                        <p class="text-sm text-[var(--muted)]">{move || i18n.t("collab.acl_help")}</p>

                        // Roles picker
                        <div class="space-y-2">
                            <p class="text-sm font-semibold">{move || i18n.t("collab.acl_roles")}</p>
                            <div class="flex flex-col gap-2 sm:flex-row">
                                <div class="min-w-0 flex-1">
                                    <Input
                                        label_key="collab.acl_search_role"
                                        value=role_query.into()
                                        on_input=Callback::new(move |v| role_query.set(v))
                                        placeholder="Trésorier…"
                                    />
                                </div>
                            </div>
                            <Show when=move || !role_query.get().trim().is_empty()>
                                <ul class="max-h-36 overflow-y-auto rounded-xl border border-[var(--border)] bg-[var(--bg-elevated)]">
                                    <For
                                        each=move || {
                                            let q = role_query.get().trim().to_lowercase();
                                            let selected = selected_role_ids.get();
                                            roles
                                                .get()
                                                .into_iter()
                                                .filter(|r| {
                                                    !selected.iter().any(|id| id == &r.id)
                                                        && r.name.to_lowercase().contains(&q)
                                                })
                                                .collect::<Vec<_>>()
                                        }
                                        key=|r| r.id.clone()
                                        children=move |r| {
                                            let rid = r.id.clone();
                                            let name = r.name.clone();
                                            view! {
                                                <li>
                                                    <button
                                                        type="button"
                                                        class="flex w-full items-center justify-between px-3 py-2 text-left text-sm hover:bg-[color-mix(in_srgb,var(--brand)_10%,transparent)]"
                                                        on:click=move |_| {
                                                            selected_role_ids.update(|ids| {
                                                                if !ids.iter().any(|id| id == &rid) {
                                                                    ids.push(rid.clone());
                                                                }
                                                            });
                                                            role_query.set(String::new());
                                                        }
                                                    >
                                                        <span>{name}</span>
                                                        <span class="text-xs font-semibold text-[var(--brand)]">{move || i18n.t("collab.acl_add")}</span>
                                                    </button>
                                                </li>
                                            }
                                        }
                                    />
                                </ul>
                            </Show>
                            <ul class="flex flex-col gap-1">
                                <For
                                    each=move || {
                                        let selected = selected_role_ids.get();
                                        roles
                                            .get()
                                            .into_iter()
                                            .filter(|r| selected.iter().any(|id| id == &r.id))
                                            .collect::<Vec<_>>()
                                    }
                                    key=|r| r.id.clone()
                                    children=move |r| {
                                        let rid = r.id.clone();
                                        let name = r.name.clone();
                                        view! {
                                            <li class="flex items-center justify-between rounded-xl border border-[var(--border)] px-3 py-2 text-sm">
                                                <span>{name}</span>
                                                <button
                                                    type="button"
                                                    class="text-xs font-semibold text-red-600 hover:underline"
                                                    on:click=move |_| {
                                                        selected_role_ids.update(|ids| {
                                                            ids.retain(|id| id != &rid);
                                                        });
                                                    }
                                                >
                                                    {move || i18n.t("collab.acl_remove")}
                                                </button>
                                            </li>
                                        }
                                    }
                                />
                            </ul>
                        </div>

                        // Users picker
                        <div class="space-y-2">
                            <p class="text-sm font-semibold">{move || i18n.t("collab.acl_users")}</p>
                            <Input
                                label_key="collab.acl_search_user"
                                value=user_query.into()
                                on_input=Callback::new(move |v| user_query.set(v))
                                placeholder="Assane…"
                            />
                            <Show when=move || !user_query.get().trim().is_empty()>
                                <ul class="max-h-36 overflow-y-auto rounded-xl border border-[var(--border)] bg-[var(--bg-elevated)]">
                                    <For
                                        each=move || {
                                            let q = user_query.get().trim().to_lowercase();
                                            let selected = selected_staff_ids.get();
                                            staff
                                                .get()
                                                .into_iter()
                                                .filter(|p| {
                                                    p.is_active
                                                        && !selected.iter().any(|id| id == &p.id)
                                                        && (p.full_name.to_lowercase().contains(&q)
                                                            || p.username.to_lowercase().contains(&q))
                                                })
                                                .collect::<Vec<_>>()
                                        }
                                        key=|p| p.id.clone()
                                        children=move |p| {
                                            let sid = p.id.clone();
                                            let label = format!("{} ({})", p.full_name, p.username);
                                            view! {
                                                <li>
                                                    <button
                                                        type="button"
                                                        class="flex w-full items-center justify-between px-3 py-2 text-left text-sm hover:bg-[color-mix(in_srgb,var(--brand)_10%,transparent)]"
                                                        on:click=move |_| {
                                                            selected_staff_ids.update(|ids| {
                                                                if !ids.iter().any(|id| id == &sid) {
                                                                    ids.push(sid.clone());
                                                                }
                                                            });
                                                            user_query.set(String::new());
                                                        }
                                                    >
                                                        <span>{label}</span>
                                                        <span class="text-xs font-semibold text-[var(--brand)]">{move || i18n.t("collab.acl_add")}</span>
                                                    </button>
                                                </li>
                                            }
                                        }
                                    />
                                </ul>
                            </Show>
                            <ul class="flex flex-col gap-1">
                                <For
                                    each=move || {
                                        let selected = selected_staff_ids.get();
                                        let root = root_id.get();
                                        staff
                                            .get()
                                            .into_iter()
                                            .filter(|p| selected.iter().any(|id| id == &p.id))
                                            .map(|p| {
                                                let is_root_row = root.as_ref() == Some(&p.id) || p.is_founder;
                                                (p, is_root_row)
                                            })
                                            .collect::<Vec<_>>()
                                    }
                                    key=|(p, _)| p.id.clone()
                                    children=move |(p, is_root_row)| {
                                        let sid_rm = p.id.clone();
                                        let label = format!("{} ({})", p.full_name, p.username);
                                        view! {
                                            <li class="flex items-center justify-between rounded-xl border border-[var(--border)] px-3 py-2 text-sm">
                                                <div class="flex min-w-0 flex-col">
                                                    <span class="truncate">{label}</span>
                                                    <Show when=move || is_root_row>
                                                        <span class="text-xs text-[var(--muted)]">{move || i18n.t("collab.acl_root")}</span>
                                                    </Show>
                                                </div>
                                                <Show
                                                    when=move || !is_root_row
                                                    fallback=|| {
                                                        view! {
                                                            <input type="checkbox" checked=true disabled=true class="h-4 w-4 opacity-60" />
                                                        }
                                                    }
                                                >
                                                    <button
                                                        type="button"
                                                        class="text-xs font-semibold text-red-600 hover:underline"
                                                        on:click={
                                                            let sid_rm = sid_rm.clone();
                                                            move |_| {
                                                                selected_staff_ids.update(|ids| {
                                                                    ids.retain(|id| id != &sid_rm);
                                                                });
                                                            }
                                                        }
                                                    >
                                                        {move || i18n.t("collab.acl_remove")}
                                                    </button>
                                                </Show>
                                            </li>
                                        }
                                    }
                                />
                            </ul>
                        </div>

                        <Button on_click=Callback::new(move |_| {
                            let mut role_ids = selected_role_ids.get_untracked();
                            let mut staff_ids = selected_staff_ids.get_untracked();
                            if let Some(rid) = root_id.get_untracked() {
                                if !staff_ids.iter().any(|id| id == &rid) {
                                    staff_ids.push(rid);
                                }
                            }
                            let _ = &mut role_ids;
                            spawn_local(async move {
                                match api::collab_set_push_acl(role_ids, staff_ids).await {
                                    Ok(()) => {
                                        info.set(Some("Autorisations d'envoi enregistrées.".into()));
                                        load();
                                    }
                                    Err(e) => error.set(Some(e)),
                                }
                            });
                        })>
                            {move || i18n.t("collab.save_acl")}
                        </Button>
                    </section>
                </div>
            </Show>
        </div>

        <Modal
            open=remote_login_open.into()
            on_close=Callback::new(move |_| remote_login_open.set(false))
            title_signal=Signal::derive(move || i18n.t("collab.remote_login_title"))
        >
            <div class="flex flex-col gap-3">
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
                <Show when=move || error.get().is_some()>
                    <p class="text-sm text-[var(--brand-red)]" role="alert">
                        {move || error.get().unwrap_or_default()}
                    </p>
                </Show>
                <div class="flex justify-end gap-2">
                    <Button
                        variant=ButtonVariant::Secondary
                        on_click=Callback::new(move |_| remote_login_open.set(false))
                    >
                        {move || i18n.t("common.cancel")}
                    </Button>
                    <Button on_click=Callback::new(move |_| {
                        let u = uri.get_untracked().trim().to_string();
                        let user = remote_user.get_untracked();
                        let pass = remote_pass.get_untracked();
                        if user.trim().is_empty() || pass.is_empty() {
                            error.set(Some("Identifiant et mot de passe obligatoires".into()));
                            return;
                        }
                        finish_connect(u, Some(user), Some(pass));
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
            open=pull_confirm_open.into()
            on_close=Callback::new(move |_| pull_confirm_open.set(false))
            title_signal=Signal::derive(move || i18n.t("collab.pull"))
        >
            <div class="flex flex-col gap-3">
                <p class="text-sm text-[var(--muted)]">
                    "Recevoir écrasera les données locales avec la version distante. Continuer ?"
                </p>
                <div class="flex justify-end gap-2">
                    <Button
                        variant=ButtonVariant::Secondary
                        on_click=Callback::new(move |_| pull_confirm_open.set(false))
                    >
                        {move || i18n.t("common.cancel")}
                    </Button>
                    <Button on_click=Callback::new(move |_| confirm_pull())>
                        {move || i18n.t("collab.pull")}
                    </Button>
                </div>
            </div>
        </Modal>

        <Modal
            open=restore_confirm_open.into()
            on_close=Callback::new(move |_| {
                restore_confirm_open.set(false);
                restore_commit_id.set(None);
            })
            title_signal=Signal::derive(move || i18n.t("collab.rollback"))
        >
            <div class="flex flex-col gap-3">
                <p class="text-sm text-[var(--muted)]">
                    "Restaurer cette version complète des données en local ?"
                </p>
                <div class="flex justify-end gap-2">
                    <Button
                        variant=ButtonVariant::Secondary
                        on_click=Callback::new(move |_| {
                            restore_confirm_open.set(false);
                            restore_commit_id.set(None);
                        })
                    >
                        {move || i18n.t("common.cancel")}
                    </Button>
                    <Button on_click=Callback::new(move |_| {
                        let Some(cid) = restore_commit_id.get_untracked() else {
                            return;
                        };
                        restore_confirm_open.set(false);
                        busy.set(true);
                        spawn_local(async move {
                            match api::collab_rollback(&cid).await {
                                Ok(_) => {
                                    let _ = api::restart_app().await;
                                }
                                Err(e) => {
                                    error.set(Some(e));
                                    busy.set(false);
                                }
                            }
                        });
                    })>
                        {move || i18n.t("collab.rollback")}
                    </Button>
                </div>
            </div>
        </Modal>
    }
}
