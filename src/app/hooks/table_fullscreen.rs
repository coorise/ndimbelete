use leptos::prelude::*;

#[derive(Clone, Copy)]
pub struct TableFullscreenCtx {
    pub active: RwSignal<bool>,
}

pub fn provide_table_fullscreen() {
    provide_context(TableFullscreenCtx {
        active: RwSignal::new(false),
    });
}

pub fn use_table_fullscreen() -> TableFullscreenCtx {
    use_context::<TableFullscreenCtx>()
        .expect("TableFullscreenCtx missing — wrap dashboard in provide_table_fullscreen()")
}
