use leptos::prelude::*;
use leptos_router::components::A;

use crate::app::components::help::HelpDialog;
use crate::app::components::ui::Switch;
use crate::app::i18n::{use_i18n, LanguageSwitcher};
use crate::app::lib::{use_theme, ColorMode};

/// Sticky top bar for landing / login / setup pages.
#[component]
pub fn PublicNavbar() -> impl IntoView {
    let theme = use_theme();
    let i18n = use_i18n();
    let help_open = RwSignal::new(false);

    view! {
        <header class="sticky top-0 z-40 border-b border-[var(--border)] bg-[color-mix(in_srgb,var(--bg-elevated)_88%,transparent)] backdrop-blur-md">
            <div class="mx-auto flex max-w-5xl items-center gap-3 px-4 py-3 sm:px-6">
                <A
                    href="/"
                    attr:class="flex items-center gap-2 font-display text-xl font-bold tracking-tight text-[var(--brand)] sm:text-2xl"
                >
                    <img
                        src="/public/brand/logo.png"
                        alt="NDIMBELENTÉ"
                        class="h-9 w-9 rounded-lg object-contain"
                    />
                    <span>"NDIMBELENTÉ"</span>
                </A>

                <div class="ml-auto flex items-center gap-2 sm:gap-3">
                    <LanguageSwitcher />
                    <Switch
                        checked=Signal::derive(move || theme.mode.get() == ColorMode::Dark)
                        label=""
                        on_change=Callback::new(move |_| {
                            theme.mode.update(|m| *m = m.toggle());
                        })
                    />
                    <span class="hidden text-sm text-[var(--muted)] sm:inline">
                        {move || i18n.t("nav.theme")}
                    </span>
                    <button
                        type="button"
                        class="tap-target rounded-xl border border-[var(--border)] px-3 py-2 text-sm font-semibold hover:bg-[color-mix(in_srgb,var(--brand-yellow)_22%,transparent)]"
                        on:click=move |_| help_open.set(true)
                    >
                        {move || i18n.t("help.button")}
                    </button>
                </div>
            </div>
        </header>
        <HelpDialog open=help_open context="auth" />
    }
}
