use leptos::prelude::*;

use crate::app::lib::cn;

#[derive(Clone, PartialEq, Eq)]
pub struct TabItem {
    pub id: &'static str,
    pub label: String,
}

#[component]
pub fn Tabs(
    #[prop(into)] items: Signal<Vec<TabItem>>,
    active: RwSignal<&'static str>,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    view! {
        <div
            role="tablist"
            class=cn(&["flex flex-wrap gap-2 rounded-xl bg-[color-mix(in_srgb,var(--fg)_5%,transparent)] p-1", class])
        >
            <For
                each=move || items.get()
                key=|item| item.id
                children=move |item| {
                    let id = item.id;
                    let label = item.label.clone();
                    view! {
                        <button
                            type="button"
                            role="tab"
                            class=move || {
                                cn(&[
                                    "tap-target rounded-lg px-4 py-2 text-base font-semibold transition",
                                    if active.get() == id {
                                        "bg-[var(--brand)] text-white shadow-sm"
                                    } else {
                                        "text-[var(--muted)] hover:text-[var(--fg)]"
                                    },
                                ])
                            }
                            aria-selected=move || (active.get() == id).to_string()
                            on:click=move |_| active.set(id)
                        >
                            {label}
                        </button>
                    }
                }
            />
        </div>
    }
}
