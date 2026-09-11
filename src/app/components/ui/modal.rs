use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::app::components::ui::{Button, ButtonVariant};
use crate::app::lib::cn;

/// Centered modal rendered under `document.body` so navbar/overflow parents cannot clip it.
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
    let kids = children();
    let host_ref = NodeRef::<leptos::html::Div>::new();

    // Move host into <body> once so `position:fixed` is viewport-relative.
    Effect::new(move |_| {
        let Some(host) = host_ref.get() else {
            return;
        };
        let Some(window) = web_sys::window() else {
            return;
        };
        let Some(document) = window.document() else {
            return;
        };
        let Some(body) = document.body() else {
            return;
        };
        if host.parent_node().as_ref() != Some(body.unchecked_ref()) {
            let _ = body.append_child(&host);
        }
    });

    on_cleanup(move || {
        if let Some(host) = host_ref.get_untracked() {
            if let Some(parent) = host.parent_node() {
                let _ = parent.remove_child(&host);
            }
        }
    });

    view! {
        <div node_ref=host_ref>
            <div
                class=move || {
                    if open.get() {
                        "fixed inset-0 z-[200] flex items-center justify-center bg-black/50 p-4"
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
                        "relative max-h-[85vh] w-full overflow-y-auto rounded-2xl border border-[var(--border)] bg-[var(--bg-elevated)] p-5 shadow-[var(--shadow)]",
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
        </div>
    }
}
