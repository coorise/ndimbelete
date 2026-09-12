use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::app::lib::cn;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TableLoadMode {
    /// Show every row at once.
    All,
    /// Classic page size + page controls.
    Paginated,
    /// Append more rows as the user scrolls.
    Lazy,
}

impl TableLoadMode {
    pub fn from_str(s: &str) -> Self {
        match s {
            "page" => Self::Paginated,
            "lazy" => Self::Lazy,
            _ => Self::All,
        }
    }
}

/// Default load mode for management tables.
pub const DEFAULT_TABLE_LOAD_MODE: &str = "lazy";

/// Initial window size when entering lazy / scroll mode.
pub fn default_lazy_count(page_size: usize) -> usize {
    page_size.max(25)
}

/// Slice rows according to load mode / page / lazy window.
pub fn paginate_slice<T: Clone>(
    rows: &[T],
    mode: TableLoadMode,
    page: usize,
    page_size: usize,
    lazy_count: usize,
) -> Vec<T> {
    if rows.is_empty() {
        return Vec::new();
    }
    match mode {
        TableLoadMode::All => rows.to_vec(),
        TableLoadMode::Paginated => {
            let size = page_size.max(1);
            let start = page.saturating_mul(size);
            rows.iter().skip(start).take(size).cloned().collect()
        }
        TableLoadMode::Lazy => rows.iter().take(lazy_count.max(1)).cloned().collect(),
    }
}

/// Grow the lazy window when a scrollable element is near the bottom.
pub fn try_load_more_on_scroll(
    el: &web_sys::Element,
    mode: &str,
    lazy_count: RwSignal<usize>,
    total: usize,
    step: usize,
) {
    if mode != "lazy" || total == 0 {
        return;
    }
    let shown = lazy_count.get_untracked();
    if shown >= total {
        return;
    }
    let scroll_top = el.scroll_top() as f64;
    let client_h = el.client_height() as f64;
    let scroll_h = el.scroll_height() as f64;
    if client_h <= 0.0 {
        return;
    }
    let remaining = scroll_h - scroll_top - client_h;
    if remaining <= 160.0 {
        let step = step.max(25);
        lazy_count.update(|n| *n = (*n + step).min(total));
    }
}

/// Wire vertical near-bottom loading on a scroll container (table body or grid).
#[component]
pub fn LazyScrollRegion(
    mode: RwSignal<String>,
    lazy_count: RwSignal<usize>,
    total: Signal<usize>,
    page_size: RwSignal<usize>,
    #[prop(optional)] class: &'static str,
    children: Children,
) -> impl IntoView {
    let node_ref = NodeRef::<leptos::html::Div>::new();

    // Auto-fill short viewports when lazy window / mode changes.
    Effect::new(move |_| {
        let _ = mode.get();
        let _ = lazy_count.get();
        let _ = total.get();
        let Some(el) = node_ref.get() else {
            return;
        };
        if mode.get_untracked() != "lazy" {
            return;
        }
        if let Some(win) = web_sys::window() {
            let el2 = el.clone();
            let cb = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(move || {
                try_load_more_on_scroll(
                    el2.as_ref(),
                    &mode.get_untracked(),
                    lazy_count,
                    total.get_untracked(),
                    page_size.get_untracked(),
                );
            });
            let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                80,
            );
            cb.forget();
        }
    });

    view! {
        <div
            node_ref=node_ref
            class=cn(&[class, "min-h-0"])
            on:scroll=move |ev| {
                if let Some(el) = ev
                    .current_target()
                    .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                {
                    try_load_more_on_scroll(
                        &el,
                        &mode.get_untracked(),
                        lazy_count,
                        total.get_untracked(),
                        page_size.get_untracked(),
                    );
                }
            }
        >
            {children()}
        </div>
    }
}

