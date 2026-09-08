use leptos::prelude::*;

use crate::app::components::ui::Modal;
use crate::app::i18n::use_i18n;

/// Contextual help modal for auth pages or the dashboard.
#[component]
pub fn HelpDialog(
    open: RwSignal<bool>,
    /// `"auth"` | `"dashboard"`
    #[prop(default = "auth")] context: &'static str,
) -> impl IntoView {
    let i18n = use_i18n();
    let is_dashboard = context == "dashboard";

    view! {
        <Modal
            open=open.into()
            on_close=Callback::new(move |_| open.set(false))
            title_signal=Signal::derive(move || i18n.t("help.title"))
        >
            <div class="flex flex-col gap-4 text-base leading-relaxed text-[var(--fg)]">
                <section>
                    <h3 class="mb-1 font-semibold text-[var(--brand)]">
                        {move || i18n.t("help.login_title")}
                    </h3>
                    <p class="text-[var(--muted)]">{move || i18n.t("help.login_body")}</p>
                </section>
                <section>
                    <h3 class="mb-1 font-semibold text-[var(--brand)]">
                        {move || i18n.t("help.recover_title")}
                    </h3>
                    <p class="text-[var(--muted)]">{move || i18n.t("help.recover_body")}</p>
                </section>
                <Show when=move || is_dashboard>
                    <section>
                        <h3 class="mb-1 font-semibold text-[var(--brand)]">
                            {move || i18n.t("help.nav_title")}
                        </h3>
                        <p class="text-[var(--muted)]">{move || i18n.t("help.nav_body")}</p>
                    </section>
                    <section>
                        <h3 class="mb-1 font-semibold text-[var(--brand)]">
                            {move || i18n.t("help.cotisations_title")}
                        </h3>
                        <p class="text-[var(--muted)]">{move || i18n.t("help.cotisations_body")}</p>
                    </section>
                </Show>
            </div>
        </Modal>
    }
}
