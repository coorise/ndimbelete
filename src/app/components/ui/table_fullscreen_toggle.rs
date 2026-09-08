use leptos::prelude::*;

use crate::app::hooks::use_table_fullscreen;
use crate::app::i18n::use_i18n;

/// Expand icon for table chrome. Close (red ×) is rendered by the dashboard shell
/// so `position: fixed` is not trapped by transformed ancestors.
#[component]
pub fn TableFullscreenToggle() -> impl IntoView {
    let fs = use_table_fullscreen();
    let i18n = use_i18n();

    view! {
        <Show when=move || !fs.active.get()>
            <button
                type="button"
                class="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-lg border border-[var(--border)] bg-[var(--surface)] text-[var(--fg)] transition hover:border-[var(--brand)] hover:text-[var(--brand)]"
                aria-label=move || i18n.t("common.fullscreen")
                title=move || i18n.t("common.fullscreen")
                on:click=move |_| fs.active.set(true)
            >
                <svg
                    xmlns="http://www.w3.org/2000/svg"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    class="h-4 w-4"
                    aria-hidden="true"
                >
                    <polyline points="15 3 21 3 21 9"></polyline>
                    <polyline points="9 21 3 21 3 15"></polyline>
                    <line x1="21" y1="3" x2="14" y2="10"></line>
                    <line x1="3" y1="21" x2="10" y2="14"></line>
                </svg>
            </button>
        </Show>
    }
}

/// Window-style red close — mount at shell root while table fullscreen is on.
#[component]
pub fn TableFullscreenCloseButton() -> impl IntoView {
    let fs = use_table_fullscreen();
    let i18n = use_i18n();

    view! {
        <Show when=move || fs.active.get()>
            <button
                type="button"
                class="z-[200] flex h-9 w-9 items-center justify-center rounded-md bg-[var(--brand-red)] text-xl font-bold leading-none text-white shadow-lg transition hover:brightness-110"
                style="position:fixed;top:16px;right:16px;"
                aria-label=move || i18n.t("common.exit_fullscreen")
                title=move || i18n.t("common.exit_fullscreen")
                on:click=move |_| fs.active.set(false)
            >
                "×"
            </button>
        </Show>
    }
}
