use leptos::prelude::*;

use crate::app::lib::cn;

#[derive(Clone, PartialEq, Eq)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

#[component]
pub fn Select(
    #[prop(optional)] label: &'static str,
    options: Signal<Vec<SelectOption>>,
    value: Signal<String>,
    #[prop(into)] on_change: Callback<String>,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    view! {
        <label class=cn(&[
            "flex min-w-0 flex-col gap-1.5 text-sm font-medium",
            if class.is_empty() { "w-auto" } else { class },
        ])>
            {(!label.is_empty()).then(|| view! { <span>{label}</span> })}
            <select
                class="tap-target w-full rounded-xl border border-[var(--border)] bg-[var(--bg-elevated)] px-3 py-2.5 text-base focus:border-[var(--brand)] focus:outline-none focus:ring-2 focus:ring-[color-mix(in_srgb,var(--brand)_35%,transparent)]"
                prop:value=move || value.get()
                on:change=move |ev| on_change.run(event_target_value(&ev))
            >
            <For
                each=move || options.get()
                key=|o| o.value.clone()
                children=move |o| {
                    let val = o.value.clone();
                    let val_selected = o.value.clone();
                    view! {
                        <option
                            value=val.clone()
                            selected=move || value.get() == val_selected
                        >
                            {o.label}
                        </option>
                    }
                }
            />
            </select>
        </label>
    }
}