#[component]
pub fn TablePaginationBar(
    total: Signal<usize>,
    page: RwSignal<usize>,
    page_size: RwSignal<usize>,
    mode: RwSignal<String>,
    lazy_count: RwSignal<usize>,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let page_count = Signal::derive(move || {
        let total = total.get();
        let size = page_size.get().max(1);
        if total == 0 {
            1usize
        } else {
            total.div_ceil(size)
        }
    });

    let can_prev = Signal::derive(move || page.get() > 0);
    let can_next = Signal::derive(move || {
        let p = page.get();
        let pc = page_count.get();
        p + 1 < pc
    });

    Effect::new(move |_| {
        let max_page = page_count.get().saturating_sub(1);
        if page.get() > max_page {
            page.set(max_page);
        }
    });

    view! {
        <div class=cn(&[
            "table-pagination-bar z-20 flex shrink-0 flex-wrap items-center justify-center gap-2 rounded-xl border border-[var(--border)] bg-[var(--bg-elevated)] px-3 py-2 shadow-sm",
            class,
        ])>
            <label class="flex items-center gap-1.5 text-xs font-medium text-[var(--muted)]">
                <span class="sr-only">"Mode"</span>
                <select
                    class="h-8 rounded-lg border border-[var(--border)] bg-[var(--bg-elevated)] px-2 text-xs font-semibold text-[var(--fg)]"
                    prop:value=move || mode.get()
                    on:change=move |ev| {
                        let v = event_target_value(&ev);
                        mode.set(v.clone());
                        page.set(0);
                        if v == "lazy" {
                            lazy_count.set(default_lazy_count(page_size.get_untracked()));
                        }
                    }
                >
                    <option value="lazy" selected=move || mode.get() == "lazy">"Au défilement"</option>
                    <option value="page" selected=move || mode.get() == "page">"Pagination"</option>
                    <option value="all" selected=move || mode.get() == "all">"Tout charger"</option>
                </select>
            </label>

            <Show when=move || mode.get() == "page">
                <div class="flex flex-wrap items-center gap-1.5">
                    <select
                        class="h-8 rounded-lg border border-[var(--border)] bg-[var(--bg-elevated)] px-2 text-xs font-semibold text-[var(--fg)]"
                        prop:value=move || page_size.get().to_string()
                        on:change=move |ev| {
                            if let Ok(n) = event_target_value(&ev).parse::<usize>() {
                                page_size.set(n);
                                page.set(0);
                            }
                        }
                    >
                        <option value="10" selected=move || page_size.get() == 10>"10 / page"</option>
                        <option value="25" selected=move || page_size.get() == 25>"25 / page"</option>
                        <option value="50" selected=move || page_size.get() == 50>"50 / page"</option>
                        <option value="100" selected=move || page_size.get() == 100>"100 / page"</option>
                    </select>
                    <button
                        type="button"
                        class="inline-flex h-8 w-8 items-center justify-center rounded-lg border border-[var(--border)] text-sm font-bold disabled:opacity-35"
                        prop:disabled=move || !can_prev.get()
                        on:click=move |_| page.update(|p| *p = p.saturating_sub(1))
                    >
                        "‹"
                    </button>
                    <span class="min-w-[6.5rem] text-center text-xs font-semibold tabular-nums text-[var(--fg)]">
                        {move || {
                            format!(
                                "Page {}/{} · {}",
                                page.get() + 1,
                                page_count.get(),
                                total.get()
                            )
                        }}
                    </span>
                    <button
                        type="button"
                        class="inline-flex h-8 w-8 items-center justify-center rounded-lg border border-[var(--border)] text-sm font-bold disabled:opacity-35"
                        prop:disabled=move || !can_next.get()
                        on:click=move |_| page.update(|p| *p += 1)
                    >
                        "›"
                    </button>
                </div>
            </Show>

            <Show when=move || mode.get() == "lazy">
                <div class="flex items-center gap-2">
                    <span class="text-xs font-medium text-[var(--muted)]">
                        {move || format!("{} / {}", lazy_count.get().min(total.get()), total.get())}
                    </span>
                    <button
                        type="button"
                        class="h-8 rounded-lg border border-[var(--border)] px-3 text-xs font-semibold disabled:opacity-35"
                        disabled=Signal::derive(move || lazy_count.get() >= total.get())
                        on:click=move |_| {
                            let step = default_lazy_count(page_size.get_untracked());
                            let max = total.get_untracked();
                            lazy_count.update(|n| *n = (*n + step).min(max));
                        }
                    >
                        "Charger plus"
                    </button>
                </div>
            </Show>

            <Show when=move || mode.get() == "all">
                <span class="text-xs font-medium text-[var(--muted)]">
                    {move || format!("{} lignes", total.get())}
                </span>
            </Show>
        </div>
    }
}
