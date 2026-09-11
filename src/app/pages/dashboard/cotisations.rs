use chrono::Datelike;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::HashMap;

use crate::app::components::cotisations::{ExcelImportModal, PaymentModal, ReceiptDialog};
use crate::app::components::ui::{
    Button, ButtonVariant, EditableCell, Input, Modal, RowDensitySlider, Select, SelectOption,
    Table, TableFullscreenToggle, TableLoadMode, TablePaginationBar, TBody, Td, Th, THead, Tr,
    paginate_slice,
};
use crate::app::hooks::use_table_fullscreen;
use crate::app::i18n::use_i18n;
use crate::app::lib::{
    api, build_payment_receipt, debt_text_class, format_stored_money, is_intuitive_sign, paid_text_class,
    AppSettings, Member, PaymentReceipt, YearGrid, YearGridRow,
};

fn money(v: f64) -> String {
    format!("{v:.2}")
}

fn parse_money_opt(s: &str) -> Option<f64> {
    let t = s.trim().replace(',', ".").replace('—', "");
    if t.is_empty() {
        None
    } else {
        t.parse().ok()
    }
}

fn period_paid(c: &crate::app::lib::PeriodCell) -> bool {
    c.amount_paid.map(|p| p.abs() >= 0.001).unwrap_or(false)
}

fn period_unpaid(c: &crate::app::lib::PeriodCell) -> bool {
    !period_paid(c)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SortKey {
    Carte,
    Nom,
    Prenom,
    Prior,
    Total,
    Balance,
    PeriodDue(usize),
    PeriodPaid(usize),
}

fn sort_arrow(active: bool, asc: bool) -> &'static str {
    if !active {
        return ""
    }
    if asc {
        " ↑"
    } else {
        " ↓"
    }
}

