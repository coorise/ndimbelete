use leptos::prelude::*;
use leptos_router::components::Outlet;
use leptos_router::hooks::{use_location, use_navigate};
use wasm_bindgen::JsCast;

use crate::app::components::layout::{Navbar, Sidebar};
use crate::app::components::onboarding::OnboardingTour;
use crate::app::components::ui::TableFullscreenCloseButton;
use crate::app::components::updates::UpdateCheckModal;
use crate::app::hooks::{provide_table_fullscreen, use_auth, use_table_fullscreen};
use crate::app::i18n::use_i18n;
use crate::app::lib::{api, apply_font_scale, apply_theme_color, cn};

const SIDEBAR_KEY: &str = "ndimbelente.sidebarCollapsed";

fn load_collapsed() -> bool {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|ls| ls.get_item(SIDEBAR_KEY).ok().flatten())
        .map(|v| v == "1" || v == "true")
        .unwrap_or(false)
}

fn persist_collapsed(v: bool) {
    if let Some(ls) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = ls.set_item(SIDEBAR_KEY, if v { "1" } else { "0" });
    }
}

#[component]
pub fn DashboardShell() -> impl IntoView {
    let auth = use_auth();
    let i18n = use_i18n();
    let navigate = use_navigate();
    let nav_open = RwSignal::new(false);
    let collapsed = RwSignal::new(load_collapsed());
    let update_open = RwSignal::new(false);
    let version_label = RwSignal::new("…".to_string());

    provide_table_fullscreen();
    let fs = use_table_fullscreen();
    let location = use_location();

    Effect::new(move |_| {
        persist_collapsed(collapsed.get());
    });

    // Leave table fullscreen when changing routes.
    Effect::new(move |_| {
        let _ = location.pathname.get();
        fs.active.set(false);
    });

    // Escape exits table fullscreen.
    Effect::new(move |_| {
        let Some(window) = web_sys::window() else {
            return;
        };
        let Some(document) = window.document() else {
            return;
        };
        let handler = wasm_bindgen::closure::Closure::wrap(Box::new(move |ev: web_sys::KeyboardEvent| {
            if ev.key() == "Escape" && fs.active.get_untracked() {
                fs.active.set(false);
            }
        }) as Box<dyn FnMut(_)>);
        let _ = document.add_event_listener_with_callback("keydown", handler.as_ref().unchecked_ref());
        handler.forget();
    });

    Effect::new({
        let navigate = navigate.clone();
        move |_| {
            if !auth.loading.get() && auth.session.get().is_none() {
                navigate("/login", Default::default());
            }
        }
    });

    Effect::new(move |_| {
        if auth.session.get().is_some() {
            leptos::task::spawn_local(async move {
                if let Ok(s) = api::get_settings().await {
                    apply_font_scale(s.font_scale);
                    apply_theme_color(&s.theme_color);
                }
            });
        }
    });

    // Version label + quiet update check on login / reload.
    Effect::new(move |_| {
        if auth.session.get().is_none() {
            return;
        }
        leptos::task::spawn_local(async move {
            if let Ok(v) = api::get_app_version_info().await {
                version_label.set(format!("v{} · {}", v.version, v.channel));
            }
            match api::check_for_update().await {
                Ok(u) if u.available => {
                    update_open.set(true);
                }
                _ => {}
            }
        });
    });

    view! {
        <Show
            when=move || !auth.loading.get() && auth.session.get().is_some()
            fallback=move || {
                view! {
                    <div class="grid min-h-screen place-items-center text-lg text-[var(--muted)]">
                        {move || i18n.t("shell.loading")}
                    </div>
                }
            }
        >
            <div class=move || {
                let fullscreen = fs.active.get();
                cn(&[
                    "min-h-screen lg:h-screen lg:overflow-hidden",
                    if fullscreen {
                        "flex flex-col"
                    } else {
                        "lg:grid"
                    },
                    if fullscreen {
                        ""
                    } else if collapsed.get() {
                        "lg:grid-cols-[4.5rem_1fr]"
                    } else {
                        "lg:grid-cols-[18rem_1fr]"
                    },
                ])
            }>
                <Show when=move || !fs.active.get()>
                    <Show when=move || nav_open.get()>
                        <div
                            class="fixed inset-0 z-20 bg-black/40 lg:hidden"
                            on:click=move |_| nav_open.set(false)
                        ></div>
                    </Show>
                    <Sidebar
                        open=nav_open.into()
                        collapsed=collapsed.into()
                        on_navigate=Callback::new(move |_| nav_open.set(false))
                        on_check_update=Callback::new(move |_| {
                            update_open.set(true);
                        })
                        version_label=version_label.into()
                    />
                </Show>
                <div class="flex min-h-0 min-w-0 flex-1 flex-col lg:h-screen lg:overflow-hidden">
                    <Show when=move || !fs.active.get()>
                        <Navbar
                            on_toggle_nav=Callback::new(move |_| {
                                nav_open.update(|v| *v = !*v);
                            })
                            on_toggle_collapse=Callback::new(move |_| {
                                collapsed.update(|v| *v = !*v);
                            })
                        />
                    </Show>
                    <main class=move || {
                        cn(&[
                            "flex min-h-0 flex-1 flex-col overflow-y-auto",
                            if fs.active.get() {
                                "p-3 sm:p-4"
                            } else {
                                "p-4 sm:p-6"
                            },
                        ])
                    }>
                        <Outlet />
                    </main>
                </div>
                <TableFullscreenCloseButton />
            </div>
            <UpdateCheckModal open=update_open auto_check=true />
            <OnboardingTour />
        </Show>
    }
}
