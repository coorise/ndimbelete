use chrono::Datelike;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::HashMap;
use wasm_bindgen::JsCast;

use crate::app::components::cotisations::{ExcelImportModal, PaymentModal, ReceiptDialog};
use crate::app::components::ui::{
    Button, ButtonVariant, EditableCell, Input, LazyScrollRegion, Modal, MultiSelect,
    RowDensitySlider, Select, SelectOption, Table, TableFullscreenToggle, TableLoadMode,
    TablePaginationBar, TBody, Td, Th, THead, Tr, DEFAULT_TABLE_LOAD_MODE, default_lazy_count,
    paginate_slice,
};
use crate::app::hooks::use_table_fullscreen;
use crate::app::i18n::use_i18n;
use crate::app::lib::{
    api, build_payment_receipt, debt_text_class, format_stored_money, is_intuitive_sign,
    paid_text_class, payment_date_for_period, AppSettings, ContributionPeriod, Member,
    PaymentReceipt, PeriodCell, YearGrid, YearGridRow,
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

/// Excel running debt after that period's payment (not a sum of cumulative dues).
fn balance_as_of(row: &YearGridRow, as_of_month: Option<i32>) -> f64 {
    let Some(m) = as_of_month else {
        return row.balance;
    };
    row.periods
        .iter()
        .filter(|c| c.period_month <= m)
        .last()
        .map(|c| c.amount_due - c.amount_paid.unwrap_or(0.0))
        .unwrap_or(row.prior_december_debt - row.ristourne)
}

fn parse_selected_months(raw: &[String]) -> Vec<i32> {
    let mut months: Vec<i32> = raw
        .iter()
        .filter_map(|s| s.parse().ok())
        .filter(|m| (1..=12).contains(m))
        .collect();
    months.sort_unstable();
    months.dedup();
    months
}

fn months_as_of(months: &[i32]) -> Option<i32> {
    months.iter().copied().max()
}

fn row_paid_for_months(row: &YearGridRow, months: &[i32]) -> bool {
    if months.is_empty() {
        !row.periods.is_empty() && row.periods.iter().all(period_paid)
    } else {
        months.iter().all(|m| {
            row.periods
                .iter()
                .find(|c| c.period_month == *m)
                .map(period_paid)
                .unwrap_or(false)
        })
    }
}

fn row_unpaid_for_months(row: &YearGridRow, months: &[i32]) -> bool {
    if months.is_empty() {
        row.periods.is_empty() || row.periods.iter().any(period_unpaid)
    } else {
        !row_paid_for_months(row, months)
    }
}

fn payment_method_rank(pm: &str) -> u8 {
    match pm {
        "bank_transfer" | "bank" | "virement" => 0,
        "none" | "aucun" => 2,
        _ => 1,
    }
}

fn payment_method_label(pm: &str) -> &'static str {
    match pm {
        "bank_transfer" | "bank" | "virement" => "Banque",
        "none" | "aucun" => "Aucun",
        _ => "Espèces",
    }
}

fn cotisation_chip_class(active: bool) -> &'static str {
    if active {
        "tap-target rounded-xl bg-[var(--brand)] px-3 py-2 text-sm font-semibold text-white shadow-sm"
    } else {
        "tap-target rounded-xl border border-[var(--border)] bg-[var(--bg-elevated)] px-3 py-2 text-sm font-semibold text-[var(--fg)] hover:bg-[color-mix(in_srgb,var(--brand-yellow)_28%,var(--bg-elevated))]"
    }
}

fn last_paid_cell(row: &YearGridRow) -> Option<&PeriodCell> {
    row.periods
        .iter()
        .rev()
        .find(|c| c.amount_paid.map(|p| p > 0.001).unwrap_or(false))
}

