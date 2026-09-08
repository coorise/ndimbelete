use leptos::prelude::*;

use crate::app::lib::cn;

#[component]
pub fn Badge(
    #[prop(optional)] tone: &'static str,
    children: Children,
) -> impl IntoView {
    let tone_cls = match tone {
        "red" => "bg-[color-mix(in_srgb,var(--brand-red)_18%,transparent)] text-[var(--brand-red)]",
        "yellow" => {
            "bg-[color-mix(in_srgb,var(--brand-yellow)_45%,transparent)] text-[#6b5a12]"
        }
        "muted" => "bg-[color-mix(in_srgb,var(--muted)_18%,transparent)] text-[var(--muted)]",
        _ => "bg-[color-mix(in_srgb,var(--brand)_18%,transparent)] text-[var(--brand)]",
    };
    view! {
        <span class=cn(&[
            "inline-flex items-center rounded-lg px-2.5 py-1 text-sm font-semibold",
            tone_cls,
        ])>
            {children()}
        </span>
    }
}
