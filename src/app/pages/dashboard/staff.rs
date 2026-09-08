use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::HashMap;

use crate::app::components::ui::{
    Badge, Button, ButtonVariant, EditableCell, Input, Modal, Select, SelectOption, TabItem, Table,
    TableFullscreenToggle, TableLoadMode, TablePaginationBar, Tabs, TBody, Td, Th, THead, Tr,
    paginate_slice,
};
use crate::app::hooks::use_table_fullscreen;
use crate::app::i18n::use_i18n;
use crate::app::lib::{
    api, parse_permissions_list, CreateRoleInput, CreateStaffInput, Role, Staff,
    UpdateRoleInput, UpdateStaffInput, PERMISSION_OPTIONS,
};

fn permission_label(key: &str) -> String {
    PERMISSION_OPTIONS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, label)| (*label).to_string())
        .unwrap_or_else(|| key.to_string())
}

fn staff_display_name(s: &Staff) -> String {
    let last = s.last_name.trim();
    let first = s.first_name.trim();
    if !last.is_empty() || !first.is_empty() {
        format!("{last} {first}").trim().to_string()
    } else {
        s.full_name.clone()
    }
}

fn toggle_permission(selected: RwSignal<Vec<String>>, key: String) {
    selected.update(|sel| {
        if key == "*" {
            if sel.iter().any(|p| p == "*") {
                sel.clear();
            } else {
                *sel = vec!["*".into()];
            }
            return;
        }
        sel.retain(|p| p != "*");
        if let Some(i) = sel.iter().position(|p| p == &key) {
            sel.remove(i);
        } else {
            sel.push(key);
        }
    });
}

#[component]
pub fn StaffPage() -> impl IntoView {
    let i18n = use_i18n();
    let tab = RwSignal::new("staff");
    let staff = RwSignal::new(Vec::<Staff>::new());
    let roles = RwSignal::new(Vec::<Role>::new());
    let error = RwSignal::new(Option::<String>::None);

    let reload = move || {
        spawn_local(async move {
            match (api::list_staff().await, api::list_roles().await) {
                (Ok(s), Ok(r)) => {
                    staff.set(s);
                    roles.set(r);
                    error.set(None);
                }
                (Err(e), _) | (_, Err(e)) => error.set(Some(e)),
            }
        });
    };

    Effect::new(move |_| reload());

    let fs = use_table_fullscreen();

    view! {
        <div class="flex min-h-0 flex-1 flex-col gap-4">
            <Show when=move || !fs.active.get()>
                <div class="flex shrink-0 flex-wrap items-end justify-between gap-3">
                    <div>
                        <h1 class="font-display text-3xl font-semibold">{move || i18n.t("staff.title")}</h1>
                        <p class="text-[var(--muted)]">{move || i18n.t("staff.subtitle")}</p>
                    </div>
                </div>
            </Show>

            <Show when=move || !fs.active.get()>
                <div class="shrink-0">
                    <Tabs
                        items=Signal::derive(|| {
                            vec![
                                TabItem {
                                    id: "staff",
                                    label: "Personnel".into(),
                                },
                                TabItem {
                                    id: "roles",
                                    label: "Rôles".into(),
                                },
                            ]
                        })
                        active=tab
                    />
                </div>
            </Show>

            <Show when=move || error.get().is_some()>
                <p class="shrink-0 text-[var(--brand-red)]">{move || error.get().unwrap_or_default()}</p>
            </Show>

            <Show when=move || tab.get() == "staff">
                <StaffTab
                    staff=staff
                    roles=Signal::derive(move || roles.get())
                    on_change=Callback::new(move |_| reload())
                />
            </Show>
            <Show when=move || tab.get() == "roles" && !fs.active.get()>
                <div class="min-h-0 flex-1 overflow-auto">
                    <RolesTab roles=roles on_change=Callback::new(move |_| reload()) />
                </div>
            </Show>
        </div>
    }
}

