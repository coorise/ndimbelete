use leptos::prelude::*;

use crate::app::lib::cn;

#[component]
pub fn Progress(
    /// 0.0 – 1.0
    value: Signal<f64>,
    #[prop(optional)] label: &'static str,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    view! {
        <div class=cn(&["w-full", class])>
            {(!label.is_empty()).then(|| {
                view! {
                    <div class="mb-1 flex justify-between text-sm text-[var(--muted)]">
                        <span>{label}</span>
                        <span>{move || format!("{:.0}%", value.get() * 100.0)}</span>
                    </div>
                }
            })}
            <div
                class="h-3 overflow-hidden rounded-full bg-[color-mix(in_srgb,var(--fg)_10%,transparent)]"
                role="progressbar"
                aria-valuemin="0"
                aria-valuemax="100"
                prop:aria-valuenow=move || (value.get() * 100.0).round() as i32
            >
                <div
                    class="h-full rounded-full bg-[var(--brand)] transition-all duration-300"
                    style=move || format!("width: {}%", (value.get() * 100.0).clamp(0.0, 100.0))
                ></div>
            </div>
        </div>
    }
}
