use leptos::prelude::*;

use crate::app::components::ui::{Button, ButtonVariant, Modal};

/// A reusable confirmation dialog modal component.
#[component]
pub fn ConfirmModal(
    open: Signal<bool>,
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] on_confirm: Callback<()>,
    #[prop(optional, into)] title: Signal<String>,
    #[prop(into)] message: Signal<String>,
    #[prop(optional, into)] confirm_label: Signal<String>,
    #[prop(optional, into)] cancel_label: Signal<String>,
    #[prop(optional)] confirm_variant: ButtonVariant,
) -> impl IntoView {
    let title_sig = Signal::derive(move || {
        let t = title.get();
        if t.trim().is_empty() {
            "Confirmation".to_string()
        } else {
            t
        }
    });
    let confirm_text = Signal::derive(move || {
        let c = confirm_label.get();
        if c.trim().is_empty() {
            "Confirmer".to_string()
        } else {
            c
        }
    });
    let cancel_text = Signal::derive(move || {
        let c = cancel_label.get();
        if c.trim().is_empty() {
            "Annuler".to_string()
        } else {
            c
        }
    });
    let variant = if confirm_variant == ButtonVariant::Primary {
        ButtonVariant::Danger
    } else {
        confirm_variant
    };

    view! {
        <Modal
            open=open
            on_close=on_close
            title_signal=title_sig
        >
            <div class="flex flex-col gap-4">
                <p class="text-sm leading-relaxed text-[var(--muted)]">
                    {move || message.get()}
                </p>
                <div class="flex justify-end gap-2 pt-3 border-t border-[var(--border)]">
                    <Button
                        variant=ButtonVariant::Secondary
                        on_click=Callback::new(move |_| on_close.run(()))
                    >
                        {move || cancel_text.get()}
                    </Button>
                    <Button
                        variant=variant
                        on_click=Callback::new(move |_| on_confirm.run(()))
                    >
                        {move || confirm_text.get()}
                    </Button>
                </div>
            </div>
        </Modal>
    }
}
