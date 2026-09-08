use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::app::components::ui::{Button, ButtonVariant, Modal, Select, SelectOption};
use crate::app::lib::cn;
use crate::app::lib::{
    api, AppSettings, PaymentReceipt, PrintReceiptInput, PrinterInfo, ReceiptField,
    ReceiptPrintPayload,
};

fn money(v: f64) -> String {
    format!("{v:.2} €")
}

fn to_payload(r: &PaymentReceipt, format: &str, settings: &AppSettings) -> ReceiptPrintPayload {
    ReceiptPrintPayload {
        org_name: if r.org_name.is_empty() {
            settings.org_name.clone()
        } else {
            r.org_name.clone()
        },
        org_address: if r.org_address.is_empty() {
            settings.org_address.clone()
        } else {
            r.org_address.clone()
        },
        member_name: r.member_name.clone(),
        card_number: r.card_number.clone(),
        member_uid: r.member_uid.clone(),
        period_label: r.period_label.clone(),
        amount: r.amount,
        debt_before: r.debt_before,
        balance_after: r.balance_after,
        total_paid_year: r.total_paid_year,
        year: r.year,
        format: format.to_string(),
    }
}

fn fields_to_html(fields: &[ReceiptField]) -> String {
    let mut sorted = fields.to_vec();
    sorted.sort_by(|a, b| a.position.cmp(&b.position));
    sorted
        .into_iter()
        .map(|f| {
            let mut inner = f.content.replace('&', "&amp;").replace('<', "&lt;");
            if f.bold {
                inner = format!("<b>{inner}</b>");
            }
            if f.italic {
                inner = format!("<i>{inner}</i>");
            }
            if inner.is_empty() {
                "<div><br></div>".into()
            } else {
                format!(
                    "<div style=\"font-size:{}pt;text-align:{};color:{}\">{inner}</div>",
                    f.font_size, f.align, f.color
                )
            }
        })
        .collect()
}

/// Returns HTML with placeholders replaced (template HTML kept; fields → simple divs).
fn preview_text(r: &PaymentReceipt, settings: &AppSettings, mini: bool) -> String {
    let now = js_sys::Date::new_0();
    let date = format!(
        "{:02}/{:02}/{} {:02}:{:02}",
        now.get_date(),
        now.get_month() + 1,
        now.get_full_year(),
        now.get_hours(),
        now.get_minutes()
    );
    let tpl = if mini {
        if settings
            .receipt_editor_mode_mini
            .eq_ignore_ascii_case("fields")
        {
            fields_to_html(&settings.receipt_fields_mini)
        } else {
            settings.receipt_template_mini.clone()
        }
    } else if settings
        .receipt_editor_mode_a4
        .eq_ignore_ascii_case("fields")
    {
        fields_to_html(&settings.receipt_fields_a4)
    } else {
        settings.receipt_template_a4.clone()
    };
    tpl.replace("{{ORG.NAME}}", &settings.org_name)
        .replace("{{ORG.ADDRESS}}", &settings.org_address)
        .replace("{{USER.NAME}}", &r.member_name)
        .replace("{{USER.CARD}}", &r.card_number)
        .replace("{{USER.UID}}", &r.member_uid)
        .replace("{{COTISATION.DAY}}", "01")
        .replace(
            "{{COTISATION.MONTH}}",
            &format!("{:02}", chrono_month_guess(&r.period_label)),
        )
        .replace("{{COTISATION.YEAR}}", &r.year.to_string())
        .replace("{{RECEIVED_AMOUNT}}", &money(r.amount))
        .replace("{{REMAINING_DEBT}}", &money(r.debt_before))
        .replace("{{PREVIOUS_DEBT}}", &money(r.debt_before))
        .replace("{{NEW_BALANCE}}", &money(r.total_paid_year))
        .replace("{{DATE}}", &date)
}

fn chrono_month_guess(label: &str) -> i32 {
    let l = label.to_lowercase();
    let months = [
        ("janvier", 1),
        ("février", 2),
        ("fevrier", 2),
        ("mars", 3),
        ("avril", 4),
        ("mai", 5),
        ("juin", 6),
        ("juillet", 7),
        ("août", 8),
        ("aout", 8),
        ("septembre", 9),
        ("octobre", 10),
        ("novembre", 11),
        ("décembre", 12),
        ("decembre", 12),
    ];
    for (n, m) in months {
        if l.contains(n) {
            return m;
        }
    }
    if let Some((a, _)) = label.split_once('/') {
        return a.trim().parse().unwrap_or(1);
    }
    1
}

