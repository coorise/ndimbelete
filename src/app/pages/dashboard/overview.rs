use chrono::Datelike;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::app::components::ui::{Card, SearchableSelect, SelectOption};
use crate::app::i18n::use_i18n;
use crate::app::lib::{
    api, DebtVsPaidPoint, Member, OverviewStats, PaymentStatusSlice, PeriodSeriesPoint,
};

fn money(v: f64) -> String {
    format!("{v:.0} €")
}

fn money2(v: f64) -> String {
    format!("{v:.2} €")
}

#[derive(Clone)]
struct ChartTooltipState {
    title: String,
    body: String,
    x: f64,
    y: f64,
}

fn tip_from_mouse(ev: &web_sys::MouseEvent, title: String, body: String) -> ChartTooltipState {
    ChartTooltipState {
        title,
        body,
        x: f64::from(ev.client_x()) + 14.0,
        y: f64::from(ev.client_y()) + 14.0,
    }
}

#[component]
pub fn OverviewPage() -> impl IntoView {
    let i18n = use_i18n();
    let year = RwSignal::new(chrono::Local::now().year());
    let member_id = RwSignal::new(String::new());
    let members = RwSignal::new(Vec::<Member>::new());
    let stats = RwSignal::new(Option::<OverviewStats>::None);
    let error = RwSignal::new(Option::<String>::None);
    let loading = RwSignal::new(true);
    let selected_period = RwSignal::new(Option::<String>::None);
    let selected_pie = RwSignal::new(Option::<String>::None);
    let tooltip = RwSignal::new(Option::<ChartTooltipState>::None);

    let reload = move || {
        let y = year.get_untracked();
        let mid = member_id.get_untracked();
        let mid_opt = if mid.is_empty() { None } else { Some(mid) };
        loading.set(true);
        spawn_local(async move {
            let _ = api::ensure_year(y).await;
            // Heal statuses when Excel had empty paid columns.
            let _ = api::recalculate_all_members(y).await;
            match api::get_overview_stats(y, mid_opt.as_deref()).await {
                Ok(s) => {
                    stats.set(Some(s));
                    error.set(None);
                }
                Err(e) => {
                    stats.set(None);
                    error.set(Some(e));
                }
            }
            loading.set(false);
        });
    };

    Effect::new(move |_| {
        let _ = year.get();
        let _ = member_id.get();
        selected_period.set(None);
        selected_pie.set(None);
        tooltip.set(None);
        reload();
    });

    Effect::new(move |_| {
        spawn_local(async move {
            if let Ok(list) = api::list_members().await {
                members.set(list);
            }
        });
    });

    let member_options = Signal::derive(move || {
        let mut opts = vec![SelectOption {
            value: String::new(),
            label: i18n.t("overview.all_members"),
        }];
        for m in members.get() {
            opts.push(SelectOption {
                value: m.id,
                label: format!("{} {} ({})", m.last_name, m.first_name, m.card_number),
            });
        }
        opts
    });

    view! {
        <div class="flex min-h-0 flex-1 flex-col gap-6 overflow-y-auto pr-3 pb-4">
            <div class="shrink-0 space-y-3">
                <div>
                    <h1 class="font-display text-3xl font-semibold">{move || i18n.t("overview.title")}</h1>
                    <p class="text-[var(--muted)]">"Indicateurs de cotisation pour l'année sélectionnée."</p>
                </div>
                <div class="flex flex-row flex-wrap items-end gap-3">
                    <label class="flex flex-col gap-1 text-sm font-medium">
                        "Année"
                        <input
                            class="tap-target w-28 rounded-xl border border-[var(--border)] bg-[var(--bg-elevated)] px-3 py-2"
                            type="number"
                            prop:value=move || year.get()
                            on:change=move |ev| {
                                if let Ok(y) = event_target_value(&ev).parse::<i32>() {
                                    year.set(y);
                                }
                            }
                        />
                    </label>
                    <SearchableSelect
                        label="Membre"
                        options=member_options
                        value=member_id.into()
                        on_change=Callback::new(move |v| member_id.set(v))
                        class="min-w-[16rem] w-72 max-w-full"
                        placeholder="Tous les membres"
                        search_placeholder="Nom ou n° carte…"
                    />
                </div>
            </div>

            <Show when=move || error.get().is_some()>
                <p class="rounded-xl bg-[color-mix(in_srgb,var(--brand-red)_12%,transparent)] px-3 py-2 text-[var(--brand-red)]" role="alert">
                    {move || error.get().unwrap_or_default()}
                </p>
            </Show>

            <Show when=move || loading.get() && stats.get().is_none()>
                <p class="text-[var(--muted)]">{move || i18n.t("common.loading")}</p>
            </Show>

            {move || {
                stats.get().map(|s| {
                    let by_period_bars = s.by_period.clone();
                    let pie = s.payment_status_pie.clone();
                    let debt_curve = s.debt_vs_paid.clone();
                    let no_payments = s.total_paid < 0.001;
                    let status_pie = vec![
                        PaymentStatusSlice {
                            label: "Actifs".into(),
                            count: s.active_count,
                        },
                        PaymentStatusSlice {
                            label: "Démissionnaires".into(),
                            count: s.demissionnaire_count,
                        },
                        PaymentStatusSlice {
                            label: "Exclus".into(),
                            count: s.exclu_count,
                        },
                    ];
                    view! {
                        <div class="flex flex-col gap-6">
                            <Show when=move || no_payments>
                                <p class="rounded-xl border border-[color-mix(in_srgb,var(--brand-yellow)_50%,var(--border))] bg-[color-mix(in_srgb,var(--brand-yellow)_18%,transparent)] px-4 py-3 text-sm">
                                    "Aucun paiement dans les colonnes « payé » Excel pour " {s.year}
                                    ". Les montants dus restent calculés. Vérifiez l’import du fichier (chaque période a deux colonnes : dû puis payé)."
                                </p>
                            </Show>

                            <Card title="Payé vs Dette (cumul)">
                                <DebtVsPaidChart
                                    points=debt_curve
                                    on_select=Callback::new(move |(label, ev): (String, web_sys::MouseEvent)| {
                                        selected_period.set(Some(label.clone()));
                                        tooltip.set(Some(tip_from_mouse(
                                            &ev,
                                            label.clone(),
                                            format!(
                                                "Période {label}. Ouvrez Cotisations pour le détail membre."
                                            ),
                                        )));
                                    })
                                />
                            </Card>

                            <div class="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
                                <StatCard label="Membres".to_string() value=s.member_count.to_string() />
                                <StatCard label="Actifs".to_string() value=s.active_count.to_string() />
                                <StatCard label="Démissionnaires".to_string() value=s.demissionnaire_count.to_string() />
                                <StatCard label="Total payé".to_string() value=money(s.total_paid) />
                                <StatCard label="Total dû".to_string() value=money(s.total_due) />
                                <StatCard label="Impayé".to_string() value=money(s.total_unpaid) />
                            </div>

                            <div class="grid gap-4 lg:grid-cols-2">
                                <Card title="Paiements par période">
                                    <BarChart
                                        points=by_period_bars.clone()
                                        on_select=Callback::new(move |(label, ev): (String, web_sys::MouseEvent)| {
                                            if let Some(p) = by_period_bars.iter().find(|x| x.label == label) {
                                                selected_period.set(Some(p.label.clone()));
                                                tooltip.set(Some(tip_from_mouse(
                                                    &ev,
                                                    p.label.clone(),
                                                    format!(
                                                        "Dû {} · Payé {} · Impayé {}",
                                                        money2(p.total_due),
                                                        money2(p.total_paid),
                                                        money2(p.unpaid)
                                                    ),
                                                )));
                                            }
                                        })
                                    />
                                </Card>
                                <Card title="Statut de paiement">
                                    <PieChart
                                        slices=pie
                                        on_select=Callback::new(move |(label, ev): (String, web_sys::MouseEvent)| {
                                            selected_pie.set(Some(label.clone()));
                                            tooltip.set(Some(tip_from_mouse(
                                                &ev,
                                                label.clone(),
                                                "Filtrez Cotisations par statut de paiement.".into(),
                                            )));
                                        })
                                    />
                                </Card>
                            </div>

                            <Card title="Statut des membres">
                                <PieChart
                                    slices=status_pie
                                    on_select=Callback::new(move |(label, ev): (String, web_sys::MouseEvent)| {
                                        selected_pie.set(Some(label.clone()));
                                        tooltip.set(Some(tip_from_mouse(
                                            &ev,
                                            format!("Membres — {label}"),
                                            "Répartition des statuts pour l'année.".into(),
                                        )));
                                    })
                                />
                            </Card>

                            <Show when=move || tooltip.get().is_some()>
                                <div
                                    class="fixed z-[80] max-w-xs rounded-xl border border-[var(--brand)] bg-[var(--bg-elevated)] px-3 py-2 text-sm shadow-[var(--shadow)]"
                                    style=move || {
                                        tooltip.get().map(|t| format!("left:{}px;top:{}px", t.x, t.y)).unwrap_or_default()
                                    }
                                    on:click=move |ev| ev.stop_propagation()
                                >
                                    <div class="flex items-start justify-between gap-3">
                                        <p class="font-semibold text-[var(--brand)]">
                                            {move || tooltip.get().map(|t| t.title).unwrap_or_default()}
                                        </p>
                                        <button
                                            type="button"
                                            class="text-xs font-bold text-[var(--muted)]"
                                            on:click=move |_| {
                                                tooltip.set(None);
                                                selected_period.set(None);
                                                selected_pie.set(None);
                                            }
                                        >
                                            "✕"
                                        </button>
                                    </div>
                                    <p class="mt-1 text-[var(--fg)]">
                                        {move || tooltip.get().map(|t| t.body).unwrap_or_default()}
                                    </p>
                                </div>
                            </Show>
                        </div>
                    }
                })
            }}
        </div>
    }
}

