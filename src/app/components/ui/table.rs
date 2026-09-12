use leptos::prelude::*;
use wasm_bindgen::JsCast;

use super::pagination::try_load_more_on_scroll;
use super::table_fullscreen_toggle::TableFullscreenToggle;
use crate::app::hooks::TableFullscreenCtx;
use crate::app::lib::cn;

#[component]
pub fn Table(
    #[prop(optional)] class: &'static str,
    /// Compact / comfortable / spacious via CSS variable --table-row-py
    #[prop(optional)] density: Option<RwSignal<f64>>,
    /// Show expand control on the top-right of the table chrome (above the h-scroll rail).
    #[prop(optional)] fullscreen_toggle: bool,
    /// When set with lazy_* signals, loads more rows near the bottom of the body scroll.
    #[prop(optional)] lazy_mode: Option<RwSignal<String>>,
    #[prop(optional)] lazy_count: Option<RwSignal<usize>>,
    #[prop(optional)] lazy_total: Option<Signal<usize>>,
    #[prop(optional)] lazy_page_size: Option<RwSignal<usize>>,
    children: Children,
) -> impl IntoView {
    // Dual scrollbars: top rail mirrors the body so users don't hunt for bottom scroll.
    let top_ref = NodeRef::<leptos::html::Div>::new();
    let body_ref = NodeRef::<leptos::html::Div>::new();
    let spacer_ref = NodeRef::<leptos::html::Div>::new();
    let syncing = RwSignal::new(false);
    let can_fullscreen = fullscreen_toggle && use_context::<TableFullscreenCtx>().is_some();

    Effect::new(move |_| {
        let Some(top) = top_ref.get() else {
            return;
        };
        let Some(body) = body_ref.get() else {
            return;
        };
        let Some(spacer) = spacer_ref.get() else {
            return;
        };

        let sync_width = {
            let body = body.clone();
            let spacer = spacer.clone();
            move || {
                let w = body.scroll_width();
                let style = web_sys::HtmlElement::style(&spacer);
                let _ = style.set_property("width", &format!("{w}px"));
                let _ = style.set_property("height", "1px");
            }
        };
        sync_width();

        let top_el = top.clone();
        let body_el = body.clone();
        let on_top = wasm_bindgen::closure::Closure::<dyn FnMut(_)>::new({
            let body = body_el.clone();
            move |_: web_sys::Event| {
                if syncing.get_untracked() {
                    return;
                }
                syncing.set(true);
                body.set_scroll_left(top_el.scroll_left());
                syncing.set(false);
            }
        });
        let on_body = wasm_bindgen::closure::Closure::<dyn FnMut(_)>::new({
            let top = top.clone();
            let body = body.clone();
            move |_: web_sys::Event| {
                if syncing.get_untracked() {
                    return;
                }
                syncing.set(true);
                top.set_scroll_left(body.scroll_left());
                syncing.set(false);

                if let (Some(mode), Some(count), Some(total), Some(step)) =
                    (lazy_mode, lazy_count, lazy_total, lazy_page_size)
                {
                    try_load_more_on_scroll(
                        body.as_ref(),
                        &mode.get_untracked(),
                        count,
                        total.get_untracked(),
                        step.get_untracked(),
                    );
                }
            }
        });
        let _ = top.add_event_listener_with_callback("scroll", on_top.as_ref().unchecked_ref());
        let _ = body.add_event_listener_with_callback("scroll", on_body.as_ref().unchecked_ref());

        // Keep spacer width in sync when the table layout changes.
        if let Some(win) = web_sys::window() {
            let cb = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(sync_width);
            let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                50,
            );
            cb.forget();
        }

        on_top.forget();
        on_body.forget();
    });

    // Keep loading until the body fills the viewport (or all rows are shown).
    Effect::new(move |_| {
        let Some(mode) = lazy_mode else {
            return;
        };
        let Some(count) = lazy_count else {
            return;
        };
        let Some(total) = lazy_total else {
            return;
        };
        let Some(step) = lazy_page_size else {
            return;
        };
        let _ = mode.get();
        let _ = count.get();
        let _ = total.get();
        let Some(body) = body_ref.get() else {
            return;
        };
        if mode.get_untracked() != "lazy" {
            return;
        }
        if let Some(win) = web_sys::window() {
            let body_fill = body.clone();
            let cb = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(move || {
                try_load_more_on_scroll(
                    body_fill.as_ref(),
                    &mode.get_untracked(),
                    count,
                    total.get_untracked(),
                    step.get_untracked(),
                );
            });
            let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                100,
            );
            cb.forget();
        }
    });

    view! {
        <div class="table-frame flex min-h-0 w-full max-w-full flex-1 flex-col gap-1">
            <div class="flex shrink-0 items-center justify-between gap-2">
                <p class="table-scroll-hint min-w-0 flex-1 text-xs font-medium text-[var(--muted)]">
                    "← Faites défiler horizontalement (barre en haut) pour voir toutes les colonnes →"
                </p>
                <Show when=move || can_fullscreen>
                    <div class="shrink-0">
                        <TableFullscreenToggle />
                    </div>
                </Show>
            </div>
            <div
                node_ref=top_ref
                class="table-h-scroll table-h-scroll-top w-full max-w-full rounded-t-xl border border-b-0 border-[var(--border)]"
                aria-hidden="true"
            >
                <div node_ref=spacer_ref></div>
            </div>
            <div
                node_ref=body_ref
                class=cn(&[
                    "table-h-scroll table-h-scroll-body w-full max-w-full rounded-b-xl border border-[var(--border)]",
                    class,
                ])
                style=move || {
                    density
                        .map(|d| format!("--table-row-py:{:.2}rem", d.get()))
                        .unwrap_or_else(|| "--table-row-py:0.4rem".into())
                }
            >
                <table class="w-max min-w-full border-separate border-spacing-0 text-left text-sm">
                    {children()}
                </table>
            </div>
        </div>
    }
}