#[component]
pub fn ReceiptDialog(receipt: RwSignal<Option<PaymentReceipt>>) -> impl IntoView {
    let mode = RwSignal::new("mini".to_string());
    let printers = RwSignal::new(Vec::<PrinterInfo>::new());
    let printer_name = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let message = RwSignal::new(Option::<String>::None);
    let error = RwSignal::new(Option::<String>::None);
    let settings = RwSignal::new(AppSettings::default());
    let open = Signal::derive(move || receipt.get().is_some());
    let preview_ref = NodeRef::<leptos::html::Div>::new();

    Effect::new(move |_| {
        if !open.get() {
            return;
        }
        error.set(None);
        message.set(None);
        spawn_local(async move {
            if let Ok(s) = api::get_settings().await {
                settings.set(s);
            }
            match api::list_printers().await {
                Ok(list) => {
                    let default = list
                        .iter()
                        .find(|p| p.is_default)
                        .or_else(|| list.first())
                        .map(|p| p.name.clone())
                        .unwrap_or_default();
                    if printer_name.get_untracked().is_empty() {
                        printer_name.set(default);
                    }
                    printers.set(list);
                }
                Err(e) => error.set(Some(e)),
            }
        });
    });

    Effect::new(move |_| {
        let current = mode.get();
        let s = settings.get();
        let html = receipt
            .get()
            .map(|r| preview_text(&r, &s, current == "mini"))
            .unwrap_or_default();
        if let Some(el) = preview_ref.get() {
            el.set_inner_html(&html);
        }
    });

    let printer_options = Signal::derive(move || {
        let mut opts: Vec<SelectOption> = printers
            .get()
            .into_iter()
            .map(|p| SelectOption {
                value: p.name.clone(),
                label: if p.is_default {
                    format!("{} (par défaut)", p.name)
                } else {
                    p.name
                },
            })
            .collect();
        if opts.is_empty() {
            opts.push(SelectOption {
                value: String::new(),
                label: "(aucune imprimante détectée)".into(),
            });
        }
        opts
    });

    let preset_label = Signal::derive(move || {
        let w = settings.get().receipt_mini_width_mm;
        if mode.get() == "mini" {
            format!("Preset : Mini(POS) · largeur {w} mm · hauteur auto (contenu)")
        } else {
            "Preset : A4 · 210 × 297 mm".to_string()
        }
    });

    view! {
        <Modal
            open=open
            title="Reçu de cotisation"
            wide=true
            on_close=Callback::new(move |_| receipt.set(None))
        >
            <div class="mb-4 flex flex-col gap-3">
                <div class="flex flex-wrap gap-2">
                    <button
                        type="button"
                        class=move || {
                            cn(&[
                                "inline-flex items-center justify-center rounded-xl px-4 py-2 text-sm font-semibold transition",
                                if mode.get() == "a4" {
                                    "bg-[var(--brand)] text-white shadow-sm"
                                } else {
                                    "border border-[var(--border)] bg-[var(--bg-elevated)]"
                                },
                            ])
                        }
                        on:click=move |_| mode.set("a4".into())
                    >
                        "Format A4"
                    </button>
                    <button
                        type="button"
                        class=move || {
                            cn(&[
                                "inline-flex items-center justify-center rounded-xl px-4 py-2 text-sm font-semibold transition",
                                if mode.get() == "mini" {
                                    "bg-[var(--brand)] text-white shadow-sm"
                                } else {
                                    "border border-[var(--border)] bg-[var(--bg-elevated)]"
                                },
                            ])
                        }
                        on:click=move |_| mode.set("mini".into())
                    >
                        "Format Mini(POS)"
                    </button>
                </div>

                <p class="rounded-lg bg-[color-mix(in_srgb,var(--brand)_10%,transparent)] px-3 py-2 text-xs font-medium">
                    {move || preset_label.get()}
                </p>

                <Select
                    label="Imprimante (liste système)"
                    options=printer_options
                    value=Signal::derive(move || printer_name.get())
                    on_change=Callback::new(move |v: String| printer_name.set(v))
                />

                <div class="flex flex-wrap gap-2">
                    <Button
                        on_click=Callback::new(move |_| {
                            if busy.get_untracked() {
                                return;
                            }
                            let Some(r) = receipt.get_untracked() else {
                                return;
                            };
                            let printer = printer_name.get_untracked();
                            if printer.is_empty() {
                                error.set(Some("Sélectionnez une imprimante.".into()));
                                return;
                            }
                            let fmt = mode.get_untracked();
                            let s = settings.get_untracked();
                            busy.set(true);
                            error.set(None);
                            message.set(None);
                            spawn_local(async move {
                                let input = PrintReceiptInput {
                                    printer_name: printer,
                                    receipt: to_payload(&r, &fmt, &s),
                                };
                                match api::print_receipt(&input).await {
                                    Ok(res) => {
                                        message.set(Some(format!(
                                            "Envoyé à « {} » — {}×{} mm",
                                            res.printer_name,
                                            res.page_width_mm,
                                            res.page_height_mm
                                        )));
                                    }
                                    Err(e) => error.set(Some(e)),
                                }
                                busy.set(false);
                            });
                        })
                    >
                        {move || {
                            if busy.get() {
                                "Impression…"
                            } else {
                                "Imprimer (native)"
                            }
                        }}
                    </Button>
                    <Button
                        variant=ButtonVariant::Secondary
                        on_click=Callback::new(move |_| {
                            let Some(r) = receipt.get_untracked() else {
                                return;
                            };
                            let fmt = mode.get_untracked();
                            let s = settings.get_untracked();
                            let default = if fmt == "mini" {
                                "recu-mini.pdf"
                            } else {
                                "recu-a4.pdf"
                            };
                            spawn_local(async move {
                                let Some(path) = api::pick_save_file(default).await else {
                                    return;
                                };
                                match api::export_receipt_pdf(&to_payload(&r, &fmt, &s), &path).await
                                {
                                    Ok(res) => {
                                        message.set(Some(format!(
                                            "PDF enregistré ({:.0}×{:.0} mm) : {}",
                                            res.page_width_mm, res.page_height_mm, res.pdf_path
                                        )));
                                    }
                                    Err(e) => error.set(Some(e)),
                                }
                            });
                        })
                    >
                        "Enregistrer PDF"
                    </Button>
                    <Button
                        variant=ButtonVariant::Ghost
                        on_click=Callback::new(move |_| {
                            spawn_local(async move {
                                match api::list_printers().await {
                                    Ok(list) => {
                                        printers.set(list);
                                        message.set(Some("Liste des imprimantes actualisée.".into()));
                                    }
                                    Err(e) => error.set(Some(e)),
                                }
                            });
                        })
                    >
                        "Actualiser imprimantes"
                    </Button>
                </div>

                <Show when=move || message.get().is_some()>
                    <p class="text-sm text-[var(--brand)]">{move || message.get().unwrap_or_default()}</p>
                </Show>
                <Show when=move || error.get().is_some()>
                    <p class="text-sm text-[var(--brand-red)]">{move || error.get().unwrap_or_default()}</p>
                </Show>
            </div>

            <div class="flex justify-center overflow-x-auto rounded-xl border border-dashed border-[var(--border)] bg-[color-mix(in_srgb,var(--fg)_4%,transparent)] p-4">
                <div
                    node_ref=preview_ref
                    class=move || {
                        if mode.get() == "mini" {
                            "receipt-screen-preview receipt-screen-mini bg-white p-2 text-[11px] leading-snug text-black shadow-md"
                        } else {
                            "receipt-screen-preview receipt-screen-a4 bg-white p-5 text-[14px] leading-relaxed text-black shadow-md"
                        }
                    }
                    style=move || {
                        if mode.get() == "mini" {
                            format!(
                                "width:{}mm;max-width:100%",
                                settings.get().receipt_mini_width_mm
                            )
                        } else {
                            "max-width:210mm".into()
                        }
                    }
                />
            </div>
        </Modal>
    }
}
