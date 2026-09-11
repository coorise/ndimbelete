//! Root App — router + providers.

mod components;
mod hooks;
mod i18n;
mod lib;
mod pages;

use leptos::prelude::*;
use leptos_router::components::{ParentRoute, Route, Router, Routes};
use leptos_router::path;

use components::layout::DashboardShell;
use hooks::AuthProvider;
use i18n::{use_i18n, I18nProvider};
use lib::provide_theme;
use pages::{
    CollaborationPage, CotisationsPage, LandingPage, LoginPage, MembersPage, OverviewPage,
    PlanningPage, ProfilePage, SettingsPage, SetupPage, StaffPage,
};

#[component]
pub fn App() -> impl IntoView {
    provide_theme();

    view! {
        <I18nProvider>
            <AuthProvider>
                <Router>
                    <AppRoutes />
                </Router>
            </AuthProvider>
        </I18nProvider>
    }
}

#[component]
fn AppRoutes() -> impl IntoView {
    let i18n = use_i18n();

    view! {
        <Routes fallback=move || {
            view! {
                <div class="grid min-h-screen place-items-center p-6">
                    <div class="text-center">
                        <h1 class="font-display text-3xl">{i18n.t("not_found")}</h1>
                        <a class="mt-4 inline-block text-[var(--brand)] underline" href="/">
                            {move || i18n.t("back_home")}
                        </a>
                    </div>
                </div>
            }
        }>
            <Route path=path!("/") view=LandingPage />
            <Route path=path!("/setup") view=SetupPage />
            <Route path=path!("/login") view=LoginPage />
            <ParentRoute path=path!("/app") view=DashboardShell>
                <Route path=path!("") view=OverviewPage />
                <Route path=path!("staff") view=StaffPage />
                <Route path=path!("members") view=MembersPage />
                <Route path=path!("cotisations") view=CotisationsPage />
                <Route path=path!("planning") view=PlanningPage />
                <Route path=path!("profile") view=ProfilePage />
                <Route path=path!("settings") view=SettingsPage />
                <Route path=path!("collaboration") view=CollaborationPage />
            </ParentRoute>
        </Routes>
    }
}
