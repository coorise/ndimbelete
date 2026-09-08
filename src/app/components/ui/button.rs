use leptos::prelude::*;

use crate::app::lib::cn;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Danger,
    Ghost,
}

#[component]
pub fn Button(
    #[prop(optional)] variant: ButtonVariant,
    #[prop(optional)] r#type: &'static str,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] class: &'static str,
    #[prop(optional)] on_click: Option<Callback<()>>,
    children: Children,
) -> impl IntoView {
    let base = "tap-target inline-flex items-center justify-center gap-2 rounded-xl px-4 py-2.5 text-base font-semibold transition focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 disabled:opacity-50 disabled:cursor-not-allowed";
    let variant_cls = match variant {
        ButtonVariant::Primary => {
            "bg-[var(--brand)] text-white hover:brightness-110 focus-visible:outline-[var(--brand)] shadow-sm"
        }
        ButtonVariant::Secondary => {
            "bg-[var(--bg-elevated)] text-[var(--fg)] border border-[var(--border)] hover:bg-[color-mix(in_srgb,var(--brand-yellow)_28%,var(--bg-elevated))]"
        }
        ButtonVariant::Danger => {
            "bg-[var(--brand-red)] text-white hover:brightness-110 focus-visible:outline-[var(--brand-red)]"
        }
        ButtonVariant::Ghost => {
            "bg-transparent text-[var(--fg)] hover:bg-[color-mix(in_srgb,var(--fg)_8%,transparent)]"
        }
    };
    let ty = if r#type.is_empty() { "button" } else { r#type };

    view! {
        <button
            type=ty
            class=cn(&[base, variant_cls, class])
            disabled=disabled
            on:click=move |_| {
                if let Some(cb) = on_click {
                    cb.run(());
                }
            }
        >
            {children()}
        </button>
    }
}
