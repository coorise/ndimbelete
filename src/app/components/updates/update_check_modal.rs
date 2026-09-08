use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::app::components::ui::{Button, ButtonVariant, Modal};
use crate::app::lib::{api, UpdateInfo};

#[component]
pub fn UpdateCheckModal(
    open: RwSignal<bool>,
    /// When true, run the GitHub check as soon as the modal opens.
    #[prop(optional)]
    auto_check: bool,
) -> impl IntoView {
    let checking = RwSignal::new(false);
    let installing = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);
    let info = RwSignal::new(Option::<UpdateInfo>::None);

    let run_check = move || {
        if checking.get_untracked() || installing.get_untracked() {
            return;
        }
        checking.set(true);
        error.set(None);
        info.set(None);
        spawn_local(async move {
            match api::check_for_update().await {
                Ok(u) => info.set(Some(u)),
                Err(e) => error.set(Some(e)),
            }
            checking.set(false);
        });
    };

    Effect::new(move |_| {
        if open.get() && auto_check {
            run_check();
        }
    });

    view! {
        <Modal
            open=open.into()
            on_close=Callback::new(move |_| {
                if !installing.get_untracked() {
                    open.set(false);
                }
            })
            title="Mise à jour"
        >
            <div class="flex flex-col gap-4">
                <Show when=move || checking.get()>
                    <p class="text-[var(--muted)]">"Recherche d’une nouvelle version…"</p>
                </Show>

                <Show when=move || error.get().is_some()>
                    <p class="rounded-xl bg-[color-mix(in_srgb,var(--brand-red)_12%,transparent)] px-3 py-2 text-[var(--brand-red)]" role="alert">
                        {move || error.get().unwrap_or_default()}
                    </p>
                </Show>

                {move || {
                    info.get().map(|u| {
                        let available = u.available;
                        view! {
                            <div class="flex flex-col gap-3 text-sm">
                                <p>{u.message.clone()}</p>
                                <dl class="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-[var(--muted)]">
                                    <dt>"Canal"</dt>
                                    <dd class="font-medium text-[var(--fg)]">{u.channel.clone()}</dd>
                                    <dt>"Version actuelle"</dt>
                                    <dd class="font-medium text-[var(--fg)]">{u.current_version.clone()}</dd>
                                    <dt>"Dernière version"</dt>
                                    <dd class="font-medium text-[var(--fg)]">{u.latest_version.clone()}</dd>
                                </dl>
                                <Show when=move || available>
                                    <p class="text-xs text-[var(--muted)]">
                                        "L’installateur remplacera l’application installée, puis le fichier d’installation sera supprimé."
                                    </p>
                                </Show>
                            </div>
                        }
                    })
                }}

                <div class="flex flex-wrap justify-end gap-2">
                    <Button
                        variant=ButtonVariant::Secondary
                        on_click=Callback::new(move |_| run_check())
                    >
                        "Vérifier"
                    </Button>
                    <Show when=move || info.get().map(|u| u.available).unwrap_or(false)>
                        <Button
                            on_click=Callback::new(move |_| {
                                if installing.get_untracked() {
                                    return;
                                }
                                installing.set(true);
                                error.set(None);
                                spawn_local(async move {
                                    if let Err(e) = api::download_and_install_update().await {
                                        error.set(Some(e));
                                        installing.set(false);
                                    }
                                });
                            })
                        >
                            {move || {
                                if installing.get() {
                                    "Téléchargement / installation…"
                                } else {
                                    "Télécharger et installer"
                                }
                            }}
                        </Button>
                    </Show>
                    <Button
                        variant=ButtonVariant::Ghost
                        on_click=Callback::new(move |_| {
                            if !installing.get_untracked() {
                                open.set(false);
                            }
                        })
                    >
                        {move || {
                            if info.get().map(|u| u.available).unwrap_or(false) {
                                "Ignorer"
                            } else {
                                "Fermer"
                            }
                        }}
                    </Button>
                </div>
            </div>
        </Modal>
    }
}
