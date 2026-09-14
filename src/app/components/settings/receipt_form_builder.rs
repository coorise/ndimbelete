//! Visual form-builder canvas for receipt field lines (drag reorder + palette).

use leptos::prelude::*;
use web_sys::DragEvent;

use crate::app::lib::ReceiptField;

const PALETTE: &[(&str, &str, &str)] = &[
    ("{{ORG.NAME}}", "Organisation", "org"),
    ("{{ORG.ADDRESS}}", "Adresse org.", "addr"),
    ("{{USER.NAME}}", "Nom membre", "user"),
    ("{{USER.CARD}}", "N° carte", "card"),
    ("{{DATE}}", "Date / heure", "date"),
    ("Date de paiement : {{PAYMENT_DATE}}", "Date de paiement", "paydate"),
    ("Montant total pour {{COTISATION.YEAR}} : {{YEAR_TOTAL_DUE}}",
        "Montant total année",
        "yeartotal",
    ),
    (
        "Mode de paiement : {{PAYMENT_METHOD}}",
        "Mode de paiement",
        "paymethod",
    ),
    ("Montant Reçu : {{RECEIVED_AMOUNT}}", "Montant reçu", "amount"),
    (
        "Restant à payer pour {{COTISATION.YEAR}} : {{REMAINING_DEBT}}",
        "Restant à payer",
        "debt",
    ),
    ("{{SURPLUS_LINE}}", "Surplus reçus (auto)", "surplus"),
    ("Nouveau Solde : {{NEW_BALANCE}}", "Nouveau solde (crédits année)", "balance"),
    ("Merci pour votre solidarité", "Message merci", "thanks"),
    ("NDIMBELENTÉ", "Pied de page", "footer"),
    ("------------------------------", "Séparateur", "sep"),
    ("", "Ligne vide", "blank"),
    ("Texte libre…", "Texte libre", "custom"),
];

fn new_id() -> String {
    format!("f-{}", js_sys::Date::now() as u64)
}

fn sorted_fields(fields: &[ReceiptField]) -> Vec<ReceiptField> {
    let mut f = fields.to_vec();
    f.sort_by(|a, b| a.position.cmp(&b.position));
    f
}

fn renumber(fields: &mut [ReceiptField]) {
    for (i, f) in fields.iter_mut().enumerate() {
        f.position = i as i32;
    }
}

