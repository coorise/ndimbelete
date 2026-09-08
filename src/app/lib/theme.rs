//! Light / dark theme + CSS variable helpers.

use leptos::prelude::*;
use wasm_bindgen::JsCast;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Light,
    Dark,
}

impl ColorMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::Light,
        }
    }
}

fn document_el() -> Option<web_sys::HtmlElement> {
    web_sys::window()?
        .document()?
        .document_element()?
        .dyn_into()
        .ok()
}

pub fn apply_color_mode(mode: ColorMode) {
    if let Some(el) = document_el() {
        match mode {
            ColorMode::Dark => {
                let _ = el.class_list().add_1("dark");
            }
            ColorMode::Light => {
                let _ = el.class_list().remove_1("dark");
            }
        }
        let _ = el.set_attribute("data-theme", mode.as_str());
    }
}

pub fn apply_font_scale(scale: f64) {
    if let Some(el) = document_el() {
        let _ = el
            .style()
            .set_property("--font-scale", &scale.to_string());
    }
}

pub fn apply_theme_color(color: &str) {
    if let Some(el) = document_el() {
        let _ = el.style().set_property("--brand", color);
        let _ = el.style().set_property("--brand-green", color);
    }
}

/// Provide a shared color-mode signal for the shell.
#[derive(Clone, Copy)]
pub struct ThemeCtx {
    pub mode: RwSignal<ColorMode>,
}

pub fn provide_theme() -> ThemeCtx {
    let stored = web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|ls| ls.get_item("ndimbelente-theme").ok().flatten());
    let initial = if stored.as_deref() == Some("dark") {
        ColorMode::Dark
    } else {
        ColorMode::Light
    };
    apply_color_mode(initial);
    let mode = RwSignal::new(initial);
    Effect::new(move |_| {
        let m = mode.get();
        apply_color_mode(m);
        if let Some(ls) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
            let _ = ls.set_item("ndimbelente-theme", m.as_str());
        }
    });
    let ctx = ThemeCtx { mode };
    provide_context(ctx);
    ctx
}

pub fn use_theme() -> ThemeCtx {
    use_context::<ThemeCtx>().expect("ThemeCtx missing — wrap app in provide_theme()")
}
