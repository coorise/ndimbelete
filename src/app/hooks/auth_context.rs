//! Session signal provider (auth context).

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::app::lib::{api, SessionInfo};

const SESSION_KEY: &str = "ndimbelente.sessionStaffId";

fn persist_staff_id(id: Option<&str>) {
    let Some(ls) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) else {
        return;
    };
    match id {
        Some(id) => {
            let _ = ls.set_item(SESSION_KEY, id);
        }
        None => {
            let _ = ls.remove_item(SESSION_KEY);
        }
    }
}

#[derive(Clone, Copy)]
pub struct AuthContext {
    pub session: RwSignal<Option<SessionInfo>>,
    pub loading: RwSignal<bool>,
    pub error: RwSignal<Option<String>>,
}

impl AuthContext {
    /// Load session from the Tauri backend (restored from disk on app start).
    pub fn refresh(self) {
        self.loading.set(true);
        spawn_local(async move {
            match api::get_current_session().await {
                Ok(s) => {
                    if let Some(ref info) = s {
                        persist_staff_id(Some(&info.staff.id));
                    }
                    self.session.set(s);
                    self.error.set(None);
                }
                Err(e) => {
                    self.session.set(None);
                    self.error.set(Some(e));
                }
            }
            self.loading.set(false);
        });
    }

    pub fn set_session(self, info: Option<SessionInfo>) {
        match &info {
            Some(s) => persist_staff_id(Some(&s.staff.id)),
            None => persist_staff_id(None),
        }
        self.session.set(info);
        self.error.set(None);
    }

    pub fn logout(self) {
        spawn_local(async move {
            let _ = api::logout().await;
            persist_staff_id(None);
            self.session.set(None);
        });
    }

    /// Reactive check for a single permission key (`*` and write→read included).
    pub fn can(self, key: &'static str) -> Signal<bool> {
        Signal::derive(move || {
            self.session
                .get()
                .map(|s| crate::app::lib::permissions_allow(&s.permissions, key))
                .unwrap_or(false)
        })
    }

    /// Reactive check: any of the listed permissions.
    pub fn can_any(self, keys: &'static [&'static str]) -> Signal<bool> {
        Signal::derive(move || {
            self.session
                .get()
                .map(|s| crate::app::lib::permissions_allow_any(&s.permissions, keys))
                .unwrap_or(false)
        })
    }

    pub fn is_founder(self) -> Signal<bool> {
        Signal::derive(move || {
            self.session
                .get()
                .map(|s| s.staff.username == "root" || s.staff.is_founder)
                .unwrap_or(false)
        })
    }
}

pub fn provide_auth() -> AuthContext {
    let ctx = AuthContext {
        session: RwSignal::new(None),
        loading: RwSignal::new(true),
        error: RwSignal::new(None),
    };
    provide_context(ctx);
    ctx.refresh();
    ctx
}

pub fn use_auth() -> AuthContext {
    use_context::<AuthContext>().expect("AuthContext missing — wrap app in AuthProvider")
}

#[component]
pub fn AuthProvider(children: Children) -> impl IntoView {
    provide_auth();
    children()
}