#[component]
pub fn THead(children: Children) -> impl IntoView {
    view! {
        <thead class="bg-[color-mix(in_srgb,var(--brand)_8%,var(--bg-elevated))] text-xs uppercase tracking-wide text-[var(--muted)]">
            <tr>{children()}</tr>
        </thead>
    }
}

#[component]
pub fn Th(
    #[prop(optional)] class: &'static str,
    #[prop(optional)] on_click: Option<Callback<()>>,
    #[prop(optional)] sticky: bool,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    view! {
        <th
            class=cn(&[
                "whitespace-nowrap px-2 font-semibold",
                if on_click.is_some() {
                    "cursor-pointer select-none hover:text-[var(--fg)]"
                } else {
                    ""
                },
                if sticky {
                    "sticky right-0 z-10 bg-[color-mix(in_srgb,var(--brand)_8%,var(--bg-elevated))] shadow-[-6px_0_8px_-6px_rgba(0,0,0,0.15)]"
                } else {
                    ""
                },
                class,
            ])
            style="padding-top:var(--table-row-py);padding-bottom:var(--table-row-py)"
            on:click=move |_| {
                if let Some(cb) = on_click {
                    cb.run(());
                }
            }
        >
            {children.map(|c| c())}
        </th>
    }
}

#[component]
pub fn TBody(children: Children) -> impl IntoView {
    view! { <tbody class="divide-y divide-[var(--border)] bg-[var(--bg-elevated)]">{children()}</tbody> }
}

#[component]
pub fn Tr(#[prop(optional)] class: &'static str, children: Children) -> impl IntoView {
    view! {
        <tr class=cn(&["hover:bg-[color-mix(in_srgb,var(--brand-yellow)_12%,transparent)]", class])>
            {children()}
        </tr>
    }
}

#[component]
pub fn Td(
    #[prop(optional)] class: &'static str,
    #[prop(optional)] sticky: bool,
    children: Children,
) -> impl IntoView {
    view! {
        <td
            class=cn(&[
                "px-2 align-middle",
                if sticky {
                    "sticky right-0 z-[1] bg-[var(--bg-elevated)] shadow-[-6px_0_8px_-6px_rgba(0,0,0,0.12)]"
                } else {
                    ""
                },
                class,
            ])
            style="padding-top:var(--table-row-py);padding-bottom:var(--table-row-py)"
        >
            {children()}
        </td>
    }
}

#[component]
pub fn RowDensitySlider(value: RwSignal<f64>) -> impl IntoView {
    view! {
        <label class="flex min-w-[12rem] flex-col gap-1 text-xs font-medium text-[var(--muted)]">
            <span>
                "Hauteur des lignes ("
                {move || format!("{:.1}", value.get())}
                ")"
            </span>
            <input
                type="range"
                min="0.15"
                max="0.9"
                step="0.05"
                class="w-full"
                prop:value=move || value.get()
                on:input=move |ev| {
                    if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                        value.set(v);
                    }
                }
            />
        </label>
    }
}
