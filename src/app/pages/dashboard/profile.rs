use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::app::components::ui::{Button, Input};
use crate::app::hooks::use_auth;
use crate::app::i18n::use_i18n;
use crate::app::lib::{api, UpdateMyProfileInput};

fn split_full_name(full: &str) -> (String, String) {
    let parts: Vec<&str> = full.split_whitespace().collect();
    match parts.as_slice() {
        [] => (String::new(), String::new()),
        [only] => ((*only).to_string(), String::new()),
        [first, rest @ ..] => ((*first).to_string(), rest.join(" ")),
    }
}

#[component]
pub fn ProfilePage() -> impl IntoView {
    let auth = use_auth();
    let i18n = use_i18n();

    let last_name = RwSignal::new(String::new());
    let first_name = RwSignal::new(String::new());
    let phone = RwSignal::new(String::new());
    let recovery_school = RwSignal::new(String::new());
    let recovery_color = RwSignal::new(String::new());
    let error = RwSignal::new(Option::<String>::None);
    let success = RwSignal::new(false);
    let busy = RwSignal::new(false);
    let has_hints = RwSignal::new(false);

    let current_password = RwSignal::new(String::new());
    let new_password = RwSignal::new(String::new());
    let confirm_password = RwSignal::new(String::new());
    let pw_error = RwSignal::new(Option::<String>::None);
    let pw_success = RwSignal::new(false);
    let pw_busy = RwSignal::new(false);

    Effect::new(move |_| {
        if let Some(session) = auth.session.get() {
            let mut first = session.staff.first_name.clone();
            let mut last = session.staff.last_name.clone();
            if first.trim().is_empty() && last.trim().is_empty() {
                let (f, l) = split_full_name(&session.staff.full_name);
                first = f;
                last = l;
            }
            first_name.set(first);
            last_name.set(last);
            phone.set(session.staff.phone.clone().unwrap_or_default());
            has_hints.set(session.staff.has_recovery_hints);
        }
    });

    view! {
        <div class="flex min-h-0 w-full flex-1 flex-col gap-5 overflow-y-auto">
            <div>
                <h1 class="font-display text-3xl font-semibold">{move || i18n.t("profile.title")}</h1>
                <p class="text-[var(--muted)]">{move || i18n.t("profile.subtitle")}</p>
            </div>

            <form
                class="flex flex-col gap-4"
                on:submit=move |ev: SubmitEvent| {
                    ev.prevent_default();
                    error.set(None);
                    success.set(false);

                    let last = last_name.get_untracked().trim().to_string();
                    let first = first_name.get_untracked().trim().to_string();
                    if last.is_empty() && first.is_empty() {
                        error.set(Some("Le nom et le prénom sont obligatoires.".into()));
                        return;
                    }

                    let phone_v = phone.get_untracked().trim().to_string();
                    let school = recovery_school.get_untracked();
                    let color = recovery_color.get_untracked();

                    busy.set(true);
                    spawn_local(async move {
                        match api::update_my_profile(UpdateMyProfileInput {
                            first_name: first,
                            last_name: last,
                            phone: if phone_v.is_empty() { None } else { Some(phone_v) },
                            recovery_school: if school.trim().is_empty() {
                                None
                            } else {
                                Some(school)
                            },
                            recovery_color: if color.trim().is_empty() {
                                None
                            } else {
                                Some(color)
                            },
                        })
                        .await
                        {
                            Ok(staff) => {
                                auth.session.update(|s| {
                                    if let Some(session) = s.as_mut() {
                                        session.staff = staff.clone();
                                    }
                                });
                                has_hints.set(staff.has_recovery_hints);
                                recovery_school.set(String::new());
                                recovery_color.set(String::new());
                                success.set(true);
                            }
                            Err(e) => error.set(Some(e)),
                        }
                        busy.set(false);
                    });
                }
            >
                <Input
                    label="Nom"
                    value=last_name.into()
                    on_input=Callback::new(move |v| last_name.set(v))
                    required=true
                />
                <Input
                    label="Prénom"
                    value=first_name.into()
                    on_input=Callback::new(move |v| first_name.set(v))
                    required=true
                />
                <Input
                    label_key="profile.phone"
                    value=phone.into()
                    on_input=Callback::new(move |v| phone.set(v))
                />

                <div class="rounded-xl border border-[var(--border)] p-4">
                    <h2 class="font-semibold">{move || i18n.t("profile.recovery_title")}</h2>
                    <p class="mt-1 text-sm text-[var(--muted)]">
                        {move || {
                            if has_hints.get() {
                                i18n.t("profile.recovery_configured")
                            } else {
                                i18n.t("profile.recovery_missing")
                            }
                        }}
                    </p>
                    <div class="mt-4 flex flex-col gap-3">
                        <Input
                            label_key="profile.recovery_school"
                            r#type="password"
                            value=recovery_school.into()
                            on_input=Callback::new(move |v| recovery_school.set(v))
                            placeholder_key="profile.recovery_placeholder"
                        />
                        <Input
                            label_key="profile.recovery_color"
                            r#type="password"
                            value=recovery_color.into()
                            on_input=Callback::new(move |v| recovery_color.set(v))
                            placeholder_key="profile.recovery_placeholder"
                        />
                    </div>
                </div>

                <Show when=move || error.get().is_some()>
                    <p
                        class="rounded-xl bg-[color-mix(in_srgb,var(--brand-red)_12%,transparent)] px-3 py-2 text-[var(--brand-red)]"
                        role="alert"
                    >
                        {move || error.get().unwrap_or_default()}
                    </p>
                </Show>
                <Show when=move || success.get()>
                    <p class="rounded-xl bg-[color-mix(in_srgb,var(--brand)_12%,transparent)] px-3 py-2 text-[var(--brand)]">
                        {move || i18n.t("profile.saved")}
                    </p>
                </Show>

                <Button r#type="submit">
                    {move || {
                        if busy.get() {
                            i18n.t("common.loading")
                        } else {
                            i18n.t("common.save")
                        }
                    }}
                </Button>
            </form>

            <div class="rounded-xl border border-[var(--border)] p-4">
                <h2 class="font-semibold">"Mot de passe"</h2>
                <p class="mt-1 text-sm text-[var(--muted)]">
                    "Changez votre mot de passe en saisissant l'actuel, puis le nouveau."
                </p>
                <form
                    class="mt-4 flex flex-col gap-3"
                    on:submit=move |ev: SubmitEvent| {
                        ev.prevent_default();
                        pw_error.set(None);
                        pw_success.set(false);

                        let current = current_password.get_untracked();
                        let new_pw = new_password.get_untracked();
                        let confirm = confirm_password.get_untracked();

                        if current.is_empty() {
                            pw_error.set(Some("Le mot de passe actuel est obligatoire.".into()));
                            return;
                        }
                        if new_pw.len() < 8 {
                            pw_error.set(Some(i18n.t_static("login.password_short")));
                            return;
                        }
                        if new_pw != confirm {
                            pw_error.set(Some(i18n.t_static("login.password_mismatch")));
                            return;
                        }

                        pw_busy.set(true);
                        spawn_local(async move {
                            match api::change_password(current, new_pw).await {
                                Ok(()) => {
                                    current_password.set(String::new());
                                    new_password.set(String::new());
                                    confirm_password.set(String::new());
                                    pw_success.set(true);
                                }
                                Err(e) => pw_error.set(Some(e)),
                            }
                            pw_busy.set(false);
                        });
                    }
                >
                    <Input
                        label="Mot de passe actuel"
                        r#type="password"
                        value=current_password.into()
                        on_input=Callback::new(move |v| current_password.set(v))
                        required=true
                    />
                    <Input
                        label="Nouveau mot de passe"
                        r#type="password"
                        value=new_password.into()
                        on_input=Callback::new(move |v| new_password.set(v))
                        required=true
                    />
                    <Input
                        label="Confirmer le mot de passe"
                        r#type="password"
                        value=confirm_password.into()
                        on_input=Callback::new(move |v| confirm_password.set(v))
                        required=true
                    />
                    <Show when=move || pw_error.get().is_some()>
                        <p
                            class="rounded-xl bg-[color-mix(in_srgb,var(--brand-red)_12%,transparent)] px-3 py-2 text-[var(--brand-red)]"
                            role="alert"
                        >
                            {move || pw_error.get().unwrap_or_default()}
                        </p>
                    </Show>
                    <Show when=move || pw_success.get()>
                        <p class="rounded-xl bg-[color-mix(in_srgb,var(--brand)_12%,transparent)] px-3 py-2 text-[var(--brand)]">
                            "Mot de passe mis à jour."
                        </p>
                    </Show>
                    <Button r#type="submit">
                        {move || {
                            if pw_busy.get() {
                                i18n.t("common.loading")
                            } else {
                                "Changer le mot de passe".to_string()
                            }
                        }}
                    </Button>
                </form>
            </div>
        </div>
    }
}