#[component]
pub fn ReceiptFormBuilder(
    fields: Signal<Vec<ReceiptField>>,
    on_change: Callback<Vec<ReceiptField>>,
    allow_color: Signal<bool>,
) -> impl IntoView {
    let selected = RwSignal::new(Option::<String>::None);
    let drag_from = RwSignal::new(Option::<usize>::None);

    let apply = Callback::new(move |mut list: Vec<ReceiptField>| {
        renumber(&mut list);
        on_change.run(list);
    });

    let add_from_palette = move |content: &str, label_hint: &str| {
        let mut list = sorted_fields(&fields.get_untracked());
        let content = if content == "Texte libre…" {
            String::new()
        } else {
            content.to_string()
        };
        let bold = label_hint.contains("Montant") || content.contains("ORG.NAME");
        list.push(ReceiptField {
            id: new_id(),
            content,
            position: list.len() as i32,
            font_size: if bold { 12.0 } else { 10.0 },
            bold,
            italic: false,
            align: "left".into(),
            color: "#000000".into(),
        });
        let id = list.last().map(|f| f.id.clone());
        apply.run(list);
        selected.set(id);
    };

    view! {
        <div class="grid gap-4 lg:grid-cols-[220px_1fr]">
            // Palette
            <aside class="rounded-xl border border-[var(--border)] bg-[color-mix(in_srgb,var(--fg)_3%,transparent)] p-3">
                <p class="mb-2 text-xs font-bold uppercase tracking-wide text-[var(--muted)]">
                    "Éléments"
                </p>
                <p class="mb-3 text-[11px] text-[var(--muted)]">
                    "Cliquez pour ajouter · glissez les blocs pour réordonner."
                </p>
                <div class="flex flex-col gap-1.5">
                    {PALETTE.iter().map(|(content, label, _key)| {
                        let c_click = (*content).to_string();
                        let c_drag = (*content).to_string();
                        let l_click = (*label).to_string();
                        let l_show = (*label).to_string();
                        view! {
                            <button
                                type="button"
                                class="tap-target flex items-center gap-2 rounded-lg border border-[var(--border)] bg-white px-2.5 py-2 text-left text-xs font-medium shadow-sm hover:border-[var(--brand)]"
                                draggable="true"
                                on:click=move |_| add_from_palette(&c_click, &l_click)
                                on:dragstart=move |ev: DragEvent| {
                                    if let Some(dt) = ev.data_transfer() {
                                        let _ = dt.set_data("text/plain", &format!("palette:{c_drag}"));
                                    }
                                }
                            >
                                <span class="text-[var(--muted)]">"⠿"</span>
                                <span>{l_show}</span>
                            </button>
                        }
                    }).collect_view()}
                </div>
            </aside>

            // Canvas
            <div class="flex flex-col gap-3">
                <p class="text-sm font-semibold">"Canvas du reçu"</p>
                <div
                    class="min-h-[280px] rounded-xl border border-dashed border-[var(--border)] bg-[color-mix(in_srgb,var(--fg)_2%,white)] p-3"
                    on:dragover=move |ev: DragEvent| {
                        ev.prevent_default();
                    }
                    on:drop=move |ev: DragEvent| {
                        ev.prevent_default();
                        if let Some(dt) = ev.data_transfer() {
                            if let Ok(data) = dt.get_data("text/plain") {
                                if let Some(rest) = data.strip_prefix("palette:") {
                                    add_from_palette(rest, rest);
                                }
                            }
                        }
                    }
                >
                    <Show when=move || fields.get().is_empty()>
                        <p class="py-10 text-center text-sm text-[var(--muted)]">
                            "Glissez ou ajoutez des éléments depuis la palette."
                        </p>
                    </Show>
                    <For
                        each=move || sorted_fields(&fields.get())
                        key=|f| f.id.clone()
                        children=move |field: ReceiptField| {
                            let fid = field.id.clone();
                            let fid_sel = field.id.clone();
                            let fid_del = field.id.clone();
                            let fid_drag = field.id.clone();
                            let fid_drop = field.id.clone();
                            let is_sel = Signal::derive({
                                let fid = fid.clone();
                                move || selected.get().as_ref() == Some(&fid)
                            });
                            let preview = if field.content.is_empty() {
                                "(ligne vide)".to_string()
                            } else {
                                field.content.clone()
                            };
                            view! {
                                <div
                                    class=move || {
                                        if is_sel.get() {
                                            "mb-2 flex cursor-grab items-start gap-2 rounded-xl border-2 border-[var(--brand)] bg-white p-3 shadow-sm"
                                        } else {
                                            "mb-2 flex cursor-grab items-start gap-2 rounded-xl border border-[var(--border)] bg-white p-3 shadow-sm"
                                        }
                                    }
                                    draggable="true"
                                    on:click=move |_| selected.set(Some(fid_sel.clone()))
                                    on:dragstart=move |ev: DragEvent| {
                                        let list = sorted_fields(&fields.get_untracked());
                                        if let Some(idx) = list.iter().position(|f| f.id == fid_drag) {
                                            drag_from.set(Some(idx));
                                            if let Some(dt) = ev.data_transfer() {
                                                let _ = dt.set_data("text/plain", &format!("move:{idx}"));
                                            }
                                        }
                                    }
                                    on:dragover=move |ev: DragEvent| ev.prevent_default()
                                    on:drop=move |ev: DragEvent| {
                                        ev.prevent_default();
                                        ev.stop_propagation();
                                        let list = sorted_fields(&fields.get_untracked());
                                        let Some(to) = list.iter().position(|f| f.id == fid_drop) else { return };
                                        let from = drag_from.get_untracked().or_else(|| {
                                            ev.data_transfer()
                                                .and_then(|dt| dt.get_data("text/plain").ok())
                                                .and_then(|d| d.strip_prefix("move:").map(|s| s.to_string()))
                                                .and_then(|s| s.parse().ok())
                                        });
                                        let Some(from) = from else { return };
                                        if from == to { return; }
                                        let mut list = list;
                                        let item = list.remove(from);
                                        let insert_at = if from < to { to } else { to };
                                        list.insert(insert_at.min(list.len()), item);
                                        apply.run(list);
                                        drag_from.set(None);
                                    }
                                >
                                    <span class="mt-1 select-none text-[var(--muted)]">"⠿"</span>
                                    <div class="min-w-0 flex-1">
                                        <p class="truncate text-sm font-semibold text-black">{preview}</p>
                                        <p class="mt-0.5 text-[11px] text-[var(--muted)]">
                                            {format!(
                                                "pos {} · {} pt{}",
                                                field.position,
                                                field.font_size,
                                                if field.bold { " · gras" } else { "" }
                                            )}
                                        </p>
                                    </div>
                                    <button
                                        type="button"
                                        class="text-xs text-[var(--brand-red)]"
                                        on:click=move |ev| {
                                            ev.stop_propagation();
                                            let mut list = sorted_fields(&fields.get_untracked());
                                            list.retain(|f| f.id != fid_del);
                                            apply.run(list);
                                            selected.set(None);
                                        }
                                    >
                                        "Suppr."
                                    </button>
                                </div>
                            }
                        }
                    />
                </div>

                // Inspector
                {move || {
                    let sel_id = selected.get()?;
                    let list = sorted_fields(&fields.get());
                    let field = list.iter().find(|f| f.id == sel_id)?.clone();
                    let fid = field.id.clone();
                    let content_val = field.content.clone();
                    let align_val = field.align.clone();
                    let color_val = field.color.clone();
                    let font_size_val = field.font_size;
                    let bold_val = field.bold;
                    let italic_val = field.italic;
                    let fid1 = fid.clone();
                    let fid2 = fid.clone();
                    let fid3 = fid.clone();
                    let fid4 = fid.clone();
                    let fid5 = fid.clone();
                    let fid6 = fid.clone();
                    let fid7 = fid.clone();
                    let fid8 = fid.clone();
                    Some(view! {
                        <div class="rounded-xl border border-[var(--border)] bg-[var(--surface)] p-4">
                            <p class="mb-3 text-sm font-semibold">"Propriétés du bloc"</p>
                            <div class="grid gap-3 sm:grid-cols-2">
                                <label class="flex flex-col gap-1 text-xs font-medium sm:col-span-2">
                                    "Contenu / template"
                                    <input
                                        class="tap-target rounded-lg border border-[var(--border)] px-3 py-2 text-sm"
                                        prop:value=content_val
                                        on:input=move |ev| {
                                            let v = event_target_value(&ev);
                                            let mut list = sorted_fields(&fields.get_untracked());
                                            if let Some(f) = list.iter_mut().find(|f| f.id == fid1) {
                                                f.content = v;
                                            }
                                            apply.run(list);
                                        }
                                    />
                                </label>
                                <label class="flex flex-col gap-1 text-xs font-medium">
                                    "Taille (pt)"
                                    <input
                                        type="number"
                                        min="7"
                                        max="24"
                                        class="tap-target rounded-lg border border-[var(--border)] px-3 py-2 text-sm"
                                        prop:value=font_size_val
                                        on:input=move |ev| {
                                            if let Ok(n) = event_target_value(&ev).parse::<f64>() {
                                                let mut list = sorted_fields(&fields.get_untracked());
                                                if let Some(f) = list.iter_mut().find(|f| f.id == fid2) {
                                                    f.font_size = n;
                                                }
                                                apply.run(list);
                                            }
                                        }
                                    />
                                </label>
                                <label class="flex flex-col gap-1 text-xs font-medium">
                                    "Alignement"
                                    <select
                                        class="tap-target rounded-lg border border-[var(--border)] px-3 py-2 text-sm"
                                        prop:value=align_val
                                        on:change=move |ev| {
                                            let v = event_target_value(&ev);
                                            let mut list = sorted_fields(&fields.get_untracked());
                                            if let Some(f) = list.iter_mut().find(|f| f.id == fid3) {
                                                f.align = v;
                                            }
                                            apply.run(list);
                                        }
                                    >
                                        <option value="left">"Gauche"</option>
                                        <option value="center">"Centre"</option>
                                        <option value="right">"Droite"</option>
                                    </select>
                                </label>
                                <label class="flex items-center gap-2 text-sm">
                                    <input
                                        type="checkbox"
                                        prop:checked=bold_val
                                        on:change=move |ev| {
                                            let checked = event_target_checked(&ev);
                                            let mut list = sorted_fields(&fields.get_untracked());
                                            if let Some(f) = list.iter_mut().find(|f| f.id == fid4) {
                                                f.bold = checked;
                                            }
                                            apply.run(list);
                                        }
                                    />
                                    "Gras"
                                </label>
                                <label class="flex items-center gap-2 text-sm">
                                    <input
                                        type="checkbox"
                                        prop:checked=italic_val
                                        on:change=move |ev| {
                                            let checked = event_target_checked(&ev);
                                            let mut list = sorted_fields(&fields.get_untracked());
                                            if let Some(f) = list.iter_mut().find(|f| f.id == fid5) {
                                                f.italic = checked;
                                            }
                                            apply.run(list);
                                        }
                                    />
                                    "Italique"
                                </label>
                                <Show when=move || allow_color.get()
                                    fallback=|| ()
                                >
                                    {
                                        let fid_color = fid6.clone();
                                        let cv = color_val.clone();
                                        view! {
                                            <label class="flex flex-col gap-1 text-xs font-medium">
                                                "Couleur (A4)"
                                                <input
                                                    type="color"
                                                    class="h-10 w-full cursor-pointer rounded border border-[var(--border)]"
                                                    prop:value=cv
                                                    on:input=move |ev| {
                                                        let v = event_target_value(&ev);
                                                        let mut list = sorted_fields(&fields.get_untracked());
                                                        if let Some(f) = list.iter_mut().find(|f| f.id == fid_color) {
                                                            f.color = v;
                                                        }
                                                        apply.run(list);
                                                    }
                                                />
                                            </label>
                                        }
                                    }
                                </Show>
                            </div>
                            <div class="mt-3 flex gap-2">
                                <button
                                    type="button"
                                    class="rounded-lg border border-[var(--border)] px-3 py-1.5 text-xs font-semibold"
                                    on:click=move |_| {
                                        let mut list = sorted_fields(&fields.get_untracked());
                                        if let Some(i) = list.iter().position(|f| f.id == fid7) {
                                            if i > 0 {
                                                list.swap(i, i - 1);
                                                apply.run(list);
                                            }
                                        }
                                    }
                                >
                                    "↑ Monter"
                                </button>
                                <button
                                    type="button"
                                    class="rounded-lg border border-[var(--border)] px-3 py-1.5 text-xs font-semibold"
                                    on:click=move |_| {
                                        let mut list = sorted_fields(&fields.get_untracked());
                                        if let Some(i) = list.iter().position(|f| f.id == fid8) {
                                            if i + 1 < list.len() {
                                                list.swap(i, i + 1);
                                                apply.run(list);
                                            }
                                        }
                                    }
                                >
                                    "↓ Descendre"
                                </button>
                            </div>
                        </div>
                    })
                }}
            </div>
        </div>
    }
}