#[component]
fn StatCard(#[prop(into)] label: String, #[prop(into)] value: String) -> impl IntoView {
    view! {
        <div class="rounded-2xl border border-[var(--border)] bg-[var(--surface)] p-5 shadow-[var(--shadow)]">
            <p class="text-sm font-medium text-[var(--muted)]">{label}</p>
            <p class="mt-2 break-all font-display text-2xl font-bold leading-tight text-[var(--brand)] sm:text-3xl">
                {value}
            </p>
        </div>
    }
}

#[component]
fn BarChart(
    points: Vec<PeriodSeriesPoint>,
    #[prop(into)] on_select: Callback<(String, web_sys::MouseEvent)>,
) -> impl IntoView {
    let max = points
        .iter()
        .map(|p| p.total_paid.max(p.total_due))
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let w = 420.0_f64;
    let h = 200.0_f64;
    let n = points.len().max(1) as f64;
    let gap = w / n;
    let bar_w = gap * 0.36;

    view! {
        <svg viewBox=format!("0 0 {w} {h}") class="h-52 w-full" role="img" aria-label="Histogramme">
            // grid lines
            {(0..4).map(|i| {
                let y = 16.0 + (i as f64) * ((h - 36.0) / 3.0);
                view! {
                    <line
                        x1="8"
                        x2=(w - 8.0).to_string()
                        y1=y.to_string()
                        y2=y.to_string()
                        stroke="color-mix(in srgb, var(--fg) 10%, transparent)"
                        stroke-width="1"
                    ></line>
                }
            }).collect_view()}
            {points.into_iter().enumerate().map(|(i, p)| {
                let x0 = (i as f64) * gap + gap * 0.18;
                let paid_h = (p.total_paid / max) * (h - 40.0);
                let due_h = (p.total_due / max) * (h - 40.0);
                let label: String = p.label.chars().take(3).collect();
                let full = p.label.clone();
                let tip = format!(
                    "{} — dû {:.0} € / payé {:.0} €",
                    p.label, p.total_due, p.total_paid
                );
                view! {
                    <g
                        class="cursor-pointer"
                        on:click=move |ev| on_select.run((full.clone(), ev))
                    >
                        <title>{tip.clone()}</title>
                        <rect
                            x=x0.to_string()
                            y=(h - 24.0 - due_h).to_string()
                            width=bar_w.to_string()
                            height=due_h.max(1.0).to_string()
                            fill="color-mix(in srgb, var(--brand) 22%, transparent)"
                            rx="5"
                        ></rect>
                        <rect
                            x=(x0 + bar_w + 3.0).to_string()
                            y=(h - 24.0 - paid_h).to_string()
                            width=bar_w.to_string()
                            height=paid_h.max(0.0).to_string()
                            fill="var(--brand)"
                            rx="5"
                        ></rect>
                        <text
                            x=(x0 + bar_w).to_string()
                            y=(h - 6.0).to_string()
                            text-anchor="middle"
                            font-size="11"
                            fill="var(--muted)"
                        >
                            {label}
                        </text>
                    </g>
                }
            }).collect_view()}
        </svg>
        <p class="mt-2 text-xs text-[var(--muted)]">"Cliquez une période · clair = dû · foncé = payé"</p>
    }
}

