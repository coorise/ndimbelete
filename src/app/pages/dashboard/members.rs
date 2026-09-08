use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::HashMap;

use crate::app::components::ui::{
    Badge, Button, ButtonVariant, EditableCell, Input, Modal, Select, SelectOption, Table,
    TableFullscreenToggle, TableLoadMode, TablePaginationBar, TBody, Td, TextArea, Th, THead, Tr,
    paginate_slice,
};
use crate::app::hooks::use_table_fullscreen;
use crate::app::lib::{
    api, CreateMemberInput, Member, UpdateMemberInput, BANK_TRANSFER_VALUES,
};

#[component]
pub fn MembersPage() -> impl IntoView {
    let fs = use_table_fullscreen();
    let members = RwSignal::new(Vec::<Member>::new());
    let query = RwSignal::new(String::new());
    let error = RwSignal::new(Option::<String>::None);
    let open = RwSignal::new(false);
    let editing = RwSignal::new(Option::<Member>::None);

    // form fields
    let card_number = RwSignal::new(String::new());
    let last_name = RwSignal::new(String::new());
    let first_name = RwSignal::new(String::new());
    let adhesion_fee = RwSignal::new("0".into());
    let address = RwSignal::new(String::new());
    let address_complement = RwSignal::new(String::new());
    let postal_code = RwSignal::new(String::new());
    let city = RwSignal::new(String::new());
    let phone = RwSignal::new(String::new());
    let email = RwSignal::new(String::new());
    let bank = RwSignal::new(String::new());
    let status = RwSignal::new("active".into());
    let notes = RwSignal::new(String::new());
    let form_error = RwSignal::new(Option::<String>::None);
    let selected = RwSignal::new(Vec::<String>::new());
    let view_mode = RwSignal::new("table".to_string()); // table | grid
    let page = RwSignal::new(0usize);
    let page_size = RwSignal::new(25usize);
    let load_mode = RwSignal::new("page".to_string());
    let lazy_count = RwSignal::new(25usize);
    // key = "{id}:{field}" → new value
    let pending = RwSignal::new(HashMap::<String, String>::new());

    let reload = move || {
        let q = query.get_untracked();
        spawn_local(async move {
            let result = if q.trim().is_empty() {
                api::list_members().await
            } else {
                api::search_members(&q).await
            };
            match result {
                Ok(m) => {
                    members.set(m);
                    pending.set(HashMap::new());
                    error.set(None);
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    Effect::new(move |_| {
        let _ = query.get();
        page.set(0);
        reload();
    });

    let filtered_total = Signal::derive(move || members.get().len());

    let visible = Signal::derive(move || {
        let rows = members.get();
        let mode = TableLoadMode::from_str(&load_mode.get());
        paginate_slice(
            &rows,
            mode,
            page.get(),
            page_size.get(),
            lazy_count.get(),
        )
    });

    let bank_options = Signal::derive(|| {
        BANK_TRANSFER_VALUES
            .iter()
            .map(|v| SelectOption {
                value: (*v).to_string(),
                label: if v.is_empty() {
                    "(aucun)".into()
                } else {
                    (*v).to_string()
                },
            })
            .collect::<Vec<_>>()
    });

    let status_options = Signal::derive(|| {
        vec![
            SelectOption {
                value: "active".into(),
                label: "Actif".into(),
            },
            SelectOption {
                value: "demissionnaire".into(),
                label: "Démissionnaire".into(),
            },
            SelectOption {
                value: "exclu".into(),
                label: "Exclu".into(),
            },
        ]
    });

    let reset_form = move || {
        card_number.set(String::new());
        last_name.set(String::new());
        first_name.set(String::new());
        adhesion_fee.set("0".into());
        address.set(String::new());
        address_complement.set(String::new());
        postal_code.set(String::new());
        city.set(String::new());
        phone.set(String::new());
        email.set(String::new());
        bank.set(String::new());
        status.set("active".into());
        notes.set(String::new());
        form_error.set(None);
    };

    let fill_form = move |m: &Member| {
        card_number.set(m.card_number.clone());
        last_name.set(m.last_name.clone());
        first_name.set(m.first_name.clone());
        adhesion_fee.set(format!("{}", m.adhesion_fee));
        address.set(m.address.clone().unwrap_or_default());
        address_complement.set(m.address_complement.clone().unwrap_or_default());
        postal_code.set(m.postal_code.clone().unwrap_or_default());
        city.set(m.city.clone().unwrap_or_default());
        phone.set(m.phone.clone().unwrap_or_default());
        email.set(m.email.clone().unwrap_or_default());
        bank.set(m.bank_transfer_status.clone().unwrap_or_default());
        status.set(m.status.clone());
        notes.set(m.notes.clone().unwrap_or_default());
    };

    let opt = |s: String| {
        let t = s.trim().to_string();
        if t.is_empty() { None } else { Some(t) }
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
        let key = format!("{id}:{field}");
        pending.update(|m| {
            m.insert(key, value);
        });
    };

    view! {
        <div class="flex min-h-0 flex-1 flex-col gap-4">
            <Show when=move || !fs.active.get()>
            <div class="flex shrink-0 flex-wrap items-end justify-between gap-3">
                <div>
                    <h1 class="font-display text-3xl font-semibold">"Membres"</h1>
                    <p class="text-[var(--muted)]">"Recherchez, créez et modifiez les fiches membres."</p>
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
                <Button on_click=Callback::new(move |_| {
                    editing.set(None);
                    reset_form();
                    open.set(true);
                })>
                    "Nouveau membre"
                </Button>
                <Show when=move || !selected.get().is_empty()>
                    <Button
                        variant=ButtonVariant::Danger
                        on_click=Callback::new(move |_| {
                            let ids = selected.get_untracked();
                            spawn_local(async move {
                                match api::delete_members(ids).await {
                                    Ok(_) => {
                                        selected.set(Vec::new());
                                        reload();
                                    }
                                    Err(e) => error.set(Some(e)),
                                }
                            });
                        })
                    >
                        {move || format!("Supprimer ({})", selected.get().len())}
                    </Button>
                </Show>
                </div>
            </div>
            </Show>

            <div class="flex shrink-0 flex-wrap items-end gap-3">
                <Input
                    label="Recherche"
                    placeholder="Nom, carte, téléphone…"
                    value=query.into()
                    on_input=Callback::new(move |v| {
                        query.set(v);
                        page.set(0);
                    })
                    class="min-w-[14rem] flex-1"
                />
                <Show when=move || fs.active.get()>
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
                </Show>
            </div>

            <div class="flex shrink-0 items-center justify-between gap-3">
                <p class="text-sm text-[var(--muted)]">
                    {move || format!("{} membre(s)", filtered_total.get())}
                </p>
                <Show when=move || view_mode.get() == "grid">
                    <TableFullscreenToggle />
                </Show>
            </div>

            <Show when=move || error.get().is_some()>
                <p class="shrink-0 text-[var(--brand-red)]">{move || error.get().unwrap_or_default()}</p>
            </Show>

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
                            let list = members.get_untracked();
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
                                    let Some(m) = list.iter().find(|x| x.id == id) else {
                                        continue;
                                    };
                                    let adhesion_fee = fields
                                        .get("adhesion_fee")
                                        .and_then(|s| s.replace(',', ".").parse().ok())
                                        .unwrap_or(m.adhesion_fee);
                                    let input = UpdateMemberInput {
                                        id: m.id.clone(),
                                        card_number: fields
                                            .get("card_number")
                                            .cloned()
                                            .unwrap_or_else(|| m.card_number.clone()),
                                        last_name: fields
                                            .get("last_name")
                                            .cloned()
                                            .unwrap_or_else(|| m.last_name.clone()),
                                        first_name: fields
                                            .get("first_name")
                                            .cloned()
                                            .unwrap_or_else(|| m.first_name.clone()),
                                        adhesion_fee,
                                        address: m.address.clone(),
                                        address_complement: m.address_complement.clone(),
                                        postal_code: m.postal_code.clone(),
                                        city: fields
                                            .get("city")
                                            .cloned()
                                            .map(|v| if v.trim().is_empty() { None } else { Some(v) })
                                            .unwrap_or_else(|| m.city.clone()),
                                        phone: fields
                                            .get("phone")
                                            .cloned()
                                            .map(|v| if v.trim().is_empty() { None } else { Some(v) })
                                            .unwrap_or_else(|| m.phone.clone()),
                                        email: m.email.clone(),
                                        bank_transfer_status: m.bank_transfer_status.clone(),
                                        status: m.status.clone(),
                                        notes: m.notes.clone(),
                                    };
                                    if let Err(e) = api::update_member(input).await {
                                        error.set(Some(e));
                                        return;
                                    }
                                }
                                pending.set(HashMap::new());
                                reload();
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
                            key=|m| m.id.clone()
                            children=move |m| {
                                let m2 = m.clone();
                                let last = m.last_name.clone();
                                let first = m.first_name.clone();
                                let card = m.card_number.clone();
                                let status_v = m.status.clone();
                                view! {
                                    <div class="rounded-2xl border border-[var(--border)] bg-[var(--surface)] p-4 shadow-[var(--shadow)]">
                                        <div class="flex items-start justify-between gap-2">
                                            <div>
                                                <p class="font-display text-lg font-semibold text-[var(--brand)]">
                                                    {format!("{last} {first}")}
                                                </p>
                                                <p class="text-xs text-[var(--muted)]">{card}</p>
                                            </div>
                                            <Badge tone=match status_v.as_str() {
                                                "exclu" => "red",
                                                "demissionnaire" => "yellow",
                                                _ => "green",
                                            }>
                                                {status_v.clone()}
                                            </Badge>
                                        </div>
                                        <div class="mt-4">
                                            <Button
                                                variant=ButtonVariant::Secondary
                                                on_click=Callback::new(move |_| {
                                                    editing.set(Some(m2.clone()));
                                                    fill_form(&m2);
                                                    open.set(true);
                                                })
                                            >
                                                "Modifier"
                                            </Button>
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
                                            && list.iter().all(|m| sel.iter().any(|id| id == &m.id))
                                    }
                                    on:change=move |_| {
                                        let list = visible.get_untracked();
                                        let all = list.iter().all(|m| {
                                            selected.get_untracked().iter().any(|id| id == &m.id)
                                        });
                                        if all {
                                            selected.update(|sel| {
                                                for m in &list {
                                                    if let Some(i) = sel.iter().position(|id| id == &m.id) {
                                                        sel.remove(i);
                                                    }
                                                }
                                            });
                                        } else {
                                            selected.update(|sel| {
                                                for m in list {
                                                    if !sel.iter().any(|id| id == &m.id) {
                                                        sel.push(m.id);
                                                    }
                                                }
                                            });
                                        }
                                    }
                                />
                            </Th>
                            <Th>"Carte"</Th>
                            <Th>"Nom"</Th>
                            <Th>"Prénom"</Th>
                            <Th>"Ville"</Th>
                            <Th>"Téléphone"</Th>
                            <Th>"Adhésion"</Th>
                            <Th>"Statut"</Th>
                            <Th>" "</Th>
                        </THead>
                        <TBody>
                            <For
                                each=move || visible.get()
                                key=|m| m.id.clone()
                                children=move |m| {
                                    let m2 = m.clone();
                                    let mid = m.id.clone();
                                    let mid_check = m.id.clone();
                                    let id_card = m.id.clone();
                                    let id_last = m.id.clone();
                                    let id_first = m.id.clone();
                                    let id_city = m.id.clone();
                                    let id_phone = m.id.clone();
                                    let id_fee = m.id.clone();
                                    let card_base = m.card_number.clone();
                                    let last_base = m.last_name.clone();
                                    let first_base = m.first_name.clone();
                                    let city_base = m.city.clone().unwrap_or_default();
                                    let phone_base = m.phone.clone().unwrap_or_default();
                                    let fee_base = format!("{}", m.adhesion_fee);
                                    view! {
                                        <Tr>
                                            <Td>
                                                <input
                                                    type="checkbox"
                                                    class="h-4 w-4"
                                                    prop:checked=move || {
                                                        selected.get().iter().any(|id| id == &mid_check)
                                                    }
                                                    on:change=move |_| {
                                                        selected.update(|sel| {
                                                            if let Some(i) = sel.iter().position(|x| x == &mid) {
                                                                sel.remove(i);
                                                            } else {
                                                                sel.push(mid.clone());
                                                            }
                                                        });
                                                    }
                                                />
                                            </Td>
                                            <Td class="font-semibold">
                                                <EditableCell
                                                    value=cell_value(id_card.clone(), "card_number", card_base)
                                                    on_commit=Callback::new(move |v| {
                                                        queue_edit(id_card.clone(), "card_number", v)
                                                    })
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
                                                    value=cell_value(id_city.clone(), "city", city_base)
                                                    on_commit=Callback::new(move |v| {
                                                        queue_edit(id_city.clone(), "city", v)
                                                    })
                                                />
                                            </Td>
                                            <Td>
                                                <EditableCell
                                                    value=cell_value(id_phone.clone(), "phone", phone_base)
                                                    on_commit=Callback::new(move |v| {
                                                        queue_edit(id_phone.clone(), "phone", v)
                                                    })
                                                />
                                            </Td>
                                            <Td>
                                                <EditableCell
                                                    value=cell_value(id_fee.clone(), "adhesion_fee", fee_base)
                                                    on_commit=Callback::new(move |v| {
                                                        queue_edit(id_fee.clone(), "adhesion_fee", v)
                                                    })
                                                    input_type="number"
                                                />
                                            </Td>
                                            <Td>
                                                <Badge tone=match m.status.as_str() {
                                                    "exclu" => "red",
                                                    "demissionnaire" => "yellow",
                                                    _ => "green",
                                                }>
                                                    {m.status.clone()}
                                                </Badge>
                                            </Td>
                                            <Td>
                                                <Button
                                                    variant=ButtonVariant::Secondary
                                                    on_click=Callback::new(move |_| {
                                                        editing.set(Some(m2.clone()));
                                                        fill_form(&m2);
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
                title="Fiche membre"
                wide=true
                on_close=Callback::new(move |_| open.set(false))
            >
                <div class="grid gap-3 sm:grid-cols-2">
                    <Input label="N° carte" value=card_number.into() on_input=Callback::new(move |v| card_number.set(v)) />
                    <Input label="Cotisation adhésion (€)" r#type="number" value=adhesion_fee.into() on_input=Callback::new(move |v| adhesion_fee.set(v)) />
                    <Input label="Nom" value=last_name.into() on_input=Callback::new(move |v| last_name.set(v)) />
                    <Input label="Prénom" value=first_name.into() on_input=Callback::new(move |v| first_name.set(v)) />
                    <Input label="Adresse" value=address.into() on_input=Callback::new(move |v| address.set(v)) />
                    <Input label="Complément" value=address_complement.into() on_input=Callback::new(move |v| address_complement.set(v)) />
                    <Input label="Code postal" value=postal_code.into() on_input=Callback::new(move |v| postal_code.set(v)) />
                    <Input label="Ville" value=city.into() on_input=Callback::new(move |v| city.set(v)) />
                    <Input label="Téléphone" value=phone.into() on_input=Callback::new(move |v| phone.set(v)) />
                    <Input label="Email" value=email.into() on_input=Callback::new(move |v| email.set(v)) />
                    <Select label="Virement" options=bank_options value=bank.into() on_change=Callback::new(move |v| bank.set(v)) />
                    <Show when=move || editing.get().is_some()>
                        <Select label="Statut" options=status_options value=status.into() on_change=Callback::new(move |v| status.set(v)) />
                    </Show>
                    <div class="sm:col-span-2">
                        <TextArea label="Notes" value=notes.into() on_input=Callback::new(move |v| notes.set(v)) />
                    </div>
                </div>
                <Show when=move || form_error.get().is_some()>
                    <p class="mt-3 text-[var(--brand-red)]">{move || form_error.get().unwrap_or_default()}</p>
                </Show>
                <div class="mt-4 flex justify-end">
                    <Button on_click=Callback::new(move |_| {
                        let fee = adhesion_fee
                            .get_untracked()
                            .replace(',', ".")
                            .parse()
                            .unwrap_or(0.0);
                        spawn_local(async move {
                            let result = if let Some(ed) = editing.get_untracked() {
                                api::update_member(UpdateMemberInput {
                                    id: ed.id,
                                    card_number: card_number.get_untracked(),
                                    last_name: last_name.get_untracked(),
                                    first_name: first_name.get_untracked(),
                                    adhesion_fee: fee,
                                    address: opt(address.get_untracked()),
                                    address_complement: opt(address_complement.get_untracked()),
                                    postal_code: opt(postal_code.get_untracked()),
                                    city: opt(city.get_untracked()),
                                    phone: opt(phone.get_untracked()),
                                    email: opt(email.get_untracked()),
                                    bank_transfer_status: opt(bank.get_untracked()),
                                    status: status.get_untracked(),
                                    notes: opt(notes.get_untracked()),
                                })
                                .await
                                .map(|_| ())
                            } else {
                                api::create_member(CreateMemberInput {
                                    card_number: card_number.get_untracked(),
                                    last_name: last_name.get_untracked(),
                                    first_name: first_name.get_untracked(),
                                    adhesion_fee: fee,
                                    address: opt(address.get_untracked()),
                                    address_complement: opt(address_complement.get_untracked()),
                                    postal_code: opt(postal_code.get_untracked()),
                                    city: opt(city.get_untracked()),
                                    phone: opt(phone.get_untracked()),
                                    email: opt(email.get_untracked()),
                                    bank_transfer_status: opt(bank.get_untracked()),
                                    notes: opt(notes.get_untracked()),
                                })
                                .await
                                .map(|_| ())
                            };
                            match result {
                                Ok(()) => {
                                    open.set(false);
                                    reload();
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