#[component]
fn StaffTab(
    staff: RwSignal<Vec<Staff>>,
    roles: Signal<Vec<Role>>,
    #[prop(into)] on_change: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();
    let open = RwSignal::new(false);
    let editing = RwSignal::new(Option::<Staff>::None);
    let search = RwSignal::new(String::new());
    let active_filter = RwSignal::new(String::new()); // "" | "1" | "0"
    let username = RwSignal::new(String::new());
    let last_name = RwSignal::new(String::new());
    let first_name = RwSignal::new(String::new());
    let phone = RwSignal::new(String::new());
    let role_id = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let form_error = RwSignal::new(Option::<String>::None);
    let selected = RwSignal::new(Vec::<String>::new());
    let view_mode = RwSignal::new("table".to_string()); // table | grid
    let page = RwSignal::new(0usize);
    let page_size = RwSignal::new(25usize);
    let load_mode = RwSignal::new("page".to_string());
    let lazy_count = RwSignal::new(25usize);
    let pending = RwSignal::new(HashMap::<String, String>::new());
    let fs = use_table_fullscreen();

    let filtered = Signal::derive(move || {
        let q = search.get().trim().to_lowercase();
        let af = active_filter.get();
        staff
            .get()
            .into_iter()
            .filter(|s| {
                let matches_q = q.is_empty()
                    || s.username.to_lowercase().contains(&q)
                    || s.last_name.to_lowercase().contains(&q)
                    || s.first_name.to_lowercase().contains(&q)
                    || s.full_name.to_lowercase().contains(&q);
                let matches_a = match af.as_str() {
                    "1" => s.is_active,
                    "0" => !s.is_active,
                    _ => true,
                };
                matches_q && matches_a
            })
            .collect::<Vec<_>>()
    });

    let filtered_total = Signal::derive(move || filtered.get().len());

    let visible = Signal::derive(move || {
        let rows = filtered.get();
        let mode = TableLoadMode::from_str(&load_mode.get());
        paginate_slice(
            &rows,
            mode,
            page.get(),
            page_size.get(),
            lazy_count.get(),
        )
    });

    let active_options = Signal::derive(move || {
        vec![
            SelectOption {
                value: String::new(),
                label: i18n.t("staff.filter_all"),
            },
            SelectOption {
                value: "1".into(),
                label: i18n.t("staff.filter_active_only"),
            },
            SelectOption {
                value: "0".into(),
                label: i18n.t("staff.filter_inactive_only"),
            },
        ]
    });

    let role_options = Signal::derive(move || {
        roles
            .get()
            .into_iter()
            .map(|r| SelectOption {
                value: r.id,
                label: r.name,
            })
            .collect::<Vec<_>>()
    });

    let open_create = move |_| {
        editing.set(None);
        username.set(String::new());
        last_name.set(String::new());
        first_name.set(String::new());
        phone.set(String::new());
        password.set(String::new());
        if let Some(r) = roles.get_untracked().first() {
            role_id.set(r.id.clone());
        }
        form_error.set(None);
        open.set(true);
    };

    let open_edit = move |s: Staff| {
        editing.set(Some(s.clone()));
        username.set(s.username.clone());
        last_name.set(s.last_name.clone());
        first_name.set(s.first_name.clone());
        if last_name.get_untracked().is_empty() && first_name.get_untracked().is_empty() {
            let parts: Vec<&str> = s.full_name.split_whitespace().collect();
            if let Some((first, rest)) = parts.split_first() {
                first_name.set((*first).to_string());
                last_name.set(rest.join(" "));
            }
        }
        phone.set(s.phone.clone().unwrap_or_default());
        role_id.set(s.role_id.clone());
        password.set(String::new());
        form_error.set(None);
        open.set(true);
    };

    let cell_value = move |id: String, field: &'static str, base: String| {
        let key = format!("{id}:{field}");
        Signal::derive(move || {
            pending
                .get()
                .get(&key)
                .cloned()
                .unwrap_or_else(|| base.clone())
        })
    };

    let queue_edit = move |id: String, field: &'static str, value: String| {
        pending.update(|m| {
            m.insert(format!("{id}:{field}"), value);
        });
    };

    view! {
        <div class="flex min-h-0 flex-1 flex-col gap-4">
            <div class="flex shrink-0 flex-wrap items-end justify-between gap-3">
                <div class="grid flex-1 gap-3 sm:grid-cols-2">
                    <Input
                        label="Recherche"
                        placeholder="Nom, prénom ou identifiant…"
                        value=search.into()
                        on_input=Callback::new(move |v| {
                            search.set(v);
                            page.set(0);
                        })
                    />
                    <Select
                        label="Statut"
                        options=active_options
                        value=active_filter.into()
                        on_change=Callback::new(move |v| {
                            active_filter.set(v);
                            page.set(0);
                        })
                        class="w-full"
                    />
                </div>
                <div class="flex flex-wrap items-center gap-2">
                <div class="flex rounded-xl border border-[var(--border)] p-1">
                    <button
                        type="button"
                        class=move || {
                            if view_mode.get() == "table" {
                                "rounded-lg bg-[var(--brand)] px-3 py-1.5 text-xs font-semibold text-white"
                            } else {
                                "rounded-lg px-3 py-1.5 text-xs font-semibold"
                            }
                        }
                        on:click=move |_| view_mode.set("table".into())
                    >
                        "Tableau"
                    </button>
                    <button
                        type="button"
                        class=move || {
                            if view_mode.get() == "grid" {
                                "rounded-lg bg-[var(--brand)] px-3 py-1.5 text-xs font-semibold text-white"
                            } else {
                                "rounded-lg px-3 py-1.5 text-xs font-semibold"
                            }
                        }
                        on:click=move |_| view_mode.set("grid".into())
                    >
                        "Grille"
                    </button>
                </div>
                <Show when=move || !fs.active.get()>
                    <Button on_click=Callback::new(open_create)>"Nouveau staff"</Button>
                </Show>
                <Show when=move || !selected.get().is_empty()>
                    <Button
                        variant=ButtonVariant::Danger
                        on_click=Callback::new(move |_| {
                            let ids = selected.get_untracked();
                            let list = staff.get_untracked();
                            spawn_local(async move {
                                for id in ids {
                                    if list.iter().any(|s| s.id == id && s.is_founder) {
                                        continue;
                                    }
                                    let _ = api::deactivate_staff(&id).await;
                                }
                                selected.set(Vec::new());
                                on_change.run(());
                            });
                        })
                    >
                        {move || format!("Désactiver ({})", selected.get().len())}
                    </Button>
                </Show>
                </div>
            </div>

            <div class="flex shrink-0 items-center justify-between gap-3">
                <p class="text-sm text-[var(--muted)]">
                    {move || format!("{} personnel", filtered_total.get())}
                </p>
                <Show when=move || view_mode.get() == "grid">
                    <TableFullscreenToggle />
                </Show>
            </div>

            <Show when=move || !pending.get().is_empty()>
                <div class="sticky top-0 z-20 flex shrink-0 flex-wrap items-center justify-between gap-3 rounded-xl border border-[var(--brand)] bg-[var(--surface)] px-4 py-3 shadow-[var(--shadow)]">
                    <p class="text-sm font-medium">
                        {move || format!("{} modification(s) en attente", pending.get().len())}
                    </p>
                    <div class="flex flex-wrap gap-2">
                        <Button
                            variant=ButtonVariant::Secondary
                            on_click=Callback::new(move |_| pending.set(HashMap::new()))
                        >
                            "Annuler"
                        </Button>
                        <Button on_click=Callback::new(move |_| {
                            let map = pending.get_untracked();
                            if map.is_empty() {
                                return;
                            }
                            let list = staff.get_untracked();
                            spawn_local(async move {
                                let mut by_id: HashMap<String, HashMap<String, String>> =
                                    HashMap::new();
                                for (key, val) in map {
                                    if let Some((id, field)) = key.split_once(':') {
                                        by_id
                                            .entry(id.to_string())
                                            .or_default()
                                            .insert(field.to_string(), val);
                                    }
                                }
                                for (id, fields) in by_id {
                                    let Some(s) = list.iter().find(|x| x.id == id) else {
                                        continue;
                                    };
                                    let input = UpdateStaffInput {
                                        id: s.id.clone(),
                                        username: fields
                                            .get("username")
                                            .cloned()
                                            .unwrap_or_else(|| s.username.clone()),
                                        first_name: fields
                                            .get("first_name")
                                            .cloned()
                                            .unwrap_or_else(|| s.first_name.clone()),
                                        last_name: fields
                                            .get("last_name")
                                            .cloned()
                                            .unwrap_or_else(|| s.last_name.clone()),
                                        phone: s.phone.clone(),
                                        role_id: s.role_id.clone(),
                                        password: None,
                                    };
                                    if let Err(e) = api::update_staff(input).await {
                                        form_error.set(Some(e));
                                        return;
                                    }
                                }
                                pending.set(HashMap::new());
                                on_change.run(());
                            });
                        })>
                            {move || format!("Mettre à jour ({})", pending.get().len())}
                        </Button>
                    </div>
                </div>
            </Show>

            <div class="min-h-0 flex-1 overflow-auto">
                <Show when=move || view_mode.get() == "grid">
                    <div class="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
                        <For
                            each=move || visible.get()
                            key=|s| s.id.clone()
                            children=move |s| {
                                let s_edit = s.clone();
                                let id = s.id.clone();
                                let name = staff_display_name(&s);
                                let username_v = s.username.clone();
                                let role_v = s.role_name.clone().unwrap_or_default();
                                let is_active = s.is_active;
                                let deactivate_btn = (!s.is_founder).then(|| {
                                    let id = id.clone();
                                    view! {
                                        <Button
                                            variant=ButtonVariant::Danger
                                            on_click=Callback::new(move |_| {
                                                let id = id.clone();
                                                spawn_local(async move {
                                                    let _ = api::deactivate_staff(&id).await;
                                                    on_change.run(());
                                                });
                                            })
                                        >
                                            "Désactiver"
                                        </Button>
                                    }
                                });
                                view! {
                                    <div class="rounded-2xl border border-[var(--border)] bg-[var(--surface)] p-4 shadow-[var(--shadow)]">
                                        <div class="flex items-start justify-between gap-2">
                                            <div>
                                                <p class="font-display text-lg font-semibold text-[var(--brand)]">{name}</p>
                                                <p class="text-xs text-[var(--muted)]">{username_v}</p>
                                            </div>
                                            <Badge tone=if is_active { "green" } else { "muted" }>
                                                {if is_active { "Actif" } else { "Inactif" }}
                                            </Badge>
                                        </div>
                                        <p class="mt-3 text-sm">
                                            <span class="text-[var(--muted)]">"Rôle : "</span>
                                            {role_v}
                                        </p>
                                        <div class="mt-4 flex flex-wrap gap-2">
                                            <Button
                                                variant=ButtonVariant::Secondary
                                                on_click=Callback::new(move |_| open_edit(s_edit.clone()))
                                            >
                                                "Modifier"
                                            </Button>
                                            {deactivate_btn}
                                        </div>
                                    </div>
                                }
                            }
                        />
                    </div>
                </Show>

                <Show when=move || view_mode.get() == "table">
                    <Table fullscreen_toggle=true>
                        <THead>
                            <Th>
                                <input
                                    type="checkbox"
                                    class="h-4 w-4"
                                    prop:checked=move || {
                                        let list = visible.get();
                                        let sel = selected.get();
                                        !list.is_empty()
                                            && list.iter().all(|s| sel.iter().any(|id| id == &s.id))
                                    }
                                    on:change=move |_| {
                                        let list = visible.get_untracked();
                                        let all = list.iter().all(|s| {
                                            selected.get_untracked().iter().any(|id| id == &s.id)
                                        });
                                        if all {
                                            selected.update(|sel| {
                                                for s in &list {
                                                    if let Some(i) = sel.iter().position(|id| id == &s.id) {
                                                        sel.remove(i);
                                                    }
                                                }
                                            });
                                        } else {
                                            selected.update(|sel| {
                                                for s in list {
                                                    if !sel.iter().any(|id| id == &s.id) {
                                                        sel.push(s.id);
                                                    }
                                                }
                                            });
                                        }
                                    }
                                />
                            </Th>
                            <Th>"NOM"</Th>
                            <Th>"PRENOM"</Th>
                            <Th>"Identifiant"</Th>
                            <Th>"Rôle"</Th>
                            <Th>"Statut"</Th>
                            <Th>"Actions"</Th>
                        </THead>
                        <TBody>
                            <For
                                each=move || visible.get()
                                key=|s| s.id.clone()
                                children=move |s| {
                                    let id_check = s.id.clone();
                                    let id_sel = s.id.clone();
                                    let s_edit = s.clone();
                                    let id_last = s.id.clone();
                                    let id_first = s.id.clone();
                                    let id_user = s.id.clone();
                                    let last_base = s.last_name.clone();
                                    let first_base = s.first_name.clone();
                                    let user_base = s.username.clone();
                                    let deactivate_btn = (!s.is_founder).then(|| {
                                        let id = s.id.clone();
                                        view! {
                                            <Button
                                                variant=ButtonVariant::Danger
                                                on_click=Callback::new(move |_| {
                                                    let id = id.clone();
                                                    spawn_local(async move {
                                                        let _ = api::deactivate_staff(&id).await;
                                                        on_change.run(());
                                                    });
                                                })
                                            >
                                                "Désactiver"
                                            </Button>
                                        }
                                    });
                                    view! {
                                        <Tr>
                                            <Td>
                                                <input
                                                    type="checkbox"
                                                    class="h-4 w-4"
                                                    prop:checked=move || {
                                                        selected.get().iter().any(|x| x == &id_check)
                                                    }
                                                    on:change=move |_| {
                                                        selected.update(|sel| {
                                                            if let Some(i) = sel.iter().position(|x| x == &id_sel) {
                                                                sel.remove(i);
                                                            } else {
                                                                sel.push(id_sel.clone());
                                                            }
                                                        });
                                                    }
                                                />
                                            </Td>
                                            <Td>
                                                <EditableCell
                                                    value=cell_value(id_last.clone(), "last_name", last_base)
                                                    on_commit=Callback::new(move |v| {
                                                        queue_edit(id_last.clone(), "last_name", v)
                                                    })
                                                />
                                            </Td>
                                            <Td>
                                                <EditableCell
                                                    value=cell_value(id_first.clone(), "first_name", first_base)
                                                    on_commit=Callback::new(move |v| {
                                                        queue_edit(id_first.clone(), "first_name", v)
                                                    })
                                                />
                                            </Td>
                                            <Td>
                                                <EditableCell
                                                    value=cell_value(id_user.clone(), "username", user_base)
                                                    on_commit=Callback::new(move |v| {
                                                        queue_edit(id_user.clone(), "username", v)
                                                    })
                                                />
                                            </Td>
                                            <Td>{s.role_name.clone().unwrap_or_default()}</Td>
                                            <Td>
                                                <Badge tone=if s.is_active { "green" } else { "muted" }>
                                                    {if s.is_active { "Actif" } else { "Inactif" }}
                                                </Badge>
                                            </Td>
                                            <Td>
                                                <div class="flex flex-wrap gap-2">
                                                    <Button
                                                        variant=ButtonVariant::Secondary
                                                        on_click=Callback::new(move |_| open_edit(s_edit.clone()))
                                                    >
                                                        "Modifier"
                                                    </Button>
                                                    {deactivate_btn}
                                                </div>
                                            </Td>
                                        </Tr>
                                    }
                                }
                            />
                        </TBody>
                    </Table>
                </Show>
            </div>

            <div class="shrink-0">
                <TablePaginationBar
                    total=filtered_total
                    page=page
                    page_size=page_size
                    mode=load_mode
                    lazy_count=lazy_count
                />
            </div>

            <Modal
                open=open.into()
                title="Staff"
                on_close=Callback::new(move |_| open.set(false))
            >
                <div class="flex flex-col gap-3">
                    <Input label="Identifiant" value=username.into() on_input=Callback::new(move |v| username.set(v)) />
                    <Input label="Nom" value=last_name.into() on_input=Callback::new(move |v| last_name.set(v)) />
                    <Input label="Prénom" value=first_name.into() on_input=Callback::new(move |v| first_name.set(v)) />
                    <Input label="Téléphone" value=phone.into() on_input=Callback::new(move |v| phone.set(v)) />
                    <Select
                        label="Rôle"
                        options=role_options
                        value=role_id.into()
                        on_change=Callback::new(move |v| role_id.set(v))
                    />
                    <Input
                        label="Mot de passe"
                        r#type="password"
                        value=password.into()
                        on_input=Callback::new(move |v| password.set(v))
                    />
                    <p class="text-xs text-[var(--muted)]">
                        "À la création : obligatoire (≥ 8). En modification : laisser vide pour ne pas changer."
                    </p>
                    <Show when=move || form_error.get().is_some()>
                        <p class="text-[var(--brand-red)]">{move || form_error.get().unwrap_or_default()}</p>
                    </Show>
                    <Button on_click=Callback::new(move |_| {
                        let phone_val = phone.get_untracked();
                        let phone_opt = if phone_val.trim().is_empty() {
                            None
                        } else {
                            Some(phone_val)
                        };
                        spawn_local(async move {
                            let result = if let Some(ed) = editing.get_untracked() {
                                let pw = password.get_untracked();
                                api::update_staff(UpdateStaffInput {
                                    id: ed.id,
                                    username: username.get_untracked(),
                                    first_name: first_name.get_untracked(),
                                    last_name: last_name.get_untracked(),
                                    phone: phone_opt,
                                    role_id: role_id.get_untracked(),
                                    password: if pw.is_empty() { None } else { Some(pw) },
                                })
                                .await
                                .map(|_| ())
                            } else {
                                api::create_staff(CreateStaffInput {
                                    username: username.get_untracked(),
                                    password: password.get_untracked(),
                                    first_name: first_name.get_untracked(),
                                    last_name: last_name.get_untracked(),
                                    phone: phone_opt,
                                    role_id: role_id.get_untracked(),
                                    recovery_school: None,
                                    recovery_color: None,
                                })
                                .await
                                .map(|_| ())
                            };
                            match result {
                                Ok(()) => {
                                    open.set(false);
                                    on_change.run(());
                                }
                                Err(e) => form_error.set(Some(e)),
                            }
                        });
                    })>
                        "Enregistrer"
                    </Button>
                </div>
            </Modal>
        </div>
    }
}

