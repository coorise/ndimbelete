use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::app::components::ui::{
    Button, ButtonVariant, Input, Modal, SearchableSelect, Select, SelectOption,
};
use crate::app::lib::{api, Member, MemberDebtSummary, PaymentReceipt};

fn money(v: f64) -> String {
    format!("{v:.2} €")
}

#[component]
pub fn PaymentModal(
    open: RwSignal<bool>,
    year: Signal<i32>,
    /// Fallback / default period when creating a new payment.
    current_period_id: Signal<String>,
    current_period_label: Signal<String>,
    current_period_month: Signal<i32>,
    /// All periods for the year — enables month choice in edit mode.
    #[prop(optional)]
    period_options: Option<Signal<Vec<SelectOption>>>,
    members: Signal<Vec<Member>>,
    org_name: Signal<String>,
    #[prop(optional)] prefill_member_id: Option<RwSignal<String>>,
    #[prop(into)] on_paid: Callback<PaymentReceipt>,
) -> impl IntoView {
    let member_id = RwSignal::new(String::new());
    let amount = RwSignal::new(String::new());
    let debt = RwSignal::new(Option::<MemberDebtSummary>::None);
    let error = RwSignal::new(Option::<String>::None);
    let busy = RwSignal::new(false);
    let selected_period_id = RwSignal::new(String::new());

    let is_edit = Signal::derive(move || {
        prefill_member_id
            .map(|s| !s.get().is_empty())
            .unwrap_or(false)
    });

    Effect::new(move |_| {
        if !open.get() {
            return;
        }
        // Reset amount so debt effect can prefill.
        amount.set(String::new());
        if let Some(sig) = prefill_member_id {
            let v = sig.get();
            if !v.is_empty() {
                member_id.set(v);
            }
        }
        let cur = current_period_id.get();
        if !cur.is_empty() {
            selected_period_id.set(cur);
        }
    });

    let active_period_id = Signal::derive(move || {
        let sel = selected_period_id.get();
        if !sel.is_empty() {
            sel
        } else {
            current_period_id.get()
        }
    });

    let active_period_label = Signal::derive(move || {
        if let Some(opts) = period_options {
            let id = active_period_id.get();
            if let Some(o) = opts.get().into_iter().find(|o| o.value == id) {
                return o.label;
            }
        }
        current_period_label.get()
    });

    let active_period_month = Signal::derive(move || {
        if let Some(opts) = period_options {
            let id = active_period_id.get();
            // Labels like "Septembre (9)" or plain "Septembre" — prefer current_period_month fallback.
            if let Some(o) = opts.get().into_iter().find(|o| o.value == id) {
                if let Some(start) = o.label.rfind('(') {
                    if let Some(end) = o.label.rfind(')') {
                        if let Ok(m) = o.label[start + 1..end].parse::<i32>() {
                            return m;
                        }
                    }
                }
            }
        }
        current_period_month.get()
    });

    Effect::new(move |_| {
        let mid = member_id.get();
        let y = year.get();
        let as_of = active_period_month.get();
        let is_open = open.get();
        if mid.is_empty() || !is_open {
            debt.set(None);
            return;
        }
        spawn_local(async move {
            match api::get_member_debt_as_of(&mid, y, as_of).await {
                Ok(d) => {
                    debt.set(Some(d.clone()));
                    if amount.get_untracked().is_empty() {
                        amount.set(format!("{:.2}", d.balance.max(0.0)));
                    }
                }
                Err(e) => error.set(Some(e)),
            }
        });
    });

    let member_options = Signal::derive(move || {
        members
            .get()
            .into_iter()
            .map(|m| SelectOption {
                value: m.id.clone(),
                label: format!("{} {} ({})", m.last_name, m.first_name, m.card_number),
            })
            .collect::<Vec<_>>()
    });

    let periods_signal = period_options.unwrap_or_else(|| {
        Signal::derive(move || {
            let id = current_period_id.get();
            let label = current_period_label.get();
            if id.is_empty() {
                Vec::new()
            } else {
                vec![SelectOption {
                    value: id,
                    label,
                }]
            }
        })
    });

    view! {
        <Modal
            open=open.into()
            title_signal=Signal::derive(move || {
                if is_edit.get() {
                    "Mettre à jour la cotisation".into()
                } else {
                    "Nouvelle cotisation".into()
                }
            })
            on_close=Callback::new(move |_| open.set(false))
        >
            <div class="flex flex-col gap-4">
                <SearchableSelect
                    label="Membre"
                    options=member_options
                    value=member_id.into()
                    on_change=Callback::new(move |v| {
                        amount.set(String::new());
                        member_id.set(v);
                    })
                    class="w-full"
                    placeholder="Choisir un membre…"
                    search_placeholder="Nom ou n° carte…"
                />
                <Show
                    when=move || is_edit.get() && !periods_signal.get().is_empty()
                    fallback=move || {
                        view! {
                            <div class="rounded-xl border border-[var(--border)] bg-[var(--bg-elevated)] px-3 py-2 text-sm">
                                <span class="text-[var(--muted)]">"Période en cours : "</span>
                                <strong>{move || active_period_label.get()}</strong>
                            </div>
                        }
                    }
                >
                    <Select
                        label="Choix du mois"
                        options=periods_signal
                        value=selected_period_id.into()
                        on_change=Callback::new(move |v| {
                            amount.set(String::new());
                            selected_period_id.set(v);
                        })
                    />
                </Show>
                <div class="rounded-xl border border-[var(--border)] bg-[color-mix(in_srgb,var(--brand-yellow)_18%,transparent)] p-3 text-sm">
                    {move || {
                        match debt.get() {
                            Some(d) => view! {
                                <p>"Dette accumulée : " <strong>{money(d.balance.max(0.0))}</strong></p>
                                <p class="text-[var(--muted)]">
                                    "Dû " {money(d.total_due)} " — Payé " {money(d.total_paid)}
                                </p>
                            }.into_any(),
                            None => view! {
                                <p>"Dette accumulée : " <strong>"0.00 €"</strong></p>
                                <p class="text-[var(--muted)]">
                                    {if member_id.get().is_empty() {
                                        "Sélectionnez un membre pour calculer la dette."
                                    } else {
                                        "Chargement de la dette…"
                                    }}
                                </p>
                            }.into_any(),
                        }
                    }}
                </div>
                <Input
                    label="Montant à encaisser (€)"
                    r#type="number"
                    value=amount.into()
                    on_input=Callback::new(move |v| amount.set(v))
                />
                <Show when=move || error.get().is_some()>
                    <p class="text-[var(--brand-red)]">{move || error.get().unwrap_or_default()}</p>
                </Show>
                <div class="flex justify-end gap-2">
                    <Button variant=ButtonVariant::Secondary on_click=Callback::new(move |_| open.set(false))>
                        "Annuler"
                    </Button>
                    <Button
                        on_click=Callback::new(move |_| {
                            if busy.get_untracked() {
                                return;
                            }
                            error.set(None);
                            let mid = member_id.get_untracked();
                            let pid = active_period_id.get_untracked();
                            let amt: f64 = amount
                                .get_untracked()
                                .replace(',', ".")
                                .parse()
                                .unwrap_or(0.0);
                            if mid.is_empty() || pid.is_empty() || amt <= 0.0 {
                                error.set(Some(
                                    "Sélectionnez un membre, un mois et un montant.".into(),
                                ));
                                return;
                            }
                            busy.set(true);
                            let y = year.get_untracked();
                            let members_snap = members.get_untracked();
                            let period_label = active_period_label.get_untracked();
                            let org = org_name.get_untracked();
                            let debt_before = debt
                                .get_untracked()
                                .map(|d| d.balance.max(0.0))
                                .unwrap_or(amt);
                            spawn_local(async move {
                                match api::record_payment(&mid, &pid, amt).await {
                                    Ok(summary) => {
                                        let m = members_snap.iter().find(|m| m.id == mid);
                                        let receipt = PaymentReceipt {
                                            member_name: m
                                                .map(|m| {
                                                    format!("{} {}", m.last_name, m.first_name)
                                                })
                                                .unwrap_or_default(),
                                            card_number: m
                                                .map(|m| m.card_number.clone())
                                                .unwrap_or_default(),
                                            member_uid: mid.clone(),
                                            period_label,
                                            amount: amt,
                                            debt_before,
                                            balance_after: summary.balance,
                                            total_paid_year: summary.total_paid,
                                            year: y,
                                            org_name: org,
                                            org_address: String::new(),
                                        };
                                        open.set(false);
                                        member_id.set(String::new());
                                        amount.set(String::new());
                                        debt.set(None);
                                        if let Some(sig) = prefill_member_id {
                                            sig.set(String::new());
                                        }
                                        on_paid.run(receipt);
                                    }
                                    Err(e) => error.set(Some(e)),
                                }
                                busy.set(false);
                            });
                        })
                    >
                        {move || if busy.get() { "Enregistrement…" } else { "Confirmer" }}
                    </Button>
                </div>
            </div>
        </Modal>
    }
}
