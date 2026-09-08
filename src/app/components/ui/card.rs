use leptos::prelude::*;

use crate::app::lib::cn;

#[component]
pub fn Card(
    #[prop(optional)] class: &'static str,
    #[prop(optional)] title: &'static str,
    children: Children,
) -> impl IntoView {
    view! {
        <section class=cn(&[
            "rounded-2xl border border-[var(--border)] bg-[var(--surface)] p-5 shadow-[var(--shadow)]",
            class,
        ])>
            {(!title.is_empty()).then(|| {
                view! {
                    <h2 class="mb-3 font-display text-xl font-semibold text-[var(--fg)]">{title}</h2>
                }
            })}
            {children()}
        </section>
    }
}
