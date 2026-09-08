use leptos::prelude::*;

use crate::app::lib::cn;

#[component]
pub fn Switch(
    checked: Signal<bool>,
    #[prop(into)] on_change: Callback<bool>,
    #[prop(optional)] label: &'static str,
) -> impl IntoView {
    view! {
        <button
            type="button"
            role="switch"
            class="tap-target inline-flex items-center gap-3"
            aria-checked=move || checked.get().to_string()
            on:click=move |_| on_change.run(!checked.get_untracked())
        >
            <span class=move || {
                cn(&[
                    "relative h-7 w-12 rounded-full transition",
                    if checked.get() {
                        "bg-[var(--brand)]"
                    } else {
                        "bg-[var(--border)]"
                    },
                ])
            }>
                <span class=move || {
                    cn(&[
                        "absolute top-0.5 h-6 w-6 rounded-full bg-white shadow transition",
                        if checked.get() { "left-5" } else { "left-0.5" },
                    ])
                }></span>
            </span>
            {(!label.is_empty()).then(|| view! { <span class="text-base font-medium">{label}</span> })}
        </button>
    }
}
