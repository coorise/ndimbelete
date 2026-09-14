//! Contenteditable rich-text editor for receipt templates.

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

const PLACEHOLDERS: &[(&str, &str)] = &[
    ("{{ORG.NAME}}", "Organisation"),
    ("{{ORG.ADDRESS}}", "Adresse"),
    ("{{USER.NAME}}", "Membre"),
    ("{{USER.CARD}}", "N° carte"),
    ("{{DATE}}", "Date/heure"),
    ("{{PAYMENT_DATE}}", "Date de paiement"),
    ("{{COTISATION.YEAR}}", "Année"),
    ("{{YEAR_TOTAL_DUE}}", "Montant total année"),
    ("{{PAYMENT_METHOD}}", "Mode de paiement"),
    ("{{RECEIVED_AMOUNT}}", "Montant reçu"),
    ("{{REMAINING_DEBT}}", "Restant à payer"),
    ("{{SURPLUS_LINE}}", "Ligne surplus"),
    ("{{NEW_BALANCE}}", "Nouveau solde (crédits)"),
];

fn exec(cmd: &str, value: Option<&str>) {
    let script = match value {
        Some(v) => {
            let escaped = v.replace('\\', "\\\\").replace('\'', "\\'");
            format!("document.execCommand('{cmd}', false, '{escaped}')")
        }
        None => format!("document.execCommand('{cmd}')"),
    };
    let _ = js_sys::eval(&script);
}

fn insert_html(html: &str) {
    let escaped = html.replace('\\', "\\\\").replace('\'', "\\'");
    let _ = js_sys::eval(&format!(
        "document.execCommand('insertHTML', false, '{escaped}')"
    ));
}

#[component]
pub fn RichTextEditor(
    /// HTML content
    value: Signal<String>,
    on_change: Callback<String>,
    /// When false, color controls are hidden (Mini POS).
    allow_color: Signal<bool>,
) -> impl IntoView {
    let editor_ref = NodeRef::<leptos::html::Div>::new();
    let synced = RwSignal::new(false);

    Effect::new(move |_| {
        let html = value.get();
        if let Some(el) = editor_ref.get() {
            // Only push from outside when not focused, or first paint.
            let focused = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.active_element())
                .and_then(|a| a.dyn_into::<HtmlElement>().ok())
                .map(|a| a == *el)
                .unwrap_or(false);
            if !synced.get_untracked() || !focused {
                el.set_inner_html(&html);
                synced.set(true);
            }
        }
    });

    let emit = move || {
        if let Some(el) = editor_ref.get_untracked() {
            on_change.run(el.inner_html());
        }
    };

    let btn = "tap-target inline-flex h-8 min-w-8 items-center justify-center rounded-md border border-[var(--border)] bg-white px-2 text-sm font-semibold hover:bg-[var(--bg-elevated)]";

    view! {
        <div class="overflow-hidden rounded-xl border border-[var(--border)] bg-[var(--surface)]">
            <div class="flex flex-wrap items-center gap-1 border-b border-[var(--border)] bg-[color-mix(in_srgb,var(--fg)_3%,transparent)] p-2">
                <button type="button" class=btn style="font-weight:700" on:mousedown=move |ev| {
                    ev.prevent_default();
                    exec("bold", None);
                    emit();
                }>"B"</button>
                <button type="button" class=btn style="font-style:italic" on:mousedown=move |ev| {
                    ev.prevent_default();
                    exec("italic", None);
                    emit();
                }>"I"</button>
                <button type="button" class=btn style="text-decoration:underline" on:mousedown=move |ev| {
                    ev.prevent_default();
                    exec("underline", None);
                    emit();
                }>"U"</button>
                <span class="mx-1 h-5 w-px bg-[var(--border)]"></span>
                <select
                    class="h-8 rounded-md border border-[var(--border)] bg-white px-2 text-sm"
                    on:change=move |ev| {
                        let v = event_target_value(&ev);
                        exec("fontSize", Some(&v));
                        emit();
                    }
                >
                    <option value="2">"10 pt"</option>
                    <option value="3" selected>"12 pt"</option>
                    <option value="4">"14 pt"</option>
                    <option value="5">"18 pt"</option>
                </select>
                <span class="mx-1 h-5 w-px bg-[var(--border)]"></span>
                <button type="button" class=btn on:mousedown=move |ev| {
                    ev.prevent_default();
                    exec("justifyLeft", None);
                    emit();
                }>"⟸"</button>
                <button type="button" class=btn on:mousedown=move |ev| {
                    ev.prevent_default();
                    exec("justifyCenter", None);
                    emit();
                }>"≡"</button>
                <button type="button" class=btn on:mousedown=move |ev| {
                    ev.prevent_default();
                    exec("justifyRight", None);
                    emit();
                }>"⟹"</button>
                <Show when=move || allow_color.get()>
                    <span class="mx-1 h-5 w-px bg-[var(--border)]"></span>
                    <label class="flex items-center gap-1 text-xs font-medium">
                        "Couleur"
                        <input
                            type="color"
                            value="#000000"
                            class="h-8 w-10 cursor-pointer rounded border border-[var(--border)] bg-white"
                            on:input=move |ev| {
                                let v = event_target_value(&ev);
                                exec("foreColor", Some(&v));
                                emit();
                            }
                        />
                    </label>
                </Show>
                <Show when=move || !allow_color.get()>
                    <span class="ml-2 text-xs text-[var(--muted)]">"Mini : noir & blanc uniquement"</span>
                </Show>
                <span class="mx-1 h-5 w-px bg-[var(--border)]"></span>
                <select
                    class="h-8 max-w-[180px] rounded-md border border-[var(--border)] bg-white px-2 text-sm"
                    on:change=move |ev| {
                        let v = event_target_value(&ev);
                        if !v.is_empty() {
                            insert_html(&v);
                            emit();
                        }
                        if let Some(t) = ev.target() {
                            if let Ok(sel) = t.dyn_into::<web_sys::HtmlSelectElement>() {
                                sel.set_value("");
                            }
                        }
                    }
                >
                    <option value="">"+ Placeholder…"</option>
                    {PLACEHOLDERS.iter().map(|(ph, label)| {
                        view! { <option value=*ph>{format!("{label} ({ph})")}</option> }
                    }).collect_view()}
                </select>
            </div>
            <div
                node_ref=editor_ref
                class="min-h-[220px] max-h-[420px] overflow-y-auto bg-white p-4 text-sm leading-relaxed text-black outline-none"
                contenteditable="true"
                on:input=move |_ev| emit()
                on:blur=move |_| emit()
            ></div>
        </div>
    }
}