#[component]
fn RolesTab(
    roles: RwSignal<Vec<Role>>,
    #[prop(into)] on_change: Callback<()>,
) -> impl IntoView {
    let open = RwSignal::new(false);
    let editing = RwSignal::new(Option::<Role>::None);
    let name = RwSignal::new(String::new());
    let selected_permissions = RwSignal::new(Vec::<String>::new());
    let form_error = RwSignal::new(Option::<String>::None);

    let has_star = Signal::derive(move || {
        selected_permissions.get().iter().any(|p| p == "*")
    });

    view! {
        <div class="flex flex-col gap-4">
            <div class="flex justify-end">
                <Button on_click=Callback::new(move |_| {
                    editing.set(None);
                    name.set(String::new());
                    selected_permissions.set(vec!["*".into()]);
                    form_error.set(None);
                    open.set(true);
                })>
                    "Nouveau rôle"
                </Button>
            </div>

            <Table>
                <THead>
                    <Th>"Nom"</Th>
                    <Th>"Permissions"</Th>
                    <Th>"Actions"</Th>
                </THead>
                <TBody>
                    <For
                        each=move || roles.get()
                        key=|r| r.id.clone()
                        children=move |r| {
                            let r_edit = r.clone();
                            let perm_keys = parse_permissions_list(&r.permissions_json);
                            view! {
                                <Tr>
                                    <Td>{r.name.clone()}</Td>
                                    <Td>
                                        <div class="flex max-w-md flex-wrap gap-1">
                                            <For
                                                each=move || perm_keys.clone()
                                                key=|p| p.clone()
                                                children=move |p| {
                                                    let label = permission_label(&p);
                                                    view! {
                                                        <Badge tone="muted">{label}</Badge>
                                                    }
                                                }
                                            />
                                        </div>
                                    </Td>
                                    <Td>
                                        <Button
                                            variant=ButtonVariant::Secondary
                                            on_click=Callback::new(move |_| {
                                                editing.set(Some(r_edit.clone()));
                                                name.set(r_edit.name.clone());
                                                selected_permissions.set(
                                                    parse_permissions_list(&r_edit.permissions_json),
                                                );
                                                form_error.set(None);
                                                open.set(true);
                                            })
                                        >
                                            "Modifier"
                                        </Button>
                                    </Td>
                                </Tr>
                            }
                        }
                    />
                </TBody>
            </Table>

            <Modal
                open=open.into()
                title="Rôle"
                on_close=Callback::new(move |_| open.set(false))
            >
                <div class="flex flex-col gap-3">
                    <Input label="Nom" value=name.into() on_input=Callback::new(move |v| name.set(v)) />
                    <div class="flex flex-col gap-2">
                        <p class="text-sm font-medium">"Permissions"</p>
                        <div class="flex max-h-64 flex-col gap-2 overflow-auto rounded-xl border border-[var(--border)] p-3">
                            <For
                                each=|| PERMISSION_OPTIONS.to_vec()
                                key=|(k, _)| (*k).to_string()
                                children=move |(key, label)| {
                                    let key_s = key.to_string();
                                    let key_check = key_s.clone();
                                    let key_toggle = key_s.clone();
                                    let is_star = key == "*";
                                    view! {
                                        <label class="flex cursor-pointer items-center gap-2 text-sm">
                                            <input
                                                type="checkbox"
                                                class="h-4 w-4"
                                                prop:checked=move || {
                                                    let sel = selected_permissions.get();
                                                    if is_star {
                                                        sel.iter().any(|p| p == "*")
                                                    } else if has_star.get() {
                                                        false
                                                    } else {
                                                        sel.iter().any(|p| p == &key_check)
                                                    }
                                                }
                                                prop:disabled=move || !is_star && has_star.get()
                                                on:change=move |_| {
                                                    toggle_permission(
                                                        selected_permissions,
                                                        key_toggle.clone(),
                                                    );
                                                }
                                            />
                                            <span>{label}</span>
                                        </label>
                                    }
                                }
                            />
                        </div>
                        <p class="text-xs text-[var(--muted)]">
                            "Si « Toutes les permissions » est cochée, elle seule s'applique."
                        </p>
                    </div>
                    <Show when=move || form_error.get().is_some()>
                        <p class="text-[var(--brand-red)]">{move || form_error.get().unwrap_or_default()}</p>
                    </Show>
                    <Button on_click=Callback::new(move |_| {
                        spawn_local(async move {
                            let mut perms = selected_permissions.get_untracked();
                            if perms.iter().any(|p| p == "*") {
                                perms = vec!["*".into()];
                            }
                            if perms.is_empty() {
                                form_error.set(Some("Sélectionnez au moins une permission.".into()));
                                return;
                            }
                            let result = if let Some(ed) = editing.get_untracked() {
                                api::update_role(UpdateRoleInput {
                                    id: ed.id,
                                    name: name.get_untracked(),
                                    permissions: perms,
                                })
                                .await
                                .map(|_| ())
                            } else {
                                api::create_role(CreateRoleInput {
                                    name: name.get_untracked(),
                                    permissions: perms,
                                })
                                .await
                                .map(|_| ())
                            };
                            match result {
                                Ok(()) => {
                                    open.set(false);
                                    on_change.run(());
                                }
                                Err(e) => form_error.set(Some(e)),
                            }
                        });
                    })>
                        "Enregistrer"
                    </Button>
                </div>
            </Modal>
        </div>
    }
}
