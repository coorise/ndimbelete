use leptos::prelude::*;

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
                            lazy_count.set(page_size.get_untracked().max(25));
                        }
                    }
                >
                    <option value="page" selected=move || mode.get() == "page">"Pagination"</option>
                    <option value="all" selected=move || mode.get() == "all">"Tout charger"</option>
                    <option value="lazy" selected=move || mode.get() == "lazy">"Au défilement"</option>
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
                        prop:disabled=move || lazy_count.get() >= total.get()
                        on:click=move |_| {
                            let step = page_size.get_untracked().max(25);
                            lazy_count.update(|n| *n = (*n + step).min(total.get_untracked()));
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
