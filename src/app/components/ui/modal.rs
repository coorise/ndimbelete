use leptos::prelude::*;

use crate::app::components::ui::{Button, ButtonVariant};
use crate::app::lib::cn;

#[component]
pub fn Modal(
    open: Signal<bool>,
    #[prop(into)] on_close: Callback<()>,
    #[prop(optional)] title: &'static str,
    /// When set, takes precedence over `title` (reactive i18n).
    #[prop(optional)] title_signal: Option<Signal<String>>,
    #[prop(optional)] wide: bool,
    children: Children,
) -> impl IntoView {
    // Render always; hide with CSS so children aren't consumed inside <Show>.
    let kids = children();
    view! {
        <div
            class=move || {
                if open.get() {
                    "fixed inset-0 z-50 flex items-end justify-center bg-black/45 p-3 sm:items-center"
                } else {
                    "hidden"
                }
            }
            role="dialog"
            aria-modal="true"
            on:click=move |_| on_close.run(())
        >
            <div
                class=cn(&[
                    "max-h-[90vh] w-full overflow-y-auto rounded-2xl border border-[var(--border)] bg-[var(--bg-elevated)] p-5 shadow-[var(--shadow)] animate-fade-up",
                    if wide { "max-w-3xl" } else { "max-w-lg" },
                ])
                on:click=move |ev| ev.stop_propagation()
            >
                <div class="mb-4 flex items-start justify-between gap-3">
                    <h2 class="font-display text-2xl font-semibold">
                        {move || {
                            title_signal
                                .map(|s| s.get())
                                .unwrap_or_else(|| title.to_string())
                        }}
                    </h2>
                    <Button variant=ButtonVariant::Ghost on_click=Callback::new(move |_| on_close.run(()))>
                        "Fermer"
                    </Button>
                </div>
                {kids}
            </div>
        </div>
    }
}
