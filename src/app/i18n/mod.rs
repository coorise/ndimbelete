//! Lightweight FR/EN/ES/DE i18n (no leptos_i18n — Trunk CSR friendly).

mod dict;

use leptos::prelude::*;

const STORAGE_KEY: &str = "ndimbelente.locale";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Locale {
    Fr,
    En,
    Es,
    De,
}

impl Locale {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fr => "fr",
            Self::En => "en",
            Self::Es => "es",
            Self::De => "de",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "fr" => Some(Self::Fr),
            "en" => Some(Self::En),
            "es" => Some(Self::Es),
            "de" => Some(Self::De),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Fr => "FR",
            Self::En => "EN",
            Self::Es => "ES",
            Self::De => "DE",
        }
    }

    pub const ALL: [Locale; 4] = [Self::Fr, Self::En, Self::Es, Self::De];
}

#[derive(Clone, Copy)]
pub struct I18nContext {
    pub locale: RwSignal<Locale>,
}

impl I18nContext {
    /// Look up a translation for the current locale (reactive when called in a reactive scope).
    pub fn t(self, key: &'static str) -> String {
        dict::lookup(self.locale.get(), key)
    }

    /// Non-reactive lookup (e.g. inside event handlers).
    pub fn t_static(self, key: &'static str) -> String {
        dict::lookup(self.locale.get_untracked(), key)
    }

    pub fn set_locale(self, locale: Locale) {
        self.locale.set(locale);
    }
}

fn load_stored_locale() -> Locale {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|ls| ls.get_item(STORAGE_KEY).ok().flatten())
        .and_then(|s| Locale::from_str(&s))
        .unwrap_or(Locale::Fr)
}

fn persist_locale(locale: Locale) {
    if let Some(ls) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = ls.set_item(STORAGE_KEY, locale.as_str());
    }
    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
        if let Some(el) = doc.document_element() {
            let _ = el.set_attribute("lang", locale.as_str());
        }
    }
}

pub fn provide_i18n() -> I18nContext {
    let initial = load_stored_locale();
    persist_locale(initial);
    let locale = RwSignal::new(initial);
    Effect::new(move |_| {
        persist_locale(locale.get());
    });
    let ctx = I18nContext { locale };
    provide_context(ctx);
    ctx
}

pub fn use_i18n() -> I18nContext {
    use_context::<I18nContext>().expect("I18nContext missing — wrap app in I18nProvider")
}

#[component]
pub fn I18nProvider(children: Children) -> impl IntoView {
    provide_i18n();
    children()
}

/// Compact language select for the navbar.
#[component]
pub fn LanguageSwitcher() -> impl IntoView {
    let i18n = use_i18n();

    view! {
        <label class="flex items-center gap-1 text-sm text-[var(--muted)]">
            <select
                class="tap-target rounded-xl border border-[var(--border)] bg-[var(--bg-elevated)] px-2 py-2 text-sm font-semibold focus:border-[var(--brand)] focus:outline-none focus:ring-2 focus:ring-[color-mix(in_srgb,var(--brand)_35%,transparent)]"
                prop:value=move || i18n.locale.get().as_str()
                aria-label=move || i18n.t("nav.language")
                on:change=move |ev| {
                    let v = event_target_value(&ev);
                    if let Some(loc) = Locale::from_str(&v) {
                        i18n.set_locale(loc);
                    }
                }
            >
                {Locale::ALL
                    .into_iter()
                    .map(|loc| {
                        view! {
                            <option value=loc.as_str()>{loc.label()}</option>
                        }
                    })
                    .collect_view()}
            </select>
        </label>
    }
}
