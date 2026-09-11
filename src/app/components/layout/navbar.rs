use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;
use wasm_bindgen::JsCast;

use crate::app::components::help::HelpDialog;
use crate::app::components::collab::CollabNavControls;
use crate::app::components::ui::{Button, ButtonVariant, Switch};
use crate::app::hooks::use_auth;
use crate::app::i18n::{use_i18n, LanguageSwitcher};
use crate::app::lib::{use_theme, ColorMode};

struct PageLink {
    href: &'static str,
    label_key: &'static str,
}

const PAGE_LINKS: &[PageLink] = &[
    PageLink {
        href: "/app",
        label_key: "nav.overview",
    },
    PageLink {
        href: "/app/staff",
        label_key: "nav.staff",
    },
    PageLink {
        href: "/app/members",
        label_key: "nav.members",
    },
    PageLink {
        href: "/app/cotisations",
        label_key: "nav.cotisations",
    },
    PageLink {
        href: "/app/profile",
        label_key: "nav.profile",
    },
    PageLink {
        href: "/app/settings",
        label_key: "nav.settings",
    },
    PageLink {
        href: "/app/collaboration",
        label_key: "nav.collaboration",
    },
];

#[component]
pub fn Navbar(
    #[prop(optional)] on_toggle_nav: Option<Callback<()>>,
    #[prop(optional)] on_toggle_collapse: Option<Callback<()>>,
) -> impl IntoView {
    let auth = use_auth();
    let theme = use_theme();
    let i18n = use_i18n();
    let navigate = use_navigate();
    let navigate_logout = navigate.clone();
    let navigate_enter = navigate.clone();
    let navigate_profile = navigate;
    let help_open = RwSignal::new(false);

    let query = RwSignal::new(String::new());
    let open = RwSignal::new(false);
    let input_ref = NodeRef::<leptos::html::Input>::new();

    let suggestions = Memo::new(move |_| {
        let q = query.get().trim().to_lowercase();
        PAGE_LINKS
            .iter()
            .filter_map(|link| {
                let label = i18n.t(link.label_key);
                if q.is_empty() || label.to_lowercase().contains(&q) || link.href.contains(&q) {
                    Some((link.href, label))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    });

    Effect::new(move |_| {
        let Some(document) = web_sys::window().and_then(|w| w.document()) else {
            return;
        };
        let closure = wasm_bindgen::closure::Closure::<dyn FnMut(_)>::new({
            move |ev: web_sys::KeyboardEvent| {
                let meta = ev.meta_key() || ev.ctrl_key();
                if meta && ev.key().eq_ignore_ascii_case("k") {
                    ev.prevent_default();
                    open.set(true);
                    if let Some(el) = input_ref.get_untracked() {
                        let _ = el.focus();
                    }
                }
            }
        });
        let _ = document.add_event_listener_with_callback(
            "keydown",
            closure.as_ref().unchecked_ref(),
        );
        closure.forget();
    });

    Effect::new(move |_| {
        let Some(document) = web_sys::window().and_then(|w| w.document()) else {
            return;
        };
        let closure = wasm_bindgen::closure::Closure::<dyn FnMut(_)>::new({
            move |ev: web_sys::MouseEvent| {
                if !open.get_untracked() {
                    return;
                }
                let Some(target) = ev.target() else {
                    return;
                };
                let Ok(el) = target.dyn_into::<web_sys::Element>() else {
                    open.set(false);
                    return;
                };
                if el.closest("#nav-search-wrap").ok().flatten().is_none() {
                    open.set(false);
                }
            }
        });
        let _ = document.add_event_listener_with_callback(
            "mousedown",
            closure.as_ref().unchecked_ref(),
        );
        closure.forget();
    });

    view! {
        <header class="sticky top-0 z-40 border-b border-[var(--border)] bg-[color-mix(in_srgb,var(--bg-elevated)_88%,transparent)] backdrop-blur-md">
            <div class="flex items-center gap-3 px-3 py-3 sm:px-5">
                <button
                    type="button"
                    class="tap-target rounded-xl border border-[var(--border)] px-3 text-lg lg:hidden"
                    aria-label=move || i18n.t("nav.open_menu")
                    on:click=move |_| {
                        if let Some(cb) = on_toggle_nav {
                            cb.run(());
                        }
                    }
                >
                    "☰"
                </button>

                <button
                    type="button"
                    class="tap-target hidden items-center justify-center rounded-xl border border-[var(--border)] px-2.5 text-base lg:inline-flex"
                    aria-label=move || i18n.t("nav.collapse_sidebar")
                    title=move || i18n.t("nav.collapse_sidebar")
                    on:click=move |_| {
                        if let Some(cb) = on_toggle_collapse {
                            cb.run(());
                        }
                    }
                >
                    <span class="inline-flex flex-col gap-0.5" aria-hidden="true">
                        <span class="block h-0.5 w-4 rounded bg-[var(--fg)]"></span>
                        <span class="block h-0.5 w-3 rounded bg-[var(--fg)]"></span>
                        <span class="block h-0.5 w-4 rounded bg-[var(--fg)]"></span>
                    </span>
                </button>

                <div class="relative ml-1 hidden min-w-0 flex-1 sm:block md:max-w-md" id="nav-search-wrap">
                    <label class="sr-only" for="nav-page-search">
                        {move || i18n.t("nav.search_placeholder")}
                    </label>
                    <input
                        id="nav-page-search"
                        node_ref=input_ref
                        type="search"
                        class="tap-target w-full rounded-xl border border-[var(--border)] bg-[var(--bg-elevated)] px-3 py-2 text-sm focus:border-[var(--brand)] focus:outline-none focus:ring-2 focus:ring-[color-mix(in_srgb,var(--brand)_35%,transparent)]"
                        placeholder=move || i18n.t("nav.search_placeholder")
                        prop:value=move || query.get()
                        on:input=move |ev| {
                            query.set(event_target_value(&ev));
                            open.set(true);
                        }
                        on:focus=move |_| open.set(true)
                        on:keydown=move |ev| {
                            let key = ev.key();
                            if key == "Escape" {
                                open.set(false);
                                query.set(String::new());
                            } else if key == "Enter" {
                                if let Some((href, _)) = suggestions.get_untracked().first() {
                                    navigate_enter(href, Default::default());
                                    query.set(String::new());
                                    open.set(false);
                                }
                            }
                        }
                    />
                    <Show when=move || open.get() && !suggestions.get().is_empty()>
                        <ul
                            class="absolute left-0 right-0 top-full z-50 mt-1 max-h-64 overflow-auto rounded-xl border border-[var(--border)] bg-[var(--bg-elevated)] py-1 shadow-[var(--shadow)]"
                            role="listbox"
                        >
                            <For
                                each=move || suggestions.get()
                                key=|(href, label)| format!("{href}:{label}")
                                children=move |(href, label)| {
                                    view! {
                                        <li role="option">
                                            <A
                                                href=href
                                                attr:class="tap-target flex w-full px-3 py-2 text-left text-sm font-semibold hover:bg-[color-mix(in_srgb,var(--brand-yellow)_22%,transparent)]"
                                                on:click=move |_| {
                                                    query.set(String::new());
                                                    open.set(false);
                                                }
                                            >
                                                {label}
                                            </A>
                                        </li>
                                    }
                                }
                            />
                        </ul>
                    </Show>
                </div>

                <div class="ml-auto flex items-center gap-2 sm:gap-3">
                    <CollabNavControls />
                    <LanguageSwitcher />

                    <button
                        type="button"
                        class="tap-target rounded-xl border border-[var(--border)] px-3 py-2 text-sm font-semibold hover:bg-[color-mix(in_srgb,var(--brand-yellow)_22%,transparent)]"
                        on:click=move |_| help_open.set(true)
                    >
                        {move || i18n.t("help.button")}
                    </button>

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
                        class="hidden text-right sm:block"
                        on:click=move |_| {
                            navigate_profile("/app/profile", Default::default());
                        }
                    >
                        <div class="text-sm font-semibold hover:text-[var(--brand)]">
                            {move || {
                                auth.session
                                    .get()
                                    .map(|s| s.staff.full_name)
                                    .unwrap_or_else(|| "—".into())
                            }}
                        </div>
                        <div class="text-xs text-[var(--muted)]">
                            {move || {
                                auth.session
                                    .get()
                                    .and_then(|s| s.staff.role_name)
                                    .unwrap_or_default()
                            }}
                        </div>
                    </button>

                    <Button
                        variant=ButtonVariant::Secondary
                        on_click=Callback::new(move |_| {
                            auth.logout();
                            navigate_logout("/login", Default::default());
                        })
                    >
                        {move || i18n.t("nav.logout")}
                    </Button>
                </div>
            </div>
        </header>
        <HelpDialog open=help_open context="dashboard" />
    }
}
