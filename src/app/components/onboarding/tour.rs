use leptos::prelude::*;

use crate::app::components::ui::{Button, ButtonVariant};
use crate::app::i18n::use_i18n;

const ONBOARDING_KEY: &str = "ndimbelente.onboardingDone";

fn load_done() -> bool {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|ls| ls.get_item(ONBOARDING_KEY).ok().flatten())
        .map(|v| v == "1")
        .unwrap_or(false)
}

fn persist_done() {
    if let Some(ls) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = ls.set_item(ONBOARDING_KEY, "1");
    }
}

struct TourStep {
    title_key: &'static str,
    body_key: &'static str,
}

const STEPS: &[TourStep] = &[
    TourStep {
        title_key: "tour.welcome_title",
        body_key: "tour.welcome_body",
    },
    TourStep {
        title_key: "tour.sidebar_title",
        body_key: "tour.sidebar_body",
    },
    TourStep {
        title_key: "tour.search_title",
        body_key: "tour.search_body",
    },
    TourStep {
        title_key: "tour.cotisations_title",
        body_key: "tour.cotisations_body",
    },
    TourStep {
        title_key: "tour.help_title",
        body_key: "tour.help_body",
    },
];

/// First-visit overlay tour for the dashboard.
#[component]
pub fn OnboardingTour() -> impl IntoView {
    let i18n = use_i18n();
    let visible = RwSignal::new(!load_done());
    let step = RwSignal::new(0usize);

    let finish = move || {
        persist_done();
        visible.set(false);
    };

    view! {
        <Show when=move || visible.get()>
            <div
                class="fixed inset-0 z-[60] flex items-center justify-center bg-black/55 p-4"
                role="dialog"
                aria-modal="true"
            >
                <div class="w-full max-w-md rounded-2xl border border-[var(--border)] bg-[var(--bg-elevated)] p-6 shadow-[var(--shadow)] animate-fade-up">
                    <p class="mb-2 text-xs font-semibold uppercase tracking-[0.2em] text-[var(--muted)]">
                        {move || {
                            format!(
                                "{} {} / {}",
                                i18n.t("tour.step"),
                                step.get() + 1,
                                STEPS.len()
                            )
                        }}
                    </p>
                    <h2 class="font-display text-2xl font-semibold">
                        {move || {
                            let i = step.get().min(STEPS.len() - 1);
                            i18n.t(STEPS[i].title_key)
                        }}
                    </h2>
                    <p class="mt-3 text-base leading-relaxed text-[var(--muted)]">
                        {move || {
                            let i = step.get().min(STEPS.len() - 1);
                            i18n.t(STEPS[i].body_key)
                        }}
                    </p>
                    <div class="mt-6 flex flex-wrap items-center justify-between gap-3">
                        <Button
                            variant=ButtonVariant::Ghost
                            on_click=Callback::new(move |_| finish())
                        >
                            {move || i18n.t("tour.skip")}
                        </Button>
                        <div class="flex gap-2">
                            <Show when=move || step.get() != 0>
                                <Button
                                    variant=ButtonVariant::Secondary
                                    on_click=Callback::new(move |_| {
                                        step.update(|s| *s = s.saturating_sub(1));
                                    })
                                >
                                    {move || i18n.t("tour.back")}
                                </Button>
                            </Show>
                            <Button on_click=Callback::new(move |_| {
                                if step.get_untracked() + 1 >= STEPS.len() {
                                    finish();
                                } else {
                                    step.update(|s| *s += 1);
                                }
                            })>
                                {move || {
                                    if step.get() + 1 >= STEPS.len() {
                                        i18n.t("tour.done")
                                    } else {
                                        i18n.t("tour.next")
                                    }
                                }}
                            </Button>
                        </div>
                    </div>
                </div>
            </div>
        </Show>
    }
}
