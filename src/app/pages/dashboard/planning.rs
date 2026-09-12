use chrono::Datelike;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::HashMap;

use crate::app::components::ui::{
    Button, ButtonVariant, EditableCell, Input, Select, SelectOption, Table, TableFullscreenToggle,
    TableLoadMode, TablePaginationBar, TBody, Td, Th, THead, Tr, DEFAULT_TABLE_LOAD_MODE,
    default_lazy_count, paginate_slice,
};
use crate::app::hooks::use_table_fullscreen;
use crate::app::i18n::use_i18n;
use crate::app::lib::{api, PlanningPeriod, UpsertPlanningInput};

const MONTH_NAMES: [&str; 12] = [
    "Janvier",
    "Février",
    "Mars",
    "Avril",
    "Mai",
    "Juin",
    "Juillet",
    "Août",
    "Septembre",
    "Octobre",
    "Novembre",
    "Décembre",
];

fn month_options() -> Vec<SelectOption> {
    (1..=12)
        .map(|m| SelectOption {
            value: m.to_string(),
            label: MONTH_NAMES[(m - 1) as usize].to_string(),
        })
        .collect()
}

#[component]
pub fn PlanningPage() -> impl IntoView {
    let i18n = use_i18n();
    let year = RwSignal::new(chrono::Local::now().year());
    let periods = RwSignal::new(Vec::<PlanningPeriod>::new());
    let error = RwSignal::new(Option::<String>::None);
    let view_mode = RwSignal::new("table".to_string()); // table | calendar
    let begin_month = RwSignal::new("1".to_string());
    let end_month = RwSignal::new("12".to_string());
    let search = RwSignal::new(String::new());
    let advanced_open = RwSignal::new(false);
    let form_open = RwSignal::new(false);
    let page = RwSignal::new(0usize);
    let page_size = RwSignal::new(25usize);
    let load_mode = RwSignal::new(DEFAULT_TABLE_LOAD_MODE.to_string());
    let lazy_count = RwSignal::new(25usize);

    // Add / edit form
    let edit_id = RwSignal::new(Option::<String>::None);
    let form_month = RwSignal::new("1".to_string());
    let form_label = RwSignal::new(String::new());
    let form_date = RwSignal::new(String::new());
    let form_start = RwSignal::new("14:30".to_string());
    let form_end = RwSignal::new("15:30".to_string());
    let form_color = RwSignal::new("#007A3E".to_string());
    let selected = RwSignal::new(Vec::<String>::new());
    let pending = RwSignal::new(HashMap::<String, String>::new());

    let reload = move || {
        let y = year.get_untracked();
        spawn_local(async move {
            let _ = api::ensure_year(y).await;
            match api::list_planning(y).await {
                Ok(list) => {
                    periods.set(list);
                    pending.set(HashMap::new());
                    error.set(None);
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    Effect::new(move |_| {
        let _ = year.get();
        reload();
    });

    let filtered = Signal::derive(move || {
        let begin: i32 = begin_month.get().parse().unwrap_or(1);
        let end: i32 = end_month.get().parse().unwrap_or(12);
        let (lo, hi) = if begin <= end {
            (begin, end)
        } else {
            (end, begin)
        };
        let q = search.get().trim().to_lowercase();
        let mut rows = periods
            .get()
            .into_iter()
            .filter(|p| p.period_month >= lo && p.period_month <= hi)
            .filter(|p| {
                if q.is_empty() {
                    return true;
                }
                let month_name = MONTH_NAMES
                    .get((p.period_month as usize).saturating_sub(1))
                    .copied()
                    .unwrap_or("")
                    .to_lowercase();
                p.label.to_lowercase().contains(&q)
                    || month_name.contains(&q)
                    || p.meeting_date
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&q)
            })
            .collect::<Vec<_>>();
        rows.sort_by_key(|p| p.period_month);
        rows
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

    let month_opts = Signal::derive(month_options);
    let fs = use_table_fullscreen();

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
            <Show when=move || !fs.active.get()>
                <div class="flex shrink-0 flex-wrap items-end justify-between gap-3">
                    <div>
                        <h1 class="font-display text-3xl font-semibold">{move || i18n.t("planning.title")}</h1>
                        <p class="text-[var(--muted)]">{move || i18n.t("planning.subtitle")}</p>
                    </div>
                    <div class="flex flex-wrap gap-2">
                        <Button on_click=Callback::new(move |_| {
                            edit_id.set(None);
                            form_open.update(|v| *v = !*v);
                        })>
                            {move || i18n.t("planning.add")}
                        </Button>
                        <Show when=move || !selected.get().is_empty()>
                            <Button
                                variant=ButtonVariant::Danger
                                on_click=Callback::new(move |_| {
                                    let ids = selected.get_untracked();
                                    spawn_local(async move {
                                        for id in ids {
                                            let _ = api::delete_planning_period(&id, true).await;
                                        }
                                        selected.set(Vec::new());
                                        reload();
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
                <label class="flex flex-col gap-1 text-sm font-medium">
                    {move || i18n.t("common.year")}
                    <input
                        class="tap-target w-28 rounded-xl border border-[var(--border)] bg-[var(--bg-elevated)] px-3"
                        type="number"
                        prop:value=move || year.get()
                        on:change=move |ev| {
                            if let Ok(y) = event_target_value(&ev).parse::<i32>() {
                                year.set(y);
                                page.set(0);
                                lazy_count.set(default_lazy_count(page_size.get_untracked()));
                            }
                        }
                    />
                </label>
                <Input
                    label="Recherche"
                    placeholder="Mois, libellé, date…"
                    value=search.into()
                    on_input=Callback::new(move |v| {
                        search.set(v);
                        page.set(0);
                        lazy_count.set(default_lazy_count(page_size.get_untracked()));
                    })
                    class="min-w-[14rem] flex-1"
                />
                <button
                    type="button"
                    class=move || {
                        if advanced_open.get() {
                            "tap-target rounded-xl bg-[color-mix(in_srgb,var(--brand)_16%,transparent)] px-4 py-2 text-sm font-semibold"
                        } else {
                            "tap-target rounded-xl border border-[var(--border)] px-4 py-2 text-sm font-semibold"
                        }
                    }
                    on:click=move |_| advanced_open.update(|v| *v = !*v)
                >
                    {move || {
                        if advanced_open.get() {
                            "▾ Avancé"
                        } else {
                            "▸ Avancé"
                        }
                    }}
                </button>
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
                        {move || i18n.t("planning.view_table")}
                    </button>
                    <button
                        type="button"
                        class=move || {
                            if view_mode.get() == "calendar" {
                                "rounded-lg bg-[var(--brand)] px-3 py-1.5 text-xs font-semibold text-white"
                            } else {
                                "rounded-lg px-3 py-1.5 text-xs font-semibold"
                            }
                        }
                        on:click=move |_| view_mode.set("calendar".into())
                    >
                        {move || i18n.t("planning.view_calendar")}
                    </button>
                </div>
            </div>

            <Show when=move || advanced_open.get()>
                <div class="flex shrink-0 flex-wrap items-end gap-3 rounded-2xl border border-[var(--border)] bg-[var(--surface)] p-4 shadow-[var(--shadow)]">
                    <Select
                        label="Du mois"
                        options=month_opts
                        value=begin_month.into()
                        on_change=Callback::new(move |v| {
                            begin_month.set(v);
                            page.set(0);
                            lazy_count.set(default_lazy_count(page_size.get_untracked()));
                        })
                        class="w-40"
                    />
                    <Select
                        label="Au mois"
                        options=Signal::derive(month_options)
                        value=end_month.into()
                        on_change=Callback::new(move |v| {
                            end_month.set(v);
                            page.set(0);
                            lazy_count.set(default_lazy_count(page_size.get_untracked()));
                        })
                        class="w-40"
                    />
                </div>
            </Show>

            <Show when=move || error.get().is_some()>
                <p class="shrink-0 text-[var(--brand-red)]">{move || error.get().unwrap_or_default()}</p>
            </Show>

            <div class="flex shrink-0 items-center justify-between gap-3">
                <p class="text-sm text-[var(--muted)]">
                    {move || format!("{} période(s)", filtered_total.get())}
                </p>
                <Show when=move || view_mode.get() == "calendar">
                    <TableFullscreenToggle />
                </Show>
            </div>

            <Show when=move || form_open.get() || edit_id.get().is_some()>
                <div class="grid shrink-0 gap-3 rounded-2xl border border-[var(--border)] bg-[var(--surface)] p-4 sm:grid-cols-2 lg:grid-cols-3">
                    <Select
                        label="Mois"
                        options=Signal::derive(month_options)
                        value=form_month.into()
                        on_change=Callback::new(move |v| form_month.set(v))
                        class="w-full"
                    />
                    <Input
                        label="Libellé"
                        value=form_label.into()
                        on_input=Callback::new(move |v| form_label.set(v))
                        placeholder="Janvier"
                    />
                    <Input
                        label="Date de réunion"
                        r#type="date"
                        value=form_date.into()
                        on_input=Callback::new(move |v| form_date.set(v))
                    />
                    <Input
                        label="Collecte début"
                        value=form_start.into()
                        on_input=Callback::new(move |v| form_start.set(v))
                        placeholder="14:30"
                    />
                    <Input
                        label="Collecte fin"
                        value=form_end.into()
                        on_input=Callback::new(move |v| form_end.set(v))
                        placeholder="15:30"
                    />
                    <label class="flex flex-col gap-1 text-sm font-medium">
                        "Couleur du mois"
                        <div class="flex items-center gap-2">
                            <input
                                type="color"
                                class="h-10 w-14 cursor-pointer rounded-lg border border-[var(--border)] bg-[var(--bg-elevated)]"
                                prop:value=move || form_color.get()
                                on:input=move |ev| form_color.set(event_target_value(&ev))
                            />
                            <input
                                type="text"
                                class="tap-target min-w-0 flex-1 rounded-xl border border-[var(--border)] bg-[var(--bg-elevated)] px-3"
                                prop:value=move || form_color.get()
                                on:change=move |ev| form_color.set(event_target_value(&ev))
                                placeholder="#007A3E"
                            />
                        </div>
                    </label>
                    <div class="flex items-end gap-2">
                        <Button on_click=Callback::new(move |_| {
                            let y = year.get_untracked();
                            let month: i32 = form_month.get_untracked().parse().unwrap_or(1);
                            let label = form_label.get_untracked();
                            let date = form_date.get_untracked();
                            let start = form_start.get_untracked();
                            let end = form_end.get_untracked();
                            let color = form_color.get_untracked();
                            let id = edit_id.get_untracked();
                            spawn_local(async move {
                                match api::upsert_planning_period(UpsertPlanningInput {
                                    id,
                                    year: y,
                                    period_month: month,
                                    label: if label.is_empty() { None } else { Some(label) },
                                    meeting_date: if date.is_empty() { None } else { Some(date) },
                                    collect_start: Some(start),
                                    collect_end: Some(end),
                                    sort_order: Some(month),
                                    label_color: if color.trim().is_empty() {
                                        None
                                    } else {
                                        Some(color)
                                    },
                                })
                                .await
                                {
                                    Ok(_) => {
                                        edit_id.set(None);
                                        form_open.set(false);
                                        form_label.set(String::new());
                                        form_date.set(String::new());
                                        form_color.set("#007A3E".into());
                                        reload();
                                    }
                                    Err(e) => error.set(Some(e)),
                                }
                            });
                        })>
                            {move || {
                                if edit_id.get().is_some() {
                                    i18n.t("common.save")
                                } else {
                                    i18n.t("planning.add")
                                }
                            }}
                        </Button>
                        <Button
                            variant=ButtonVariant::Secondary
                            on_click=Callback::new(move |_| {
                                edit_id.set(None);
                                form_open.set(false);
                                form_label.set(String::new());
                                form_date.set(String::new());
                                form_color.set("#007A3E".into());
                            })
                        >
                            {move || i18n.t("common.cancel")}
                        </Button>
                    </div>
                </div>
            </Show>

            <Show when=move || view_mode.get() == "table">
                <div class="flex min-h-0 flex-1 flex-col gap-4">
                    <Show when=move || !pending.get().is_empty()>
                        <div class="sticky top-0 z-20 flex shrink-0 flex-wrap items-center justify-between gap-3 rounded-xl border border-[var(--brand)] bg-[var(--surface)] px-4 py-3 shadow-[var(--shadow)]">
                            <p class="text-sm font-medium">
                                {move || {
                                    format!("{} modification(s) en attente", pending.get().len())
                                }}
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
                                    let list = periods.get_untracked();
                                    let y = year.get_untracked();
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
                                            let Some(p) = list.iter().find(|x| x.id == id) else {
                                                continue;
                                            };
                                            let label = fields
                                                .get("label")
                                                .cloned()
                                                .unwrap_or_else(|| p.label.clone());
                                            let meeting_date = fields
                                                .get("meeting_date")
                                                .cloned()
                                                .or_else(|| p.meeting_date.clone());
                                            let collect_start = fields
                                                .get("collect_start")
                                                .cloned()
                                                .or_else(|| p.collect_start.clone());
                                            let collect_end = fields
                                                .get("collect_end")
                                                .cloned()
                                                .or_else(|| p.collect_end.clone());
                                            let input = UpsertPlanningInput {
                                                id: Some(p.id.clone()),
                                                year: y,
                                                period_month: p.period_month,
                                                label: Some(label),
                                                meeting_date: meeting_date
                                                    .filter(|s| !s.trim().is_empty()),
                                                collect_start: collect_start
                                                    .filter(|s| !s.trim().is_empty()),
                                                collect_end: collect_end
                                                    .filter(|s| !s.trim().is_empty()),
                                                sort_order: Some(p.period_month),
                                                label_color: p.label_color.clone(),
                                            };
                                            if let Err(e) = api::upsert_planning_period(input).await
                                            {
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

                    <div class="flex min-h-0 flex-1 flex-col overflow-hidden">
                        <Table
                            class="text-sm"
                            fullscreen_toggle=true
                            lazy_mode=load_mode
                            lazy_count=lazy_count
                            lazy_total=filtered_total
                            lazy_page_size=page_size
                        >
                            <THead>
                                <Th>
                                    <input
                                        type="checkbox"
                                        class="h-4 w-4"
                                        prop:checked=move || {
                                            let list = visible.get();
                                            let sel = selected.get();
                                            !list.is_empty()
                                                && list.iter().all(|p| sel.iter().any(|id| id == &p.id))
                                        }
                                        on:change=move |_| {
                                            let list = visible.get_untracked();
                                            let all = list.iter().all(|p| {
                                                selected.get_untracked().iter().any(|id| id == &p.id)
                                            });
                                            if all {
                                                selected.update(|sel| {
                                                    for p in &list {
                                                        if let Some(i) = sel.iter().position(|id| id == &p.id) {
                                                            sel.remove(i);
                                                        }
                                                    }
                                                });
                                            } else {
                                                selected.update(|sel| {
                                                    for p in list {
                                                        if !sel.iter().any(|id| id == &p.id) {
                                                            sel.push(p.id);
                                                        }
                                                    }
                                                });
                                            }
                                        }
                                    />
                                </Th>
                                <Th>"Mois"</Th>
                                <Th>"Couleur"</Th>
                                <Th>"Libellé"</Th>
                                <Th>"Date AG"</Th>
                                <Th>"Début"</Th>
                                <Th>"Fin"</Th>
                                <Th>{move || i18n.t("common.actions")}</Th>
                            </THead>
                            <TBody>
                                <For
                                    each=move || visible.get()
                                    key=|p| p.id.clone()
                                    children=move |p| {
                                        let pid = p.id.clone();
                                        let pid_check = p.id.clone();
                                        let pid_sel = p.id.clone();
                                        let pid_edit = p.id.clone();
                                        let p_edit = p.clone();
                                        let id_label = p.id.clone();
                                        let id_date = p.id.clone();
                                        let id_start = p.id.clone();
                                        let id_end = p.id.clone();
                                        let label_base = p.label.clone();
                                        let date_base =
                                            p.meeting_date.clone().unwrap_or_default();
                                        let start_base = p
                                            .collect_start
                                            .clone()
                                            .unwrap_or_else(|| "14:30".into());
                                        let end_base = p
                                            .collect_end
                                            .clone()
                                            .unwrap_or_else(|| "15:30".into());
                                        view! {
                                            <Tr>
                                                <Td>
                                                    <input
                                                        type="checkbox"
                                                        class="h-4 w-4"
                                                        prop:checked=move || {
                                                            selected.get().iter().any(|id| id == &pid_check)
                                                        }
                                                        on:change=move |_| {
                                                            selected.update(|sel| {
                                                                if let Some(i) =
                                                                    sel.iter().position(|x| x == &pid_sel)
                                                                {
                                                                    sel.remove(i);
                                                                } else {
                                                                    sel.push(pid_sel.clone());
                                                                }
                                                            });
                                                        }
                                                    />
                                                </Td>
                                                <Td>{MONTH_NAMES.get((p.period_month as usize).saturating_sub(1)).copied().unwrap_or("?")}</Td>
                                                <Td>
                                                    <span
                                                        class="inline-block h-6 w-6 rounded-md border border-[var(--border)]"
                                                        style=format!(
                                                            "background-color:{}",
                                                            p.label_color.clone().unwrap_or_else(|| "#007A3E".into())
                                                        )
                                                        title=p.label_color.clone().unwrap_or_default()
                                                    ></span>
                                                </Td>
                                                <Td>
                                                    <EditableCell
                                                        value=cell_value(id_label.clone(), "label", label_base)
                                                        on_commit=Callback::new(move |v| {
                                                            queue_edit(id_label.clone(), "label", v)
                                                        })
                                                    />
                                                </Td>
                                                <Td>
                                                    <EditableCell
                                                        value=cell_value(id_date.clone(), "meeting_date", date_base)
                                                        on_commit=Callback::new(move |v| {
                                                            queue_edit(id_date.clone(), "meeting_date", v)
                                                        })
                                                        placeholder="AAAA-MM-JJ"
                                                    />
                                                </Td>
                                                <Td>
                                                    <EditableCell
                                                        value=cell_value(id_start.clone(), "collect_start", start_base)
                                                        on_commit=Callback::new(move |v| {
                                                            queue_edit(id_start.clone(), "collect_start", v)
                                                        })
                                                    />
                                                </Td>
                                                <Td>
                                                    <EditableCell
                                                        value=cell_value(id_end.clone(), "collect_end", end_base)
                                                        on_commit=Callback::new(move |v| {
                                                            queue_edit(id_end.clone(), "collect_end", v)
                                                        })
                                                    />
                                                </Td>
                                                <Td>
                                                    <div class="flex flex-wrap gap-2">
                                                        <Button
                                                            variant=ButtonVariant::Secondary
                                                            on_click=Callback::new(move |_| {
                                                                edit_id.set(Some(pid_edit.clone()));
                                                                form_open.set(true);
                                                                form_month.set(p_edit.period_month.to_string());
                                                                form_label.set(p_edit.label.clone());
                                                                form_date.set(
                                                                    p_edit.meeting_date.clone().unwrap_or_default(),
                                                                );
                                                                form_start.set(
                                                                    p_edit
                                                                        .collect_start
                                                                        .clone()
                                                                        .unwrap_or_else(|| "14:30".into()),
                                                                );
                                                                form_end.set(
                                                                    p_edit
                                                                        .collect_end
                                                                        .clone()
                                                                        .unwrap_or_else(|| "15:30".into()),
                                                                );
                                                                form_color.set(
                                                                    p_edit
                                                                        .label_color
                                                                        .clone()
                                                                        .unwrap_or_else(|| "#007A3E".into()),
                                                                );
                                                            })
                                                        >
                                                            {move || i18n.t("common.edit")}
                                                        </Button>
                                                        <Button
                                                            variant=ButtonVariant::Danger
                                                            on_click=Callback::new(move |_| {
                                                                let id = pid.clone();
                                                                spawn_local(async move {
                                                                    match api::delete_planning_period(&id, false).await {
                                                                        Ok(()) => reload(),
                                                                        Err(e) => {
                                                                            if e.contains("force") || e.contains("paiement") {
                                                                                if let Ok(()) =
                                                                                    api::delete_planning_period(&id, true).await
                                                                                {
                                                                                    reload();
                                                                                } else {
                                                                                    error.set(Some(e));
                                                                                }
                                                                            } else {
                                                                                error.set(Some(e));
                                                                            }
                                                                        }
                                                                    }
                                                                });
                                                            })
                                                        >
                                                            {move || i18n.t("common.delete")}
                                                        </Button>
                                                    </div>
                                                </Td>
                                            </Tr>
                                        }
                                    }
                                />
                            </TBody>
                        </Table>
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
                </div>
            </Show>

            <Show when=move || view_mode.get() == "calendar">
                <div class="min-h-0 flex-1 overflow-auto">
                    <PlanningCalendar year=year.into() periods=filtered />
                </div>
            </Show>
        </div>
    }
}

#[component]
fn PlanningCalendar(year: Signal<i32>, periods: Signal<Vec<PlanningPeriod>>) -> impl IntoView {
    let meetings = Signal::derive(move || {
        periods
            .get()
            .into_iter()
            .filter_map(|p| {
                let date = p.meeting_date?;
                // YYYY-MM-DD
                let parts: Vec<&str> = date.split('-').collect();
                if parts.len() != 3 {
                    return None;
                }
                let m: u32 = parts[1].parse().ok()?;
                let d: u32 = parts[2].parse().ok()?;
                Some((m, d, p.label))
            })
            .collect::<Vec<_>>()
    });

    view! {
        <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {(1u32..=12).map(|month| {
                view! {
                    <div class="rounded-2xl border border-[var(--border)] bg-[var(--surface)] p-3 shadow-[var(--shadow)]">
                        <p class="mb-2 font-display text-lg font-semibold">
                            {MONTH_NAMES[(month as usize) - 1]} " " {move || year.get()}
                        </p>
                        <div class="grid grid-cols-7 gap-1 text-center text-xs text-[var(--muted)]">
                            {"L M M J V S D".split_whitespace().map(|d| view! { <span>{d}</span> }).collect_view()}
                        </div>
                        {move || {
                            let y = year.get();
                            let days_in_month = days_in_month(y, month as i32);
                            // weekday of 1st (Mon=0)
                            let first_wd = weekday_monday0(y, month as i32, 1);
                            let meet = meetings.get();
                            let mut cells = Vec::new();
                            for _ in 0..first_wd {
                                cells.push(view! { <span class="h-8"></span> }.into_any());
                            }
                            for day in 1..=days_in_month {
                                let is_meet = meet.iter().any(|(m, d, _)| *m == month && *d == day as u32);
                                let label = meet
                                    .iter()
                                    .find(|(m, d, _)| *m == month && *d == day as u32)
                                    .map(|(_, _, l)| l.clone());
                                let cls = if is_meet {
                                    "flex h-8 items-center justify-center rounded-lg bg-[var(--brand)] text-xs font-bold text-white"
                                } else {
                                    "flex h-8 items-center justify-center text-xs"
                                };
                                let title = label.unwrap_or_default();
                                cells.push(view! {
                                    <span class=cls title=title>{day}</span>
                                }.into_any());
                            }
                            view! { <div class="mt-1 grid grid-cols-7 gap-1">{cells.collect_view()}</div> }
                        }}
                    </div>
                }
            }).collect_view()}
        </div>
    }
}

fn days_in_month(year: i32, month: i32) -> i32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

fn weekday_monday0(year: i32, month: i32, day: i32) -> i32 {
    // Sakamoto — returns 0=Sun … convert to Mon=0
    let t = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let mut y = year;
    if month < 3 {
        y -= 1;
    }
    let sun0 = (y + y / 4 - y / 100 + y / 400 + t[(month as usize) - 1] + day) % 7;
    (sun0 + 6) % 7 // Mon=0
}