fn receipt_date_for_cell(
    cell: Option<&PeriodCell>,
    periods: &[ContributionPeriod],
    year: i32,
) -> Option<String> {
    let cell = cell?;
    let meeting = periods
        .iter()
        .find(|p| p.id == cell.period_id)
        .and_then(|p| p.meeting_date.as_deref());
    Some(payment_date_for_period(
        meeting,
        cell.period_month,
        year,
    ))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SortKey {
    Carte,
    Nom,
    Prenom,
    Payment,
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
    let period_filter = RwSignal::new(Vec::<String>::new());
    let pay_method_filter = RwSignal::new(String::new()); // "" | bank_transfer | cash
    let current_period = RwSignal::new(String::new()); // period id

    let sort_key = RwSignal::new(SortKey::Payment);
    let sort_secondary = RwSignal::new(SortKey::Nom);
    let sort_asc = RwSignal::new(true);

    let view_open = RwSignal::new(false);
    let view_row = RwSignal::new(Option::<YearGridRow>::None);
    let prefill_member = RwSignal::new(String::new());

    let selected = RwSignal::new(Vec::<String>::new());
    let row_density = RwSignal::new(0.35_f64);
    let page = RwSignal::new(0usize);
    let page_size = RwSignal::new(25usize);
    let load_mode = RwSignal::new(DEFAULT_TABLE_LOAD_MODE.to_string());
    let lazy_count = RwSignal::new(25usize);
    let advanced_open = RwSignal::new(false);
    let fs = use_table_fullscreen();
    let view_mode = RwSignal::new("table".to_string()); // table | grid
    // key = "{member_id}:{period_id}:{field}" → (member_id, period_id, field, value_str)
    let pending_cells =
        RwSignal::new(HashMap::<String, (String, String, String, String)>::new());
    let cell_save_gen = RwSignal::new(0u64);
    let cell_saving = RwSignal::new(false);

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
        pending_cells.set(HashMap::new());
        cell_save_gen.update(|g| *g += 1);
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

    let flush_pending_cells = move || {
        let y = year.get_untracked();
        let intuit = intuitive.get_untracked();
        spawn_local(async move {
            let entries_map = pending_cells.get_untracked();
            if entries_map.is_empty() {
                return;
            }
            // Snapshot then clear so edits during save stay queued.
            let mut entries: Vec<_> = entries_map.into_values().collect();
            pending_cells.set(HashMap::new());
            cell_saving.set(true);
            // prior first (full rebuild), then paid, then due anchors — Excel order.
            entries.sort_by_key(|(_, _, field, _)| match field.as_str() {
                "prior" => 0,
                "paid" => 1,
                "due" => 2,
                _ => 3,
            });
            for (mid, pid, field, value_str) in entries {
                let parsed = parse_money_opt(&value_str);
                if field == "prior" {
                    let stored = match parsed {
                        Some(v) if intuit => -v,
                        Some(v) => v,
                        None => 0.0,
                    };
                    if let Err(e) = api::set_prior_december_debt(&mid, y, stored).await {
                        error.set(Some(e));
                        cell_saving.set(false);
                        return;
                    }
                    continue;
                }
                let value = if field == "due" {
                    parsed.map(|v| if intuit { -v } else { v })
                } else {
                    parsed
                };
                if let Err(e) = api::set_period_cell(&mid, &pid, &field, value).await {
                    error.set(Some(e));
                    cell_saving.set(false);
                    return;
                }
            }
            cell_saving.set(false);
            reload();
        });
    };

    let queue_cell_edit = move |key: String, entry: (String, String, String, String)| {
        pending_cells.update(|m| {
            m.insert(key, entry);
        });
        cell_save_gen.update(|g| *g += 1);
        let gen = cell_save_gen.get_untracked();
        spawn_local(async move {
            // Debounce: feel Excel-like — pause after editing, then recalc forward.
            gloo_timers::future::TimeoutFuture::new(400).await;
            if cell_save_gen.get_untracked() != gen {
                return;
            }
            flush_pending_cells();
        });
    };

    let toggle_sort = move |key: SortKey| {
        if sort_key.get_untracked() == key {
            sort_asc.update(|v| *v = !*v);
        } else {
            sort_secondary.set(sort_key.get_untracked());
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
        let mut opts = Vec::new();
        let mut seen = std::collections::HashSet::new();
        if let Some(g) = grid.get() {
            for p in g.periods {
                if seen.insert(p.period_month) {
                    opts.push(SelectOption {
                        value: p.period_month.to_string(),
                        label: format!("{} ({})", p.label, p.period_month),
                    });
                }
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
                label: "Soldés".into(),
            },
            SelectOption {
                value: "unpaid".into(),
                label: "Non soldés".into(),
            },
            SelectOption {
                value: "debt".into(),
                label: "Avec dette".into(),
            },
            SelectOption {
                value: "surplus".into(),
                label: "Avec surplus".into(),
            },
        ]
    });

    let filtered_rows = Signal::derive(move || {
        let q = search.get().trim().to_lowercase();
        let pf = payment_filter.get();
        let months = parse_selected_months(&period_filter.get());
        let pm_filter = pay_method_filter.get();
        let key = sort_key.get();
        let secondary = sort_secondary.get();
        let asc = sort_asc.get();
        let as_of = months_as_of(&months);

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
                if !pm_filter.is_empty() {
                    let pm = row.member.payment_method.as_str();
                    let ok = match pm_filter.as_str() {
                        "bank_transfer" => {
                            matches!(pm, "bank_transfer" | "bank" | "virement")
                        }
                        "cash" => {
                            !matches!(pm, "bank_transfer" | "bank" | "virement" | "none" | "aucun")
                        }
                        _ => true,
                    };
                    if !ok {
                        return false;
                    }
                }
                let bal = balance_as_of(row, as_of);
                match pf.as_str() {
                    "debt" => bal > 0.001,
                    "surplus" => bal < -0.001,
                    "paid" => row_paid_for_months(row, &months),
                    "unpaid" | "unfulfilled" => row_unpaid_for_months(row, &months),
                    _ => true,
                }
            })
            .collect::<Vec<_>>();

        let cmp_key = |a: &YearGridRow, b: &YearGridRow, key: SortKey| -> std::cmp::Ordering {
            match key {
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
                SortKey::Payment => payment_method_rank(&a.member.payment_method)
                    .cmp(&payment_method_rank(&b.member.payment_method)),
                SortKey::Prior => a
                    .prior_december_debt
                    .partial_cmp(&b.prior_december_debt)
                    .unwrap_or(std::cmp::Ordering::Equal),
                SortKey::Total => a
                    .total_paid
                    .partial_cmp(&b.total_paid)
                    .unwrap_or(std::cmp::Ordering::Equal),
                SortKey::Balance => {
                    let ab = balance_as_of(a, as_of);
                    let bb = balance_as_of(b, as_of);
                    ab.partial_cmp(&bb).unwrap_or(std::cmp::Ordering::Equal)
                }
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
            }
        };

        rows.sort_by(|a, b| {
            let mut ord = cmp_key(a, b, key);
            if !asc {
                ord = ord.reverse();
            }
            if ord == std::cmp::Ordering::Equal && secondary != key {
                let mut sec = cmp_key(a, b, secondary);
                // Secondary always ascending for stable name/bank grouping.
                if key == SortKey::Payment {
                    sec = cmp_key(a, b, secondary);
                }
                ord = sec;
            }
            ord
        });

        rows
    });

    let filtered_total = Signal::derive(move || filtered_rows.get().len());

    let status_counts = Signal::derive(move || {
        let rows = grid.get().map(|g| g.rows).unwrap_or_default();
        let months = parse_selected_months(&period_filter.get());
        let as_of = months_as_of(&months);
        let mut paid = 0i64;
        let mut unpaid = 0i64;
        let mut debt = 0i64;
        let mut surplus = 0i64;
        for row in &rows {
            if row_paid_for_months(row, &months) {
                paid += 1;
            } else {
                unpaid += 1;
            }
            let bal = balance_as_of(row, as_of);
            if bal > 0.001 {
                debt += 1;
            } else if bal < -0.001 {
                surplus += 1;
            }
        }
        (paid, unpaid, debt, surplus)
    });

    let reset_lazy_window = move || {
        page.set(0);
        lazy_count.set(default_lazy_count(page_size.get_untracked()));
    };

    let filtered_money_stats = Signal::derive(move || {
        let rows = filtered_rows.get();
        let g = grid.get();
        let months = parse_selected_months(&period_filter.get());
        let monthly_amt = g.as_ref().map(|g| g.year.monthly_amount).unwrap_or(20.0);
        let as_of = months_as_of(&months);
        let n_scope = if months.is_empty() {
            g.as_ref().map(|g| g.periods.len().max(1)).unwrap_or(1) as f64
        } else {
            months.len() as f64
        };
        // Expected due = members × monthly × scoped periods (never Excel running dues).
        let expected = rows.len() as f64 * monthly_amt * n_scope;
        let received: f64 = if months.is_empty() {
            rows.iter().map(|r| r.total_paid).sum()
        } else {
            rows.iter()
                .map(|r| {
                    r.periods
                        .iter()
                        .filter(|c| months.contains(&c.period_month))
                        .map(|c| c.amount_paid.unwrap_or(0.0))
                        .sum::<f64>()
                })
                .sum()
        };
        let debt: f64 = rows
            .iter()
            .map(|r| balance_as_of(r, as_of).max(0.0))
            .sum();
        let surplus: f64 = rows
            .iter()
            .map(|r| (-balance_as_of(r, as_of)).max(0.0))
            .sum();
        (received, expected, debt, surplus)
    });

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
                <MultiSelect
                    label="Mois (planning)"
                    options=period_filter_options
                    value=period_filter.into()
                    on_change=Callback::new(move |v| {
                        period_filter.set(v);
                        reset_lazy_window();
                    })
                    class="w-52"
                    empty_label="Toute l'année"
                />
                <div class="relative w-52 max-w-full shrink-0">
                    <Input
                        label="Recherche"
                        placeholder="N° carte ou nom…"
                        value=search.into()
                        on_input=Callback::new(move |v| {
                            search.set(v);
                            reset_lazy_window();
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
                                reset_lazy_window();
                            }
                        >
                            "✕"
                        </button>
                    </Show>
                </div>
                <div class="flex flex-col gap-1">
                    <span class="text-sm font-medium">"Statut"</span>
                    <div class="flex flex-wrap gap-2">
                        <button
                            type="button"
                            class=move || cotisation_chip_class(payment_filter.get() == "paid")
                            on:click=move |_| {
                                if payment_filter.get_untracked() == "paid" {
                                    payment_filter.set(String::new());
                                } else {
                                    payment_filter.set("paid".into());
                                }
                                reset_lazy_window();
                            }
                        >
                            {move || {
                                let n = status_counts.get().0;
                                if period_filter.get().is_empty() {
                                    format!("Soldés ({n})")
                                } else {
                                    format!("Payés ({n})")
                                }
                            }}
                        </button>
                        <button
                            type="button"
                            class=move || cotisation_chip_class(payment_filter.get() == "unpaid")
                            on:click=move |_| {
                                if payment_filter.get_untracked() == "unpaid" {
                                    payment_filter.set(String::new());
                                } else {
                                    payment_filter.set("unpaid".into());
                                }
                                reset_lazy_window();
                            }
                        >
                            {move || {
                                let n = status_counts.get().1;
                                if period_filter.get().is_empty() {
                                    format!("Non soldés ({n})")
                                } else {
                                    format!("Non payés ({n})")
                                }
                            }}
                        </button>
                        <button
                            type="button"
                            class=move || cotisation_chip_class(payment_filter.get() == "debt")
                            on:click=move |_| {
                                if payment_filter.get_untracked() == "debt" {
                                    payment_filter.set(String::new());
                                } else {
                                    payment_filter.set("debt".into());
                                }
                                reset_lazy_window();
                            }
                        >
                            {move || format!("Avec dette ({})", status_counts.get().2)}
                        </button>
                        <button
                            type="button"
                            class=move || cotisation_chip_class(payment_filter.get() == "surplus")
                            on:click=move |_| {
                                if payment_filter.get_untracked() == "surplus" {
                                    payment_filter.set(String::new());
                                } else {
                                    payment_filter.set("surplus".into());
                                }
                                reset_lazy_window();
                            }
                        >
                            {move || format!("Avec surplus ({})", status_counts.get().3)}
                        </button>
                    </div>
                </div>
                <div class="flex flex-col gap-1">
                    <span class="text-sm font-medium">"Paiement"</span>
                    <div class="flex flex-wrap gap-2">
                        <button
                            type="button"
                            class=move || cotisation_chip_class(pay_method_filter.get() == "bank_transfer")
                            on:click=move |_| {
                                if pay_method_filter.get_untracked() == "bank_transfer" {
                                    pay_method_filter.set(String::new());
                                } else {
                                    pay_method_filter.set("bank_transfer".into());
                                }
                                reset_lazy_window();
                            }
                        >
                            "Banque"
                        </button>
                        <button
                            type="button"
                            class=move || cotisation_chip_class(pay_method_filter.get() == "cash")
                            on:click=move |_| {
                                if pay_method_filter.get_untracked() == "cash" {
                                    pay_method_filter.set(String::new());
                                } else {
                                    pay_method_filter.set("cash".into());
                                }
                                reset_lazy_window();
                            }
                        >
                            "Espèces"
                        </button>
                    </div>
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
                        <div class="flex flex-col gap-1">
                            <div class="flex items-end gap-2">
                                <Input
                                    label="Cotisation mensuelle (€)"
                                    r#type="number"
                                    value=monthly.into()
                                    on_input=Callback::new(move |v| monthly.set(v))
                                    class="w-44"
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
                            <p class="max-w-xs text-xs text-[var(--muted)]">
                                {move || {
                                    let m = monthly
                                        .get()
                                        .replace(',', ".")
                                        .parse::<f64>()
                                        .unwrap_or(20.0);
                                    format!(
                                        "Défaut par période de planning (Jan, Mar…). Avec {:.0} €, janvier sans dette précédente affiche {:.0} €. Vous pouvez forcer une dette personnalisée dans une cellule.",
                                        m, m
                                    )
                                }}
                            </p>
                        </div>
                    </div>
                    <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
                        <Select
                            label="Paiement"
                            options=payment_options
                            value=payment_filter.into()
                            on_change=Callback::new(move |v| {
                                payment_filter.set(v);
                                reset_lazy_window();
                            })
                        />
                        <MultiSelect
                            label="Période"
                            options=period_filter_options
                            value=period_filter.into()
                            on_change=Callback::new(move |v| {
                                period_filter.set(v);
                                reset_lazy_window();
                            })
                            empty_label="Toute l'année"
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

            <div class="grid shrink-0 gap-3 sm:grid-cols-3">
                <div class="rounded-xl border border-[var(--border)] bg-[var(--surface)] px-4 py-3">
                    <p class="text-xs font-medium text-[var(--muted)]">
                        {move || {
                            let n = period_filter.get().len();
                            if n == 0 {
                                "Payé / Dû (année)".to_string()
                            } else if n == 1 {
                                "Payé / Dû (mois)".to_string()
                            } else {
                                format!("Payé / Dû ({n} mois)")
                            }
                        }}
                    </p>
                    <p class="mt-1 font-display text-lg font-bold text-[var(--brand)]">
                        {move || {
                            let (recv, exp, _, _) = filtered_money_stats.get();
                            format!("{:.0} € / {:.0} €", recv, exp)
                        }}
                    </p>
                </div>
                <div class="rounded-xl border border-[var(--border)] bg-[var(--surface)] px-4 py-3">
                    <p class="text-xs font-medium text-[var(--muted)]">
                        {move || {
                            if period_filter.get().is_empty() {
                                "Dette restante(inclut années précédentes)".to_string()
                            } else {
                                "Dette à date (inclut années précédentes)".to_string()
                            }
                        }}
                    </p>
                    <p class="mt-1 font-display text-lg font-bold text-[var(--brand-red)]">
                        {move || {
                            let (_, _, debt, _) = filtered_money_stats.get();
                            format!("{} €", format_stored_money(debt, is_intuitive_sign(&debt_sign.get())))
                        }}
                    </p>
                </div>
                <div class="rounded-xl border border-[var(--border)] bg-[var(--surface)] px-4 py-3">
                    <p class="text-xs font-medium text-[var(--muted)]">"Surplus"</p>
                    <p class="mt-1 font-display text-lg font-bold text-[var(--brand)]">
                        {move || {
                            let (_, _, _, surplus) = filtered_money_stats.get();
                            format!("{:.2} €", surplus)
                        }}
                    </p>
                </div>
            </div>

            <Show when=move || error.get().is_some()>
                <p class="shrink-0 text-[var(--brand-red)]">{move || error.get().unwrap_or_default()}</p>
            </Show>

            <Show when=move || !pending_cells.get().is_empty() || cell_saving.get()>
                <div class="sticky top-0 z-20 flex shrink-0 flex-wrap items-center justify-between gap-3 rounded-xl border border-[var(--brand)] bg-[var(--surface)] px-4 py-3 shadow-[var(--shadow)]">
                    <p class="text-sm font-medium">
                        {move || {
                            if cell_saving.get() {
                                "Recalcul Excel en cours…".to_string()
                            } else {
                                format!(
                                    "{} modification(s) — enregistrement automatique",
                                    pending_cells.get().len()
                                )
                            }
                        }}
                    </p>
                    <div class="flex flex-wrap gap-2">
                        <Button
                            variant=ButtonVariant::Secondary
                            on_click=Callback::new(move |_| {
                                cell_save_gen.update(|g| *g += 1);
                                pending_cells.set(HashMap::new());
                            })
                        >
                            "Annuler"
                        </Button>
                        <Button on_click=Callback::new(move |_| {
                            if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                                if let Some(el) = doc.active_element() {
                                    if let Ok(html) = el.dyn_into::<web_sys::HtmlElement>() {
                                        let _ = html.blur();
                                    }
                                }
                            }
                            cell_save_gen.update(|g| *g += 1);
                            spawn_local(async move {
                                gloo_timers::future::TimeoutFuture::new(0).await;
                                flush_pending_cells();
                            });
                        })>
                            {move || format!("Mettre à jour maintenant ({})", pending_cells.get().len())}
                        </Button>
                    </div>
                </div>
            </Show>

            <div class="flex min-h-0 flex-1 flex-col overflow-hidden">
            <Show when=move || view_mode.get() == "grid">
                <LazyScrollRegion
                    mode=load_mode
                    lazy_count=lazy_count
                    total=filtered_total
                    page_size=page_size
                    class="grid min-h-0 flex-1 content-start gap-3 overflow-y-auto overscroll-contain pr-1 pb-2 sm:grid-cols-2 xl:grid-cols-3"
                >
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
                                                    let y = year.get_untracked();
                                                    let periods = grid
                                                        .get_untracked()
                                                        .map(|g| g.periods)
                                                        .unwrap_or_default();
                                                    let cell = row_receipt
                                                        .periods
                                                        .iter()
                                                        .find(|c| {
                                                            c.period_id == pid
                                                                && c.amount_paid
                                                                    .map(|p| p > 0.001)
                                                                    .unwrap_or(false)
                                                        })
                                                        .or_else(|| last_paid_cell(&row_receipt));
                                                    let amount = cell.and_then(|c| c.amount_paid).unwrap_or(0.0);
                                                    let period_label = cell
                                                        .map(|c| c.label.clone())
                                                        .unwrap_or_default();
                                                    let pay_date = receipt_date_for_cell(
                                                        last_paid_cell(&row_receipt),
                                                        &periods,
                                                        y,
                                                    );
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
                                                        y,
                                                        org_name.get_untracked(),
                                                        String::new(),
                                                        monthly_amt,
                                                        row_receipt.prior_december_debt,
                                                        pay_date,
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
                </LazyScrollRegion>
            </Show>

            <Show when=move || view_mode.get() == "table">
            {move || {
                let g = grid.get();
                let months = parse_selected_months(&period_filter.get());
                g.map(|g| {
                let period_labels: Vec<(usize, String, Option<String>)> = g
                    .periods
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| months.is_empty() || months.contains(&p.period_month))
                    .map(|(i, p)| (i, p.label.clone(), p.label_color.clone()))
                    .collect();
                let monthly_amt = g.year.monthly_amount;
                let sk = sort_key.get();
                let asc = sort_asc.get();
                let density = row_density;
                let intuit = intuitive.get();
                let months_body = months.clone();
                view! {
                    <div class="flex min-h-0 flex-1 flex-col overflow-hidden">
                    <Table
                        class="text-sm"
                        density=density
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
                                on_click=Callback::new(move |_| toggle_sort(SortKey::Payment))
                            >
                                {format!("MODE PAIEMENT{}", sort_arrow(sk == SortKey::Payment, asc))}
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
                                        .filter(|c| {
                                            months_body.is_empty()
                                                || months_body.contains(&c.period_month)
                                        })
                                        .map(|c| {
                                            (c.period_id.clone(), c.amount_due, c.amount_paid)
                                        })
                                        .collect();
                                    let bal = format_stored_money(
                                        balance_as_of(&row, months_as_of(&months_body)),
                                        intuit,
                                    );
                                    let bal_cls = debt_text_class(balance_as_of(
                                        &row,
                                        months_as_of(&months_body),
                                    ));
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
                                            <Td>
                                                {
                                                    let pm = row.member.payment_method.clone();
                                                    let is_bank = matches!(
                                                        pm.as_str(),
                                                        "bank_transfer" | "bank" | "virement"
                                                    );
                                                    let label = payment_method_label(&pm);
                                                    view! {
                                                        <span
                                                            class=if is_bank {
                                                                "inline-block rounded-lg bg-[color-mix(in_srgb,var(--brand)_28%,transparent)] px-2 py-1 text-xs font-bold text-[var(--brand)]"
                                                            } else {
                                                                "inline-block px-2 py-1 text-xs font-medium text-[var(--muted)]"
                                                            }
                                                        >
                                                            {label}
                                                        </span>
                                                    }
                                                }
                                            </Td>
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
                                                        queue_cell_edit(
                                                            prior_key_c.clone(),
                                                            (
                                                                mid_prior.clone(),
                                                                String::new(),
                                                                "prior".into(),
                                                                v,
                                                            ),
                                                        );
                                                    })
                                                    placeholder="0"
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
                                                                queue_cell_edit(
                                                                    due_key_c.clone(),
                                                                    (
                                                                        mid_due.clone(),
                                                                        pid_due.clone(),
                                                                        "due".into(),
                                                                        v,
                                                                    ),
                                                                );
                                                            })
                                                            placeholder="0"
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
                                                                queue_cell_edit(
                                                                    paid_key_c.clone(),
                                                                    (
                                                                        mid_paid.clone(),
                                                                        pid_paid.clone(),
                                                                        "paid".into(),
                                                                        v,
                                                                    ),
                                                                );
                                                            })
                                                            placeholder="0"
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
                                                            let y = year.get_untracked();
                                                            let periods = grid
                                                                .get_untracked()
                                                                .map(|g| g.periods)
                                                                .unwrap_or_default();
                                                            let cell = row_receipt
                                                                .periods
                                                                .iter()
                                                                .find(|c| {
                                                                    c.period_id == pid
                                                                        && c.amount_paid
                                                                            .map(|p| p > 0.001)
                                                                            .unwrap_or(false)
                                                                })
                                                                .or_else(|| last_paid_cell(&row_receipt));
                                                            let amount = cell.and_then(|c| c.amount_paid).unwrap_or(0.0);
                                                            let period_label = cell
                                                                .map(|c| c.label.clone())
                                                                .unwrap_or_default();
                                                            let pay_date = receipt_date_for_cell(
                                                                last_paid_cell(&row_receipt),
                                                                &periods,
                                                                y,
                                                            );
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
                                                                y,
                                                                org_name.get_untracked(),
                                                                String::new(),
                                                                monthly_amt,
                                                                row_receipt.prior_december_debt,
                                                                pay_date,
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
