use leptos::prelude::*;

use crate::app::lib::cn;

/// Excel-like cell: click to edit, blur/Enter commits locally via `on_commit`.
#[component]
pub fn EditableCell(
    /// Display / source value
    value: Signal<String>,
    /// Called when the user finishes editing with a changed value.
    #[prop(into)]
    on_commit: Callback<String>,
    #[prop(optional)] class: &'static str,
    #[prop(optional)] input_type: &'static str,
    #[prop(optional)] placeholder: &'static str,
) -> impl IntoView {
    let editing = RwSignal::new(false);
    let draft = RwSignal::new(String::new());
    let input_ref = NodeRef::<leptos::html::Input>::new();
    let r#type = if input_type.is_empty() {
        "text"
    } else {
        input_type
    };

    Effect::new(move |_| {
        if editing.get() {
            if let Some(el) = input_ref.get() {
                let _ = el.focus();
                let _ = el.select();
            }
        }
    });

    let commit = move || {
        if !editing.get_untracked() {
            return;
        }
        let next = draft.get_untracked();
        let prev = value.get_untracked();
        editing.set(false);
        if next != prev {
            on_commit.run(next);
        }
    };

    view! {
        <div class=cn(&["editable-cell min-w-[3.5rem]", class])>
            <Show
                when=move || editing.get()
                fallback=move || {
                    view! {
                        <button
                            type="button"
                            class="editable-cell-display w-full rounded px-1 py-0.5 text-left hover:bg-[color-mix(in_srgb,var(--brand-yellow)_28%,transparent)]"
                            title="Cliquer pour modifier"
                            on:click=move |_| {
                                draft.set(value.get_untracked());
                                editing.set(true);
                            }
                        >
                            {move || {
                                let v = value.get();
                                if v.is_empty() {
                                    placeholder.to_string()
                                } else {
                                    v
                                }
                            }}
                        </button>
                    }
                }
            >
                <input
                    node_ref=input_ref
                    type=r#type
                    class="editable-cell-input w-full rounded border border-[var(--brand)] bg-[var(--bg-elevated)] px-1 py-0.5 text-sm outline-none"
                    prop:value=move || draft.get()
                    on:input=move |ev| draft.set(event_target_value(&ev))
                    on:blur=move |_| commit()
                    on:keydown=move |ev| {
                        let key = ev.key();
                        if key == "Enter" || key == "Tab" {
                            // Tab: commit then allow default focus move to next cell.
                            if key == "Enter" {
                                ev.prevent_default();
                            }
                            commit();
                        } else if key == "Escape" {
                            editing.set(false);
                        }
                    }
                />
            </Show>
        </div>
    }
}
