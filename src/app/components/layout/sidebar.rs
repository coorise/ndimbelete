use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_location;

use crate::app::i18n::use_i18n;
use crate::app::lib::cn;

#[derive(Clone, Copy)]
struct NavItem {
    href: &'static str,
    label_key: &'static str,
    badge: &'static str,
}

const ITEMS: &[NavItem] = &[
    NavItem {
        href: "/app",
        label_key: "nav.overview",
        badge: "O",
    },
    NavItem {
        href: "/app/staff",
        label_key: "nav.staff",
        badge: "P",
    },
    NavItem {
        href: "/app/planning",
        label_key: "nav.planning",
        badge: "L",
    },
    NavItem {
        href: "/app/members",
        label_key: "nav.members",
        badge: "M",
    },
    NavItem {
        href: "/app/cotisations",
        label_key: "nav.cotisations",
        badge: "C",
    },
    NavItem {
        href: "/app/profile",
        label_key: "nav.profile",
        badge: "U",
    },
    NavItem {
        href: "/app/settings",
        label_key: "nav.settings",
        badge: "S",
    },
];

#[component]
pub fn Sidebar(
    open: Signal<bool>,
    collapsed: Signal<bool>,
    #[prop(into)] on_navigate: Callback<()>,
    #[prop(optional)] on_check_update: Option<Callback<()>>,
    #[prop(optional)] version_label: Option<Signal<String>>,
) -> impl IntoView {
    let location = use_location();
    let i18n = use_i18n();

    view! {
        <aside class=move || {
            let collapsed = collapsed.get();
            cn(&[
                "fixed inset-y-0 left-0 z-30 flex flex-col overflow-x-hidden border-r border-[var(--border)] bg-[var(--bg-elevated)] p-3 transition-all lg:static lg:h-full lg:translate-x-0 lg:overflow-y-auto",
                "w-72 max-w-[85vw]",
                if collapsed {
                    "lg:w-[4.5rem]"
                } else {
                    "lg:w-72"
                },
                if open.get() {
                    "translate-x-0"
                } else {
                    "-translate-x-full"
                },
            ])
        }>
            <div class="mb-4 mt-14 min-w-0 px-1 lg:mt-1">
                <A
                    href="/app"
                    on:click=move |_| on_navigate.run(())
                    attr:class=move || {
                        cn(&[
                            "flex min-w-0 items-center gap-2 rounded-xl px-2 py-2 font-display font-bold tracking-tight text-[var(--brand)] transition hover:bg-[color-mix(in_srgb,var(--brand-yellow)_18%,transparent)]",
                            if collapsed.get() {
                                "lg:justify-center lg:px-1"
                            } else {
                                ""
                            },
                        ])
                    }
                    attr:title="NDIMBELENTÉ"
                >
                    <img
                        src="/public/brand/logo.png"
                        alt="NDIMBELENTÉ"
                        class="brand-logo shrink-0 rounded-lg object-contain"
                    />
                    <span class=move || {
                        cn(&[
                            "brand-title min-w-0 flex-1 truncate",
                            if collapsed.get() {
                                "lg:hidden"
                            } else {
                                ""
                            },
                        ])
                    }>
                        "NDIMBELENTÉ"
                    </span>
                </A>
            </div>

            <div class="mb-3 px-1">
                <p class=move || {
                    cn(&[
                        "text-xs font-semibold uppercase tracking-[0.2em] text-[var(--muted)]",
                        if collapsed.get() {
                            "lg:sr-only"
                        } else {
                            ""
                        },
                    ])
                }>
                    {move || i18n.t("nav.navigation")}
                </p>
            </div>
            <nav class="flex min-h-0 flex-1 flex-col gap-2" aria-label=move || i18n.t("nav.navigation")>
                {ITEMS
                    .iter()
                    .copied()
                    .map(|item| {
                        let href = item.href;
                        let badge = item.badge;
                        let label_key = item.label_key;
                        view! {
                            <A
                                href=href
                                on:click=move |_| on_navigate.run(())
                                attr:title=move || i18n.t(label_key)
                                attr:class=move || {
                                    let path = location.pathname.get();
                                    let active = if href == "/app" {
                                        path == "/app" || path == "/app/"
                                    } else {
                                        path.starts_with(href)
                                    };
                                    let collapsed = collapsed.get();
                                    cn(&[
                                        "tap-target flex items-center rounded-xl font-semibold transition",
                                        if collapsed {
                                            "gap-3 px-4 py-3 text-base lg:justify-center lg:gap-0 lg:px-2"
                                        } else {
                                            "gap-3 px-4 py-3 text-base"
                                        },
                                        if active {
                                            "bg-[var(--brand)] text-white shadow-sm"
                                        } else {
                                            "text-[var(--fg)] hover:bg-[color-mix(in_srgb,var(--brand-yellow)_22%,transparent)]"
                                        },
                                    ])
                                }
                            >
                                <span class="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-[color-mix(in_srgb,var(--fg)_8%,transparent)] text-sm font-bold">
                                    {badge}
                                </span>
                                <span class=move || {
                                    if collapsed.get() {
                                        "inline lg:hidden"
                                    } else {
                                        "inline"
                                    }
                                }>
                                    {move || i18n.t(label_key)}
                                </span>
                            </A>
                        }
                    })
                    .collect_view()}
            </nav>

            <div class="mt-3 shrink-0 border-t border-[var(--border)] pt-3">
                <button
                    type="button"
                    class=move || {
                        cn(&[
                            "tap-target flex w-full items-center rounded-xl font-semibold text-[var(--fg)] transition hover:bg-[color-mix(in_srgb,var(--brand-yellow)_22%,transparent)]",
                            if collapsed.get() {
                                "justify-center gap-0 px-2 py-3 lg:px-2"
                            } else {
                                "gap-3 px-4 py-3 text-sm"
                            },
                        ])
                    }
                    title="Rechercher mise à jour"
                    on:click=move |_| {
                        if let Some(cb) = on_check_update {
                            cb.run(());
                        }
                    }
                >
                    <span class="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-[color-mix(in_srgb,var(--fg)_8%,transparent)] text-sm font-bold">
                        "↻"
                    </span>
                    <span class=move || {
                        if collapsed.get() {
                            "inline lg:hidden"
                        } else {
                            "inline"
                        }
                    }>
                        "Rechercher mise à jour"
                    </span>
                </button>
                <p class=move || {
                    cn(&[
                        "mt-1 px-2 text-[10px] text-[var(--muted)]",
                        if collapsed.get() {
                            "lg:hidden"
                        } else {
                            ""
                        },
                    ])
                }>
                    {move || {
                        version_label
                            .map(|s| s.get())
                            .unwrap_or_else(|| "v?".into())
                    }}
                </p>
            </div>
        </aside>
    }
}