#[component]
fn PieChart(
    slices: Vec<PaymentStatusSlice>,
    #[prop(into)] on_select: Callback<(String, web_sys::MouseEvent)>,
) -> impl IntoView {
    let total: f64 = slices.iter().map(|s| s.count as f64).sum::<f64>();
    let colors = ["#0B7A3E", "#C4A035", "#CE1126", "#4B5563"];
    let mut angle = -90.0_f64;
    let mut paths: Vec<(String, &'static str, String, i64)> = Vec::new();

    if total <= 0.0 {
        paths.push((
            "M 80 20 A 60 60 0 1 1 79.9 20 Z".to_string(),
            "#d1d5db",
            "Vide".into(),
            0,
        ));
    } else {
        for (i, s) in slices.iter().enumerate() {
            if s.count == 0 {
                continue;
            }
            let sweep = (s.count as f64 / total) * 360.0;
            let start = angle;
            angle += sweep;
            let d = if (sweep - 360.0).abs() < 0.01 {
                "M 80 20 A 60 60 0 1 1 79.9 20 Z".to_string()
            } else {
                donut_slice(80.0, 80.0, 60.0, start, angle)
            };
            paths.push((d, colors[i % colors.len()], s.label.clone(), s.count));
        }
    }

    let legend = slices.clone();

    view! {
        <div class="flex flex-wrap items-center gap-4">
            <svg viewBox="0 0 160 160" class="h-44 w-44 shrink-0" role="img" aria-label="Camembert">
                {paths.into_iter().map(|(d, color, label, count)| {
                    let lbl = label.clone();
                    view! {
                        <path
                            d=d
                            fill=color
                            class="cursor-pointer transition opacity-95 hover:opacity-100"
                            stroke="var(--bg-elevated)"
                            stroke-width="2"
                            on:click=move |ev| on_select.run((lbl.clone(), ev))
                        >
                            <title>{format!("{label} — {count}")}</title>
                        </path>
                    }
                }).collect_view()}
                <circle cx="80" cy="80" r="30" fill="var(--bg-elevated)"></circle>
                <text x="80" y="84" text-anchor="middle" font-size="12" font-weight="700" fill="var(--fg)">
                    {format!("{:.0}", total)}
                </text>
            </svg>
            <ul class="space-y-2 text-sm">
                {legend.into_iter().enumerate().map(|(i, s)| {
                    let bg = colors[i % colors.len()];
                    let lbl = s.label.clone();
                    view! {
                        <li>
                            <button
                                type="button"
                                class="flex items-center gap-2 rounded-lg px-2 py-1 hover:bg-[color-mix(in_srgb,var(--fg)_6%,transparent)]"
                                on:click=move |ev| on_select.run((lbl.clone(), ev))
                            >
                                <span class="inline-block h-3 w-3 rounded-sm" style=format!("background:{bg}")></span>
                                <span>{s.label} " — " {s.count}</span>
                            </button>
                        </li>
                    }
                }).collect_view()}
            </ul>
        </div>
    }
}

#[component]
fn DebtVsPaidChart(
    points: Vec<DebtVsPaidPoint>,
    #[prop(into)] on_select: Callback<(String, web_sys::MouseEvent)>,
) -> impl IntoView {
    let max = points
        .iter()
        .map(|p| p.debt_cumulative.max(p.paid_cumulative))
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let w = 560.0_f64;
    let h = 200.0_f64;
    let n = (points.len().saturating_sub(1)).max(1) as f64;
    let x0 = 48.0_f64;
    let plot_w = w - x0 - 14.0;

    let debt_poly = points
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let x = (i as f64 / n) * plot_w + x0;
            let y = h - 28.0 - (p.debt_cumulative / max) * (h - 48.0);
            format!("{x:.1},{y:.1}")
        })
        .collect::<Vec<_>>()
        .join(" ");

    let paid_poly = points
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let x = (i as f64 / n) * plot_w + x0;
            let y = h - 28.0 - (p.paid_cumulative / max) * (h - 48.0);
            format!("{x:.1},{y:.1}")
        })
        .collect::<Vec<_>>()
        .join(" ");

    let debt_area = format!(
        "{} {} {}",
        format!("{:.1},{:.1}", x0, h - 28.0),
        debt_poly,
        format!(
            "{:.1},{:.1}",
            (points.len().saturating_sub(1) as f64 / n) * plot_w + x0,
            h - 28.0
        )
    );

    let y_ticks: Vec<(f64, String)> = (0..4)
        .map(|i| {
            let t = i as f64 / 3.0;
            let y = 16.0 + t * (h - 44.0);
            let val = max * (1.0 - t);
            (y, format!("{:.0}", val))
        })
        .collect();

    view! {
        <svg viewBox=format!("0 0 {w} {h}") class="h-56 w-full" role="img" aria-label="Dette vs Payé">
            {y_ticks.into_iter().map(|(y, label)| {
                view! {
                    <g>
                        <line
                            x1="42"
                            x2=(w - 10.0).to_string()
                            y1=y.to_string()
                            y2=y.to_string()
                            stroke="color-mix(in srgb, var(--fg) 10%, transparent)"
                        ></line>
                        <text x="4" y=(y + 4.0).to_string() font-size="10" fill="var(--muted)">{label}</text>
                    </g>
                }
            }).collect_view()}
            <polygon
                points=debt_area
                fill="color-mix(in srgb, var(--brand-red) 14%, transparent)"
            ></polygon>
            <polyline
                fill="none"
                stroke="var(--brand-red)"
                stroke-width="3"
                stroke-linecap="round"
                stroke-linejoin="round"
                points=debt_poly
            ></polyline>
            <polyline
                fill="none"
                stroke="var(--brand)"
                stroke-width="3"
                stroke-linecap="round"
                stroke-linejoin="round"
                points=paid_poly
            ></polyline>
            {points.iter().enumerate().map(|(i, p)| {
                let x = (i as f64 / n) * plot_w + x0;
                let y_d = h - 28.0 - (p.debt_cumulative / max) * (h - 48.0);
                let y_p = h - 28.0 - (p.paid_cumulative / max) * (h - 48.0);
                let label = p.label.clone();
                let tip = format!(
                    "{} — dette cumulée {:.0} € · payé cumulé {:.0} €",
                    p.label, p.debt_cumulative, p.paid_cumulative
                );
                let lbl = label.clone();
                let hit_y = y_d.min(y_p) - 12.0;
                let hit_h = (y_d.max(y_p) - hit_y) + 24.0;
                view! {
                    <g class="cursor-pointer" on:click=move |ev| on_select.run((lbl.clone(), ev))>
                        <title>{tip}</title>
                        <rect
                            x=(x - 14.0).to_string()
                            y=hit_y.to_string()
                            width="28"
                            height=hit_h.max(28.0).to_string()
                            fill="transparent"
                        ></rect>
                        <circle cx=x.to_string() cy=y_d.to_string() r="5.5" fill="var(--brand-red)" stroke="var(--bg-elevated)" stroke-width="2"></circle>
                        <circle cx=x.to_string() cy=y_p.to_string() r="5.5" fill="var(--brand)" stroke="var(--bg-elevated)" stroke-width="2"></circle>
                        <text
                            x=x.to_string()
                            y=(h - 8.0).to_string()
                            text-anchor="middle"
                            font-size="11"
                            fill="var(--muted)"
                        >
                            {label.chars().take(3).collect::<String>()}
                        </text>
                    </g>
                }
            }).collect_view()}
        </svg>
        <p class="mt-2 text-xs text-[var(--muted)]">
            "Rouge = dette cumulée · Vert = payé cumulé · cliquez un point pour le détail"
        </p>
    }
}

fn donut_slice(cx: f64, cy: f64, r: f64, start_deg: f64, end_deg: f64) -> String {
    let to_rad = |d: f64| d.to_radians();
    let x1 = cx + r * to_rad(start_deg).cos();
    let y1 = cy + r * to_rad(start_deg).sin();
    let x2 = cx + r * to_rad(end_deg).cos();
    let y2 = cy + r * to_rad(end_deg).sin();
    let large = if (end_deg - start_deg).abs() > 180.0 {
        1
    } else {
        0
    };
    format!("M {cx} {cy} L {x1} {y1} A {r} {r} 0 {large} 1 {x2} {y2} Z")
}
