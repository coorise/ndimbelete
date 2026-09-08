use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;

use crate::app::components::layout::PublicNavbar;
use crate::app::hooks::use_auth;
use crate::app::i18n::use_i18n;

/// Public landing — brand-first, one composition, elders-friendly.
/// If already logged in, redirect straight to the dashboard.
#[component]
pub fn LandingPage() -> impl IntoView {
    let i18n = use_i18n();
    let auth = use_auth();
    let navigate = use_navigate();

    Effect::new(move |_| {
        if !auth.loading.get() && auth.session.get().is_some() {
            navigate("/app", Default::default());
        }
    });

    view! {
        <div class="relative min-h-screen overflow-hidden">
            <PublicNavbar />
            <Show when=move || auth.loading.get()>
                <div class="grid min-h-screen place-items-center text-[var(--muted)]">
                    {move || i18n.t("common.loading")}
                </div>
            </Show>
            <Show when=move || !auth.loading.get() && auth.session.get().is_none()>
                <div class="pointer-events-none absolute inset-0 opacity-40">
                    <svg class="h-full w-full" viewBox="0 0 800 600" aria-hidden="true">
                        <defs>
                            <linearGradient id="handGrad" x1="0" y1="0" x2="1" y2="1">
                                <stop offset="0%" stop-color="#0B7A3E" stop-opacity="0.25"></stop>
                                <stop offset="100%" stop-color="#F5D76E" stop-opacity="0.35"></stop>
                            </linearGradient>
                        </defs>
                        <path
                            d="M180 340c40-80 120-120 200-90 30 12 55 35 70 60"
                            fill="none"
                            stroke="url(#handGrad)"
                            stroke-width="28"
                            stroke-linecap="round"
                            class="animate-soft-pulse"
                        ></path>
                        <path
                            d="M620 340c-40-80-120-120-200-90-30 12-55 35-70 60"
                            fill="none"
                            stroke="url(#handGrad)"
                            stroke-width="28"
                            stroke-linecap="round"
                            class="animate-soft-pulse"
                        ></path>
                    </svg>
                </div>

                <main class="relative mx-auto flex min-h-[calc(100vh-4rem)] max-w-5xl flex-col justify-center px-6 py-16">
                    <img
                        src="/public/brand/logo.png"
                        alt="NDIMBELENTÉ"
                        class="animate-fade-up mb-4 h-16 w-16 rounded-2xl object-contain sm:h-20 sm:w-20"
                    />
                    <p class="animate-fade-up mb-3 text-sm font-semibold uppercase tracking-[0.35em] text-[var(--brand)]">
                        {move || i18n.t("landing.tagline")}
                    </p>
                    <h1 class="animate-fade-up font-display text-5xl font-bold leading-[1.05] text-[var(--fg)] sm:text-7xl md:text-8xl"
                        style="animation-delay: 80ms"
                    >
                        "NDIMBELENTÉ"
                    </h1>
                    <p
                        class="animate-fade-up mt-6 max-w-xl text-xl leading-relaxed text-[var(--muted)] sm:text-2xl"
                        style="animation-delay: 160ms"
                    >
                        {move || i18n.t("landing.subtitle")}
                    </p>
                    <div class="animate-fade-up mt-10" style="animation-delay: 240ms">
                        <A
                            href="/login"
                            attr:class="tap-target inline-flex items-center justify-center rounded-2xl bg-[var(--brand)] px-8 py-4 text-lg font-semibold text-white shadow-[var(--shadow)] transition hover:brightness-110 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--brand)]"
                        >
                            {move || i18n.t("landing.start")}
                        </A>
                    </div>
                </main>
            </Show>
        </div>
    }
}