#[component]
pub fn CotisationsPage() -> impl IntoView {
    let i18n = use_i18n();
    let year = RwSignal::new(chrono::Local::now().year());
    let grid = RwSignal::new(Option::<YearGrid>::None);
    let monthly = RwSignal::new(String::new());
    let error = RwSignal::new(Option::<String>::None);
    let pay_open = RwSignal::new(false);
    let import_open = RwSignal::new(false);
    let receipt = RwSignal::new(Option::<PaymentReceipt>::None);
    let org_name = RwSignal::new("NDIMBELENTÉ".into());
    let app_settings = RwSignal::new(AppSettings::default());
    let debt_sign = RwSignal::new("intuitive".to_string());

    let search = RwSignal::new(String::new());
    let payment_filter = RwSignal::new(String::new());
    let period_filter = RwSignal::new(String::new());
    let current_period = RwSignal::new(String::new()); // period id

    let sort_key = RwSignal::new(SortKey::Nom);
    let sort_asc = RwSignal::new(true);

    let view_open = RwSignal::new(false);
    let view_row = RwSignal::new(Option::<YearGridRow>::None);
    let prefill_member = RwSignal::new(String::new());

    let selected = RwSignal::new(Vec::<String>::new());
    let row_density = RwSignal::new(0.35_f64);
    let page = RwSignal::new(0usize);
    let page_size = RwSignal::new(25usize);
    let load_mode = RwSignal::new("page".to_string());
    let lazy_count = RwSignal::new(25usize);
    let advanced_open = RwSignal::new(false);
    let fs = use_table_fullscreen();
    let view_mode = RwSignal::new("table".to_string()); // table | grid
    // key = "{member_id}:{period_id}:{field}" → (member_id, period_id, field, value_str)
    let pending_cells =
        RwSignal::new(HashMap::<String, (String, String, String, String)>::new());

    let reload = move || {
        let y = year.get_untracked();
        spawn_local(async move {
            let _ = api::ensure_year(y).await;
            match api::get_year_grid(y).await {
                Ok(g) => {
                    monthly.set(format!("{}", g.year.monthly_amount));
                    if current_period.get_untracked().is_empty()
                        || !g.periods.iter().any(|p| p.id == current_period.get_untracked())
                    {
                        let now_m = chrono::Local::now().month() as i32;
                        let pick = g
                            .periods
                            .iter()
                            .filter(|p| p.period_month <= now_m)
                            .max_by_key(|p| p.period_month)
                            .or_else(|| g.periods.first())
                            .map(|p| p.id.clone())
                            .unwrap_or_default();
                        current_period.set(pick);
                    }
                    pending_cells.set(HashMap::new());
                    grid.set(Some(g));
                    error.set(None);
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    Effect::new(move |_| {
        let _ = year.get();
        current_period.set(String::new());
        reload();
    });

    Effect::new(move |_| {
        spawn_local(async move {
            if let Ok(s) = api::get_settings().await {
                org_name.set(s.org_name.clone());
                debt_sign.set(s.debt_display_sign.clone());
                app_settings.set(s);
            }
        });
    });

    let intuitive = Signal::derive(move || is_intuitive_sign(&debt_sign.get()));

    let toggle_sort = move |key: SortKey| {
        if sort_key.get_untracked() == key {
            sort_asc.update(|v| *v = !*v);
        } else {
            sort_key.set(key);
            sort_asc.set(true);
        }
    };

    let members_sig = Signal::derive(move || {
        grid.get()
            .map(|g| g.rows.into_iter().map(|r| r.member).collect::<Vec<Member>>())
            .unwrap_or_default()
    });

    let current_period_label = Signal::derive(move || {
        let id = current_period.get();
        grid.get()
            .and_then(|g| g.periods.into_iter().find(|p| p.id == id).map(|p| p.label))
            .unwrap_or_else(|| "—".into())
    });

    let current_period_month = Signal::derive(move || {
        let id = current_period.get();
        grid.get()
            .and_then(|g| {
                g.periods
                    .into_iter()
                    .find(|p| p.id == id)
                    .map(|p| p.period_month)
            })
            .unwrap_or_else(|| chrono::Local::now().month() as i32)
    });

    let period_filter_options = Signal::derive(move || {
        let mut opts = vec![SelectOption {
            value: String::new(),
            label: i18n.t("cotisations.period_all"),
        }];
        if let Some(g) = grid.get() {
            for p in g.periods {
                opts.push(SelectOption {
                    value: p.id,
                    label: p.label,
                });
            }
        }
        opts
    });

    let current_period_options = Signal::derive(move || {
        grid.get()
            .map(|g| {
                g.periods
                    .into_iter()
                    .map(|p| SelectOption {
                        value: p.id,
                        label: format!("{} ({})", p.label, p.period_month),
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    });

    let payment_options = Signal::derive(move || {
        vec![
            SelectOption {
                value: String::new(),
                label: i18n.t("cotisations.pay_all"),
            },
            SelectOption {
                value: "paid".into(),
                label: i18n.t("cotisations.pay_paid"),
            },
            SelectOption {
                value: "unpaid".into(),
                label: i18n.t("cotisations.pay_unpaid"),
            },
            SelectOption {
                value: "debt".into(),
                label: i18n.t("cotisations.pay_debt"),
            },
        ]
    });

    let filtered_rows = Signal::derive(move || {
        let q = search.get().trim().to_lowercase();
        let pf = payment_filter.get();
        let period_id = period_filter.get();
        let key = sort_key.get();
        let asc = sort_asc.get();

        let mut rows = grid
            .get()
            .map(|g| g.rows)
            .unwrap_or_default()
            .into_iter()
            .filter(|row| {
                let name = format!("{} {}", row.member.last_name, row.member.first_name);
                let matches_q = q.is_empty()
                    || row.member.card_number.to_lowercase().contains(&q)
                    || name.to_lowercase().contains(&q);
                if !matches_q {
                    return false;
                }
                match pf.as_str() {
                    "debt" => {
                        if period_id.is_empty() {
                            row.balance > 0.001
                        } else {
                            row.periods.iter().any(|c| {
                                c.period_id == period_id && period_unpaid(c) && c.amount_due > 0.001
                            })
                        }
                    }
                    "paid" => {
                        if period_id.is_empty() {
                            !row.periods.is_empty() && row.periods.iter().all(period_paid)
                        } else {
                            row.periods
                                .iter()
                                .any(|c| c.period_id == period_id && period_paid(c))
                        }
                    }
                    "unpaid" => {
                        if period_id.is_empty() {
                            row.periods.iter().any(period_unpaid)
                        } else {
                            row.periods
                                .iter()
                                .any(|c| c.period_id == period_id && period_unpaid(c))
                        }
                    }
                    _ => {
                        // Period-only filter: keep members that have an entry for that period
                        // (always true) — still apply when combined with search above.
                        // When a period is chosen without payment status, show rows with
                        // activity focus: any due/paid cell for that period exists.
                        if period_id.is_empty() {
                            true
                        } else {
                            row.periods.iter().any(|c| c.period_id == period_id)
                        }
                    }
                }
            })
            .collect::<Vec<_>>();

        rows.sort_by(|a, b| {
            let ord = match key {
                SortKey::Carte => a.member.card_number.cmp(&b.member.card_number),
                SortKey::Nom => a
                    .member
                    .last_name
                    .to_lowercase()
                    .cmp(&b.member.last_name.to_lowercase()),
                SortKey::Prenom => a
                    .member
                    .first_name
                    .to_lowercase()
                    .cmp(&b.member.first_name.to_lowercase()),
                SortKey::Prior => a
                    .prior_december_debt
                    .partial_cmp(&b.prior_december_debt)
                    .unwrap_or(std::cmp::Ordering::Equal),
                SortKey::Total => a
                    .total_paid
                    .partial_cmp(&b.total_paid)
                    .unwrap_or(std::cmp::Ordering::Equal),
                SortKey::Balance => a
                    .balance
                    .partial_cmp(&b.balance)
                    .unwrap_or(std::cmp::Ordering::Equal),
                SortKey::PeriodDue(i) => {
                    let av = a.periods.get(i).map(|c| c.amount_due).unwrap_or(0.0);
                    let bv = b.periods.get(i).map(|c| c.amount_due).unwrap_or(0.0);
                    av.partial_cmp(&bv).unwrap_or(std::cmp::Ordering::Equal)
                }
                SortKey::PeriodPaid(i) => {
                    let av = a.periods.get(i).and_then(|c| c.amount_paid).unwrap_or(-1.0);
                    let bv = b.periods.get(i).and_then(|c| c.amount_paid).unwrap_or(-1.0);
                    av.partial_cmp(&bv).unwrap_or(std::cmp::Ordering::Equal)
                }
            };
            if asc {
                ord
            } else {
                ord.reverse()
            }
        });

        rows
    });

    let filtered_total = Signal::derive(move || filtered_rows.get().len());

    let visible_rows = Signal::derive(move || {
        let rows = filtered_rows.get();
        let mode = TableLoadMode::from_str(&load_mode.get());
        paginate_slice(
            &rows,
            mode,
            page.get(),
            page_size.get(),
            lazy_count.get(),
        )
    });

    let toggle_selected = move |id: String| {
        selected.update(|list| {
            if let Some(i) = list.iter().position(|x| x == &id) {
                list.remove(i);
            } else {
                list.push(id);
            }
        });
    };

    let all_visible_selected = Signal::derive(move || {
        let vis = visible_rows.get();
        let sel = selected.get();
        !vis.is_empty() && vis.iter().all(|r| sel.iter().any(|id| id == &r.member.id))
    });

    view! {
        <div class="flex min-h-0 flex-1 flex-col gap-4">
            <Show when=move || !fs.active.get()>
            <div class="flex shrink-0 flex-wrap items-end justify-between gap-3">
                <div>
                    <h1 class="font-display text-3xl font-semibold">{move || i18n.t("cotisations.title")}</h1>
                    <p class="text-[var(--muted)]">{move || i18n.t("cotisations.subtitle")}</p>
                </div>
                <div class="flex flex-wrap gap-2">
                    <Button on_click=Callback::new(move |_| {
                        prefill_member.set(String::new());
                        pay_open.set(true);
                    })>
                        {move || i18n.t("cotisations.new")}
                    </Button>
                    <Button variant=ButtonVariant::Secondary on_click=Callback::new(move |_| import_open.set(true))>
                        {move || i18n.t("common.import")}
                    </Button>
                    <Button
                        variant=ButtonVariant::Secondary
                        on_click=Callback::new(move |_| {
                            let y = year.get_untracked();
                            spawn_local(async move {
                                let default = format!("cotisations-{y}.xlsx");
                                let path = match api::pick_save_file(&default).await {
                                    Some(p) => p,
                                    None => return,
                                };
                                match api::export_excel(&path, y).await {
                                    Ok(res) => {
                                        error.set(None);
                                        web_sys::console::log_1(
                                            &format!("Exporté {} lignes → {}", res.rows_written, res.path).into(),
                                        );
                                    }
                                    Err(e) => error.set(Some(e)),
                                }
                            });
                        })
                    >
                        {move || i18n.t("common.export")}
                    </Button>
                </div>
            </div>
            </Show>

            <div class="flex flex-wrap items-end gap-3">
                <label class="flex flex-col gap-1 text-sm font-medium">
                    {move || i18n.t("common.year")}
                    <input
                        class="tap-target w-28 rounded-xl border border-[var(--border)] bg-[var(--bg-elevated)] px-3"
                        type="number"
                        prop:value=move || year.get()
                        on:change=move |ev| {
                            if let Ok(y) = event_target_value(&ev).parse::<i32>() {
                                year.set(y);
                            }
                        }
                    />
                </label>
                <div class="relative min-w-[14rem] flex-1">
                    <Input
                        label="Recherche"
                        placeholder="N° carte ou nom…"
                        value=search.into()
                        on_input=Callback::new(move |v| {
                            search.set(v);
                            page.set(0);
                        })
                        class="w-full pr-10"
                    />
                    <Show when=move || !search.get().is_empty()>
                        <button
                            type="button"
                            class="absolute bottom-2 right-2 rounded-lg px-2 py-1 text-sm font-semibold text-[var(--muted)] hover:bg-[color-mix(in_srgb,var(--fg)_8%,transparent)] hover:text-[var(--fg)]"
                            title="Effacer"
                            on:click=move |_| {
                                search.set(String::new());
                                page.set(0);
                            }
                        >
                            "✕"
                        </button>
                    </Show>
                </div>
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
            </div>

            <Show when=move || advanced_open.get()>
                <div class="flex flex-col gap-4 rounded-2xl border border-[var(--border)] bg-[var(--surface)] p-4 shadow-[var(--shadow)]">
                    <div class="flex flex-wrap items-end gap-4">
                        <Select
                            label="Mois actuel"
                            options=current_period_options
                            value=current_period.into()
                            on_change=Callback::new(move |v| current_period.set(v))
                            class="w-52"
                        />
                        <div class="flex items-end gap-2">
                            <Input
                                label="Montant mensuel (€)"
                                r#type="number"
                                value=monthly.into()
                                on_input=Callback::new(move |v| monthly.set(v))
                                class="w-40"
                            />
                            <Button
                                variant=ButtonVariant::Secondary
                                on_click=Callback::new(move |_| {
                                    let y = year.get_untracked();
                                    let amt = monthly
                                        .get_untracked()
                                        .replace(',', ".")
                                        .parse()
                                        .unwrap_or(0.0);
                                    spawn_local(async move {
                                        match api::set_monthly_amount(y, amt).await {
                                            Ok(_) => reload(),
                                            Err(e) => error.set(Some(e)),
                                        }
                                    });
                                })
                            >
                                {move || i18n.t("common.apply")}
                            </Button>
                        </div>
                    </div>
                    <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
                        <Select
                            label="Paiement"
                            options=payment_options
                            value=payment_filter.into()
                            on_change=Callback::new(move |v| {
                                payment_filter.set(v);
                                page.set(0);
                            })
                        />
                        <Select
                            label="Période"
                            options=period_filter_options
                            value=period_filter.into()
                            on_change=Callback::new(move |v| {
                                period_filter.set(v);
                                page.set(0);
                            })
                        />
                        <RowDensitySlider value=row_density />
                        <Select
                            label="Signe dette / surplus"
                            options=Signal::derive(|| {
                                vec![
                                    SelectOption {
                                        value: "intuitive".into(),
                                        label: "Intuitif (− dette, + surplus)".into(),
                                    },
                                    SelectOption {
                                        value: "excel".into(),
                                        label: "Excel (+ dette, − surplus)".into(),
                                    },
                                ]
                            })
                            value=debt_sign.into()
                            on_change=Callback::new(move |v: String| {
                                debt_sign.set(v.clone());
                                spawn_local(async move {
                                    let mut s = app_settings.get_untracked();
                                    s.debt_display_sign = v;
                                    match api::update_settings(&s).await {
                                        Ok(updated) => app_settings.set(updated),
                                        Err(e) => error.set(Some(e)),
                                    }
                                });
                            })
                        />
                    </div>
                </div>
            </Show>

            <div class="flex shrink-0 flex-wrap items-center justify-between gap-3">
                <p class="text-sm text-[var(--muted)]">
                    {move || format!("{} membre(s)", filtered_total.get())}
                </p>
                <div class="flex flex-wrap items-center gap-2">
                    <Show when=move || !selected.get().is_empty()>
                        <Button
                            variant=ButtonVariant::Danger
                            on_click=Callback::new(move |_| {
                                let ids = selected.get_untracked();
                                if ids.is_empty() {
                                    return;
                                }
                                spawn_local(async move {
                                    match api::delete_members(ids).await {
                                        Ok(n) => {
                                            selected.set(Vec::new());
                                            error.set(None);
                                            web_sys::console::log_1(
                                                &format!("Supprimé {n} membre(s)").into(),
                                            );
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
                    <Show when=move || view_mode.get() == "grid">
                        <TableFullscreenToggle />
                    </Show>
                </div>
            </div>

            <Show when=move || error.get().is_some()>
                <p class="shrink-0 text-[var(--brand-red)]">{move || error.get().unwrap_or_default()}</p>
            </Show>

            <Show when=move || !pending_cells.get().is_empty()>
                <div class="sticky top-0 z-20 flex shrink-0 flex-wrap items-center justify-between gap-3 rounded-xl border border-[var(--brand)] bg-[var(--surface)] px-4 py-3 shadow-[var(--shadow)]">
                    <p class="text-sm font-medium">
                        {move || {
                            format!(
                                "{} modification(s) en attente",
                                pending_cells.get().len()
                            )
                        }}
                    </p>
                    <div class="flex flex-wrap gap-2">
                        <Button
                            variant=ButtonVariant::Secondary
                            on_click=Callback::new(move |_| {
                                pending_cells.set(HashMap::new());
                            })
                        >
                            "Annuler"
                        </Button>
                        <Button on_click=Callback::new(move |_| {
                            let entries: Vec<_> =
                                pending_cells.get_untracked().into_values().collect();
                            if entries.is_empty() {
                                return;
                            }
                            let y = year.get_untracked();
                            let intuit = intuitive.get_untracked();
                            spawn_local(async move {
                                for (mid, pid, field, value_str) in entries {
                                    let parsed = parse_money_opt(&value_str);
                                    if field == "prior" {
                                        let stored = match parsed {
                                            Some(v) if intuit => -v,
                                            Some(v) => v,
                                            None => 0.0,
                                        };
                                        if let Err(e) =
                                            api::set_prior_december_debt(&mid, y, stored).await
                                        {
                                            error.set(Some(e));
                                            return;
                                        }
                                        continue;
                                    }
                                    let value = if field == "due" {
                                        parsed.map(|v| if intuit { -v } else { v })
                                    } else {
                                        parsed
                                    };
                                    if let Err(e) =
                                        api::set_period_cell(&mid, &pid, &field, value).await
                                    {
                                        error.set(Some(e));
                                        return;
                                    }
                                }
                                pending_cells.set(HashMap::new());
                                reload();
                            });
                        })>
                            {move || format!("Mettre à jour ({})", pending_cells.get().len())}
                        </Button>
                    </div>
                </div>
            </Show>

            <div class="flex min-h-0 flex-1 flex-col gap-3 overflow-hidden">
            <Show when=move || view_mode.get() == "grid">
                <div class="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
                    <For
                        each=move || visible_rows.get()
                        key=|r| r.member.id.clone()
                        children=move |row: YearGridRow| {
                            let last_pay = row
                                .periods
                                .iter()
                                .rev()
                                .find(|c| c.amount_paid.map(|p| p > 0.001).unwrap_or(false));
                            let last_label = last_pay
                                .map(|c| {
                                    format!(
                                        "{} · {:.2} €",
                                        c.label,
                                        c.amount_paid.unwrap_or(0.0)
                                    )
                                })
                                .unwrap_or_else(|| "Aucun paiement".into());
                            let name = format!("{} {}", row.member.last_name, row.member.first_name);
                            let card = row.member.card_number.clone();
                            let total = row.total_paid;
                            let balance = row.balance;
                            let mid = row.member.id.clone();
                            let row_view = row.clone();
                            let row_edit = row.clone();
                            let row_clear = row.clone();
                            let row_receipt = row.clone();
                            view! {
                                <div class="rounded-2xl border border-[var(--border)] bg-[var(--surface)] p-4 shadow-[var(--shadow)] transition hover:border-[var(--brand)]">
                                    <div class="flex items-start justify-between gap-2">
                                        <div>
                                            <p class="font-display text-lg font-semibold text-[var(--brand)]">{name}</p>
                                            <p class="text-xs text-[var(--muted)]">{card.clone()}</p>
                                        </div>
                                        <span class="rounded-lg bg-[color-mix(in_srgb,var(--fg)_6%,transparent)] px-2 py-1 text-[10px] font-bold uppercase">
                                            {mid.chars().take(4).collect::<String>()}
                                        </span>
                                    </div>
                                    <div class="mt-3 grid grid-cols-2 gap-2 text-sm">
                                        <div>
                                            <p class="text-[var(--muted)]">"Total payé"</p>
                                            <p class="font-semibold text-[var(--brand)]">{format!("{:.2} €", total)}</p>
                                        </div>
                                        <div>
                                            <p class="text-[var(--muted)]">"Dette"</p>
                                            <p class=debt_text_class(balance)>
                                                {move || {
                                                    format!(
                                                        "{} €",
                                                        format_stored_money(balance, intuitive.get())
                                                    )
                                                }}
                                            </p>
                                        </div>
                                    </div>
                                    <div class="mt-3 flex items-end justify-between gap-2">
                                        <p class="text-xs text-[var(--muted)]">
                                            "Dernier paiement : " {last_label}
                                        </p>
                                        <div class="flex flex-nowrap gap-1">
                                            <Button
                                                variant=ButtonVariant::Secondary
                                                class="!min-h-8 !px-2 !py-1 text-sm"
                                                on_click=Callback::new(move |_| {
                                                    view_row.set(Some(row_view.clone()));
                                                    view_open.set(true);
                                                })
                                            >
                                                "👁"
                                            </Button>
                                            <Button
                                                class="!min-h-8 !px-2 !py-1 text-sm"
                                                on_click=Callback::new(move |_| {
                                                    prefill_member.set(row_edit.member.id.clone());
                                                    pay_open.set(true);
                                                })
                                            >
                                                "✎"
                                            </Button>
                                            <Button
                                                variant=ButtonVariant::Danger
                                                class="!min-h-8 !px-2 !py-1 text-sm"
                                                on_click=Callback::new(move |_| {
                                                    let mid = row_clear.member.id.clone();
                                                    let pid = current_period.get_untracked();
                                                    if pid.is_empty() {
                                                        error.set(Some("Sélectionnez un mois actuel.".into()));
                                                        return;
                                                    }
                                                    spawn_local(async move {
                                                        match api::clear_payment(&mid, &pid).await {
                                                            Ok(_) => reload(),
                                                            Err(e) => error.set(Some(e)),
                                                        }
                                                    });
                                                })
                                            >
                                                "🗑"
                                            </Button>
                                            <Button
                                                variant=ButtonVariant::Secondary
                                                class="!min-h-8 !px-2 !py-1 text-sm"
                                                on_click=Callback::new(move |_| {
                                                    let pid = current_period.get_untracked();
                                                    let cell = row_receipt
                                                        .periods
                                                        .iter()
                                                        .find(|c| c.period_id == pid && c.amount_paid.is_some())
                                                        .or_else(|| {
                                                            row_receipt
                                                                .periods
                                                                .iter()
                                                                .rev()
                                                                .find(|c| c.amount_paid.is_some())
                                                        });
                                                    let amount = cell.and_then(|c| c.amount_paid).unwrap_or(0.0);
                                                    let period_label = cell
                                                        .map(|c| c.label.clone())
                                                        .unwrap_or_default();
                                                    let monthly_amt = monthly
                                                        .get_untracked()
                                                        .replace(',', ".")
                                                        .parse()
                                                        .unwrap_or(0.0);
                                                    receipt.set(Some(build_payment_receipt(
                                                        format!(
                                                            "{} {}",
                                                            row_receipt.member.last_name,
                                                            row_receipt.member.first_name
                                                        ),
                                                        row_receipt.member.card_number.clone(),
                                                        row_receipt.member.id.clone(),
                                                        period_label,
                                                        amount,
                                                        row_receipt.balance,
                                                        row_receipt.total_paid,
                                                        year.get_untracked(),
                                                        org_name.get_untracked(),
                                                        String::new(),
                                                        monthly_amt,
                                                        row_receipt.prior_december_debt,
                                                    )));
                                                })
                                            >
                                                "🧾"
                                            </Button>
                                        </div>
                                    </div>
                                </div>
                            }
                        }
                    />
                </div>
            </Show>

            <Show when=move || view_mode.get() == "table">
            {move || {
                let g = grid.get();
                g.map(|g| {
                let period_labels: Vec<(usize, String, Option<String>)> = g
                    .periods
                    .iter()
                    .enumerate()
                    .map(|(i, p)| (i, p.label.clone(), p.label_color.clone()))
                    .collect();
                let monthly_amt = g.year.monthly_amount;
                let sk = sort_key.get();
                let asc = sort_asc.get();
                let density = row_density;
                let intuit = intuitive.get();
                view! {
                    <div class="flex flex-col gap-0">
                    <Table class="text-sm" density=density fullscreen_toggle=true>
                        <THead>
                            <Th>
                                <input
                                    type="checkbox"
                                    class="h-4 w-4"
                                    prop:checked=move || all_visible_selected.get()
                                    on:change=move |_| {
                                        let vis = visible_rows.get_untracked();
                                        if all_visible_selected.get_untracked() {
                                            selected.update(|sel| {
                                                sel.retain(|id| {
                                                    !vis.iter().any(|r| &r.member.id == id)
                                                });
                                            });
                                        } else {
                                            selected.update(|sel| {
                                                for r in &vis {
                                                    if !sel.iter().any(|id| id == &r.member.id) {
                                                        sel.push(r.member.id.clone());
                                                    }
                                                }
                                            });
                                        }
                                    }
                                />
                            </Th>
                            <Th
                                class="cursor-pointer"
                                on_click=Callback::new(move |_| toggle_sort(SortKey::Nom))
                            >
                                {format!("NOM{}", sort_arrow(sk == SortKey::Nom, asc))}
                            </Th>
                            <Th
                                class="cursor-pointer"
                                on_click=Callback::new(move |_| toggle_sort(SortKey::Prenom))
                            >
                                {format!("PRÉNOM{}", sort_arrow(sk == SortKey::Prenom, asc))}
                            </Th>
                            <Th
                                class="cursor-pointer"
                                on_click=Callback::new(move |_| toggle_sort(SortKey::Carte))
                            >
                                {format!("N°CARTE{}", sort_arrow(sk == SortKey::Carte, asc))}
                            </Th>
                            <Th
                                class="cursor-pointer"
                                on_click=Callback::new(move |_| toggle_sort(SortKey::Prior))
                            >
                                {format!("DETTE PRÉC.{}", sort_arrow(sk == SortKey::Prior, asc))}
                            </Th>
                            {period_labels.clone().into_iter().map(|(i, label, color)| {
                                let due_key = SortKey::PeriodDue(i);
                                let paid_key = SortKey::PeriodPaid(i);
                                let due_style = color
                                    .as_ref()
                                    .map(|c| {
                                        format!(
                                            "background-color:color-mix(in srgb,{c} 35%,var(--bg-elevated));"
                                        )
                                    })
                                    .unwrap_or_default();
                                let paid_style = color
                                    .as_ref()
                                    .map(|c| {
                                        format!(
                                            "background-color:color-mix(in srgb,{c} 18%,var(--bg-elevated));"
                                        )
                                    })
                                    .unwrap_or_default();
                                view! {
                                    <th
                                        class="cursor-pointer whitespace-nowrap px-2 font-semibold"
                                        style=format!(
                                            "padding-top:var(--table-row-py);padding-bottom:var(--table-row-py);{due_style}"
                                        )
                                        on:click=move |_| toggle_sort(due_key)
                                    >
                                        {format!("{}{}", label, sort_arrow(sk == due_key, asc))}
                                    </th>
                                    <th
                                        class="cursor-pointer whitespace-nowrap px-2 font-normal"
                                        style=format!(
                                            "padding-top:var(--table-row-py);padding-bottom:var(--table-row-py);{paid_style}"
                                        )
                                        on:click=move |_| toggle_sort(paid_key)
                                    >
                                        {format!("{}{}", label, sort_arrow(sk == paid_key, asc))}
                                    </th>
                                }
                            }).collect_view()}
                            <Th
                                class="cursor-pointer"
                                on_click=Callback::new(move |_| toggle_sort(SortKey::Total))
                            >
                                {format!("TOTAL PAYÉ{}", sort_arrow(sk == SortKey::Total, asc))}
                            </Th>
                            <Th
                                class="cursor-pointer"
                                on_click=Callback::new(move |_| toggle_sort(SortKey::Balance))
                            >
                                {format!("DETTE{}", sort_arrow(sk == SortKey::Balance, asc))}
                            </Th>
                            <Th sticky=true>{move || i18n.t("common.actions")}</Th>
                        </THead>
                        <TBody>
                            <For
                                each=move || visible_rows.get()
                                key=|r| r.member.id.clone()
                                children=move |row| {
                                    let mid = row.member.id.clone();
                                    let mid_check = mid.clone();
                                    let period_cells: Vec<(String, f64, Option<f64>)> = row
                                        .periods
                                        .iter()
                                        .map(|c| {
                                            (c.period_id.clone(), c.amount_due, c.amount_paid)
                                        })
                                        .collect();
                                    let bal = format_stored_money(row.balance, intuit);
                                    let bal_cls = debt_text_class(row.balance);
                                    let prior_stored = row.prior_december_debt;
                                    let prior_base = format_stored_money(prior_stored, intuit);
                                    let prior_cls = debt_text_class(prior_stored);
                                    let prior_key = format!("{}:prior", row.member.id);
                                    let prior_key_v = prior_key.clone();
                                    let prior_key_c = prior_key.clone();
                                    let mid_prior = row.member.id.clone();
                                    let total_paid_disp = money(row.total_paid);
                                    let row_view = row.clone();
                                    let row_edit = row.clone();
                                    let row_clear = row.clone();
                                    let row_receipt = row.clone();
                                    view! {
                                        <Tr>
                                            <Td>
                                                <input
                                                    type="checkbox"
                                                    class="h-4 w-4"
                                                    prop:checked=move || {
                                                        selected.get().iter().any(|id| id == &mid_check)
                                                    }
                                                    on:change=move |_| toggle_selected(mid.clone())
                                                />
                                            </Td>
                                            <Td>{row.member.last_name.clone()}</Td>
                                            <Td>{row.member.first_name.clone()}</Td>
                                            <Td>{row.member.card_number.clone()}</Td>
                                            <Td class=prior_cls>
                                                <EditableCell
                                                    value=Signal::derive(move || {
                                                        pending_cells
                                                            .get()
                                                            .get(&prior_key_v)
                                                            .map(|t| t.3.clone())
                                                            .unwrap_or_else(|| prior_base.clone())
                                                    })
                                                    on_commit=Callback::new(move |v: String| {
                                                        pending_cells.update(|m| {
                                                            m.insert(
                                                                prior_key_c.clone(),
                                                                (
                                                                    mid_prior.clone(),
                                                                    String::new(),
                                                                    "prior".into(),
                                                                    v,
                                                                ),
                                                            );
                                                        });
                                                    })
                                                    input_type="number"
                                                    class=prior_cls
                                                />
                                            </Td>
                                            {period_cells.into_iter().map(|(pid, amount_due, amount_paid)| {
                                                let mid_due = row.member.id.clone();
                                                let mid_paid = row.member.id.clone();
                                                let pid_due = pid.clone();
                                                let pid_paid = pid.clone();
                                                let base_due = format_stored_money(amount_due, intuit);
                                                let due_cls = debt_text_class(amount_due);
                                                let paid_cls = paid_text_class(amount_paid);
                                                let base_paid = amount_paid
                                                    .map(money)
                                                    .unwrap_or_default();
                                                let due_key = format!("{mid_due}:{pid_due}:due");
                                                let paid_key = format!("{mid_paid}:{pid_paid}:paid");
                                                let due_key_v = due_key.clone();
                                                let paid_key_v = paid_key.clone();
                                                let due_key_c = due_key.clone();
                                                let paid_key_c = paid_key.clone();
                                                view! {
                                                    <Td class=due_cls>
                                                        <EditableCell
                                                            value=Signal::derive(move || {
                                                                pending_cells
                                                                    .get()
                                                                    .get(&due_key_v)
                                                                    .map(|t| t.3.clone())
                                                                    .unwrap_or_else(|| base_due.clone())
                                                            })
                                                            on_commit=Callback::new(move |v: String| {
                                                                pending_cells.update(|m| {
                                                                    m.insert(
                                                                        due_key_c.clone(),
                                                                        (
                                                                            mid_due.clone(),
                                                                            pid_due.clone(),
                                                                            "due".into(),
                                                                            v,
                                                                        ),
                                                                    );
                                                                });
                                                            })
                                                            input_type="number"
                                                            class=due_cls
                                                        />
                                                    </Td>
                                                    <Td class=paid_cls>
                                                        <EditableCell
                                                            value=Signal::derive(move || {
                                                                pending_cells
                                                                    .get()
                                                                    .get(&paid_key_v)
                                                                    .map(|t| t.3.clone())
                                                                    .unwrap_or_else(|| base_paid.clone())
                                                            })
                                                            on_commit=Callback::new(move |v: String| {
                                                                pending_cells.update(|m| {
                                                                    m.insert(
                                                                        paid_key_c.clone(),
                                                                        (
                                                                            mid_paid.clone(),
                                                                            pid_paid.clone(),
                                                                            "paid".into(),
                                                                            v,
                                                                        ),
                                                                    );
                                                                });
                                                            })
                                                            input_type="number"
                                                            class=paid_cls
                                                        />
                                                    </Td>
                                                }
                                            }).collect_view()}
                                            <Td class="font-semibold text-[var(--brand)]">{total_paid_disp}</Td>
                                            <Td class=bal_cls>{bal}</Td>
                                            <Td sticky=true>
                                                <div class="flex flex-nowrap gap-1">
                                                    <Button
                                                        variant=ButtonVariant::Secondary
                                                        class="!min-h-8 !px-2 !py-1 text-sm"
                                                        on_click=Callback::new(move |_| {
                                                            view_row.set(Some(row_view.clone()));
                                                            view_open.set(true);
                                                        })
                                                    >
                                                        "👁"
                                                    </Button>
                                                    <Button
                                                        class="!min-h-8 !px-2 !py-1 text-sm"
                                                        on_click=Callback::new(move |_| {
                                                            prefill_member.set(row_edit.member.id.clone());
                                                            pay_open.set(true);
                                                        })
                                                    >
                                                        "✎"
                                                    </Button>
                                                    <Button
                                                        variant=ButtonVariant::Danger
                                                        class="!min-h-8 !px-2 !py-1 text-sm"
                                                        on_click=Callback::new(move |_| {
                                                            let mid = row_clear.member.id.clone();
                                                            let pid = current_period.get_untracked();
                                                            if pid.is_empty() {
                                                                error.set(Some("Sélectionnez un mois actuel.".into()));
                                                                return;
                                                            }
                                                            spawn_local(async move {
                                                                match api::clear_payment(&mid, &pid).await {
                                                                    Ok(_) => reload(),
                                                                    Err(e) => error.set(Some(e)),
                                                                }
                                                            });
                                                        })
                                                    >
                                                        "🗑"
                                                    </Button>
                                                    <Button
                                                        variant=ButtonVariant::Secondary
                                                        class="!min-h-8 !px-2 !py-1 text-sm"
                                                        on_click=Callback::new(move |_| {
                                                            let pid = current_period.get_untracked();
                                                            let cell = row_receipt
                                                                .periods
                                                                .iter()
                                                                .find(|c| c.period_id == pid && c.amount_paid.is_some())
                                                                .or_else(|| {
                                                                    row_receipt
                                                                        .periods
                                                                        .iter()
                                                                        .rev()
                                                                        .find(|c| c.amount_paid.is_some())
                                                                });
                                                            let amount = cell.and_then(|c| c.amount_paid).unwrap_or(0.0);
                                                            let period_label = cell
                                                                .map(|c| c.label.clone())
                                                                .unwrap_or_default();
                                                            receipt.set(Some(build_payment_receipt(
                                                                format!(
                                                                    "{} {}",
                                                                    row_receipt.member.last_name,
                                                                    row_receipt.member.first_name
                                                                ),
                                                                row_receipt.member.card_number.clone(),
                                                                row_receipt.member.id.clone(),
                                                                period_label,
                                                                amount,
                                                                row_receipt.balance,
                                                                row_receipt.total_paid,
                                                                year.get_untracked(),
                                                                org_name.get_untracked(),
                                                                String::new(),
                                                                monthly_amt,
                                                                row_receipt.prior_december_debt,
                                                            )));
                                                        })
                                                    >
                                                        "🧾"
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
                }
            })}}
            </Show>
            </div>

            <TablePaginationBar
                total=filtered_total
                page=page
                page_size=page_size
                mode=load_mode
                lazy_count=lazy_count
            />

            <Modal
                open=view_open.into()
                title="Fiche membre / dette"
                on_close=Callback::new(move |_| {
                    view_open.set(false);
                    view_row.set(None);
                })
            >
                <Show when=move || view_row.get().is_some()>
                    <div class="flex flex-col gap-3 text-sm">
                        {move || {
                            view_row.get().map(|r| {
                                view! {
                                    <p class="font-semibold text-lg">
                                        {format!("{} {} ({})", r.member.last_name, r.member.first_name, r.member.card_number)}
                                    </p>
                                    <p>"Statut : " <strong>{r.member.status.clone()}</strong></p>
                                    <p>"Dette : " <strong>{money(r.balance)} " €"</strong></p>
                                    <p class="text-[var(--muted)]">
                                        "Total payé : " {money(r.total_paid)} " € — Dette préc. : "
                                        {money(r.prior_december_debt)} " € — Ristourne : " {money(r.ristourne)} " €"
                                    </p>
                                    <ul class="mt-2 space-y-1">
                                        {r.periods.into_iter().map(|c| {
                                            view! {
                                                <li>
                                                    {c.label} " — dû " {money(c.amount_due)}
                                                    " / payé "
                                                    {c.amount_paid.map(money).unwrap_or_else(|| "—".into())}
                                                </li>
                                            }
                                        }).collect_view()}
                                    </ul>
                                }
                            })
                        }}
                    </div>
                </Show>
            </Modal>

            <PaymentModal
                open=pay_open
                year=year.into()
                current_period_id=current_period.into()
                current_period_label=current_period_label
                current_period_month=current_period_month
                period_options=current_period_options
                members=members_sig
                org_name=org_name.into()
                monthly_amount=Signal::derive(move || {
                    monthly
                        .get()
                        .replace(',', ".")
                        .parse()
                        .unwrap_or(0.0)
                })
                prefill_member_id=prefill_member
                on_paid=Callback::new(move |r| {
                    receipt.set(Some(r));
                    reload();
                })
            />
            <ExcelImportModal
                open=import_open
                year=year.into()
                on_done=Callback::new(move |_| reload())
            />
            <ReceiptDialog receipt=receipt />
        </div>
    }
}
