use leptos::prelude::*;

use crate::app::i18n::use_i18n;
use crate::app::lib::cn;

#[component]
pub fn Input(
    #[prop(optional)] id: &'static str,
    #[prop(optional)] label: &'static str,
    /// When set, resolves via i18n and takes precedence over `label`.
    #[prop(optional)] label_key: &'static str,
    #[prop(optional)] r#type: &'static str,
    #[prop(optional)] placeholder: &'static str,
    #[prop(optional)] placeholder_key: &'static str,
    #[prop(optional)] value: Option<Signal<String>>,
    #[prop(optional)] on_input: Option<Callback<String>>,
    #[prop(optional)] disabled: bool,
    /// Reactive disabled override (takes precedence when set).
    #[prop(optional)] disabled_signal: Option<Signal<bool>>,
    #[prop(optional)] class: &'static str,
    #[prop(optional)] required: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let ty = if r#type.is_empty() { "text" } else { r#type };
    let input_cls = cn(&[
        "tap-target w-full rounded-xl border border-[var(--border)] bg-[var(--bg-elevated)] px-3 py-2.5 text-base text-[var(--fg)] placeholder:text-[var(--muted)] focus:border-[var(--brand)] focus:outline-none focus:ring-2 focus:ring-[color-mix(in_srgb,var(--brand)_35%,transparent)]",
        class,
    ]);
    let has_label = !label_key.is_empty() || !label.is_empty();

    view! {
        <label class="flex w-full flex-col gap-1.5 text-sm font-medium text-[var(--fg)]">
            <Show when=move || has_label>
                <span>
                    {move || {
                        if !label_key.is_empty() {
                            i18n.t(label_key)
                        } else {
                            label.to_string()
                        }
                    }}
                </span>
            </Show>
            <input
                id=id
                type=ty
                class=input_cls
                placeholder=move || {
                    if !placeholder_key.is_empty() {
                        i18n.t(placeholder_key)
                    } else {
                        placeholder.to_string()
                    }
                }
                prop:disabled=move || {
                    disabled_signal
                        .map(|s| s.get())
                        .unwrap_or(disabled)
                }
                prop:required=required
                prop:value=move || value.map(|v| v.get()).unwrap_or_default()
                on:input=move |ev| {
                    let v = event_target_value(&ev);
                    if let Some(cb) = on_input {
                        cb.run(v);
                    }
                }
            />
        </label>
    }
}

#[component]
pub fn TextArea(
    #[prop(optional)] label: &'static str,
    #[prop(optional)] placeholder: &'static str,
    #[prop(optional)] value: Option<Signal<String>>,
    #[prop(optional)] on_input: Option<Callback<String>>,
    #[prop(optional)] rows: u32,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let r = if rows == 0 { 3 } else { rows };
    view! {
        <label class="flex w-full flex-col gap-1.5 text-sm font-medium text-[var(--fg)]">
            {(!label.is_empty()).then(|| view! { <span>{label}</span> })}
            <textarea
                class=cn(&[
                    "w-full rounded-xl border border-[var(--border)] bg-[var(--bg-elevated)] px-3 py-2.5 text-base text-[var(--fg)] focus:border-[var(--brand)] focus:outline-none focus:ring-2 focus:ring-[color-mix(in_srgb,var(--brand)_35%,transparent)]",
                    class,
                ])
                placeholder=placeholder
                rows=r
                prop:value=move || value.map(|v| v.get()).unwrap_or_default()
                on:input=move |ev| {
                    let v = event_target_value(&ev);
                    if let Some(cb) = on_input {
                        cb.run(v);
                    }
                }
            />
        </label>
    }
}
