use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::app::lib::cn;

#[derive(Clone, PartialEq, Eq)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

#[component]
pub fn Select(
    #[prop(optional)] label: &'static str,
    options: Signal<Vec<SelectOption>>,
    value: Signal<String>,
    #[prop(into)] on_change: Callback<String>,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    view! {
        <label class=cn(&[
            "flex min-w-0 flex-col gap-1.5 text-sm font-medium",
            if class.is_empty() { "w-auto" } else { class },
        ])>
            {(!label.is_empty()).then(|| view! { <span>{label}</span> })}
            <select
                class="tap-target w-full rounded-xl border border-[var(--border)] bg-[var(--bg-elevated)] px-3 py-2.5 text-base focus:border-[var(--brand)] focus:outline-none focus:ring-2 focus:ring-[color-mix(in_srgb,var(--brand)_35%,transparent)]"
                prop:value=move || value.get()
                on:change=move |ev| on_change.run(event_target_value(&ev))
            >
            <For
                each=move || options.get()
                key=|o| o.value.clone()
                children=move |o| {
                    let val = o.value.clone();
                    let val_selected = o.value.clone();
                    view! {
                        <option
                            value=val.clone()
                            selected=move || value.get() == val_selected
                        >
                            {o.label}
                        </option>
                    }
                }
            />
            </select>
        </label>
    }
}

/// Combobox with type-ahead filter (name / identifier).
#[component]
pub fn SearchableSelect(
    #[prop(optional)] label: &'static str,
    options: Signal<Vec<SelectOption>>,
    value: Signal<String>,
    #[prop(into)] on_change: Callback<String>,
    #[prop(optional)] class: &'static str,
    #[prop(optional)] placeholder: &'static str,
    #[prop(optional)] search_placeholder: &'static str,
) -> impl IntoView {
    let open = RwSignal::new(false);
    let query = RwSignal::new(String::new());
    let root_ref = NodeRef::<leptos::html::Div>::new();
    let search_ref = NodeRef::<leptos::html::Input>::new();

    let placeholder = if placeholder.is_empty() {
        "Sélectionner…"
    } else {
        placeholder
    };
    let search_placeholder = if search_placeholder.is_empty() {
        "Rechercher nom ou n°…"
    } else {
        search_placeholder
    };

    let selected_label = Signal::derive(move || {
        let v = value.get();
        options
            .get()
            .into_iter()
            .find(|o| o.value == v)
            .map(|o| o.label)
            .unwrap_or_else(|| placeholder.to_string())
    });

    let filtered = Signal::derive(move || {
        let q = query.get().trim().to_lowercase();
        let list = options.get();
        if q.is_empty() {
            return list;
        }
        list.into_iter()
            .filter(|o| o.label.to_lowercase().contains(&q) || o.value.to_lowercase().contains(&q))
            .collect::<Vec<_>>()
    });

    // Close when clicking outside (listener registered once).
    Effect::new(move |_| {
        let Some(document) = web_sys::window().and_then(|w| w.document()) else {
            return;
        };
        let handler = wasm_bindgen::closure::Closure::<dyn FnMut(_)>::new({
            move |ev: web_sys::MouseEvent| {
                if !open.get_untracked() {
                    return;
                }
                let Some(root) = root_ref.get_untracked() else {
                    return;
                };
                if let Some(target) = ev.target().and_then(|t| t.dyn_into::<web_sys::Node>().ok()) {
                    if !root.contains(Some(&target)) {
                        open.set(false);
                        query.set(String::new());
                    }
                }
            }
        });
        let _ = document
            .add_event_listener_with_callback("mousedown", handler.as_ref().unchecked_ref());
        handler.forget();
    });

    Effect::new(move |_| {
        if open.get() {
            if let Some(input) = search_ref.get() {
                let _ = input.focus();
            }
        }
    });

    view! {
        <div
            node_ref=root_ref
            class=cn(&[
                "relative flex min-w-0 flex-col gap-1.5 text-sm font-medium",
                if class.is_empty() { "w-auto" } else { class },
            ])
        >
            {(!label.is_empty()).then(|| view! { <span>{label}</span> })}
            <button
                type="button"
                class="tap-target flex w-full items-center justify-between gap-2 rounded-xl border border-[var(--border)] bg-[var(--bg-elevated)] px-3 py-2.5 text-left text-base focus:border-[var(--brand)] focus:outline-none focus:ring-2 focus:ring-[color-mix(in_srgb,var(--brand)_35%,transparent)]"
                aria-expanded=move || open.get()
                on:click=move |_| {
                    let will_open = !open.get_untracked();
                    open.set(will_open);
                    if !will_open {
                        query.set(String::new());
                    }
                }
            >
                <span class=move || {
                    if value.get().is_empty() && selected_label.get() == placeholder {
                        "truncate text-[var(--muted)]"
                    } else {
                        "truncate"
                    }
                }>
                    {move || selected_label.get()}
                </span>
                <span class="shrink-0 text-[var(--muted)]">"▾"</span>
            </button>

            <Show when=move || open.get()>
                <div class="absolute left-0 right-0 top-[calc(100%-0.15rem)] z-50 mt-1 overflow-hidden rounded-xl border border-[var(--border)] bg-[var(--surface)] shadow-[var(--shadow)]">
                    <div class="border-b border-[var(--border)] p-2">
                        <input
                            node_ref=search_ref
                            type="search"
                            class="tap-target w-full rounded-lg border border-[var(--border)] bg-[var(--bg-elevated)] px-3 py-2 text-sm focus:border-[var(--brand)] focus:outline-none"
                            placeholder=search_placeholder
                            prop:value=move || query.get()
                            on:input=move |ev| query.set(event_target_value(&ev))
                            on:keydown=move |ev| {
                                if ev.key() == "Escape" {
                                    open.set(false);
                                    query.set(String::new());
                                }
                            }
                        />
                    </div>
                    <ul class="max-h-56 overflow-y-auto py-1" role="listbox">
                        <Show
                            when=move || !filtered.get().is_empty()
                            fallback=move || {
                                view! {
                                    <li class="px-3 py-2 text-sm text-[var(--muted)]">"Aucun résultat"</li>
                                }
                            }
                        >
                            <For
                                each=move || filtered.get()
                                key=|o| o.value.clone()
                                children=move |o| {
                                    let val = o.value.clone();
                                    let val_sel = o.value.clone();
                                    let label_v = o.label.clone();
                                    view! {
                                        <li>
                                            <button
                                                type="button"
                                                role="option"
                                                class=move || {
                                                    if value.get() == val_sel {
                                                        "flex w-full px-3 py-2 text-left text-sm font-semibold bg-[color-mix(in_srgb,var(--brand)_14%,transparent)]"
                                                    } else {
                                                        "flex w-full px-3 py-2 text-left text-sm hover:bg-[color-mix(in_srgb,var(--fg)_6%,transparent)]"
                                                    }
                                                }
                                                on:click=move |_| {
                                                    on_change.run(val.clone());
                                                    open.set(false);
                                                    query.set(String::new());
                                                }
                                            >
                                                {label_v}
                                            </button>
                                        </li>
                                    }
                                }
                            />
                        </Show>
                    </ul>
                </div>
            </Show>
        </div>
    }
}

/// Multi-select dropdown (checkbox list). Empty `value` = no selection (caller treats as “all”).
#[component]
pub fn MultiSelect(
    #[prop(optional)] label: &'static str,
    options: Signal<Vec<SelectOption>>,
    value: Signal<Vec<String>>,
    #[prop(into)] on_change: Callback<Vec<String>>,
    #[prop(optional)] class: &'static str,
    #[prop(optional)] empty_label: &'static str,
) -> impl IntoView {
    let open = RwSignal::new(false);
    let root_ref = NodeRef::<leptos::html::Div>::new();
    let empty_label = if empty_label.is_empty() {
        "Toute l'année"
    } else {
        empty_label
    };

    let summary = Signal::derive(move || {
        let selected = value.get();
        let opts = options.get();
        if selected.is_empty() {
            return empty_label.to_string();
        }
        let labels: Vec<String> = opts
            .into_iter()
            .filter(|o| selected.iter().any(|v| v == &o.value))
            .map(|o| o.label)
            .collect();
        if labels.is_empty() {
            empty_label.to_string()
        } else if labels.len() <= 2 {
            labels.join(", ")
        } else {
            format!("{} mois", labels.len())
        }
    });

    Effect::new(move |_| {
        let Some(document) = web_sys::window().and_then(|w| w.document()) else {
            return;
        };
        let handler = wasm_bindgen::closure::Closure::<dyn FnMut(_)>::new({
            move |ev: web_sys::MouseEvent| {
                if !open.get_untracked() {
                    return;
                }
                let Some(root) = root_ref.get_untracked() else {
                    return;
                };
                if let Some(target) = ev.target().and_then(|t| t.dyn_into::<web_sys::Node>().ok()) {
                    if !root.contains(Some(&target)) {
                        open.set(false);
                    }
                }
            }
        });
        let _ = document
            .add_event_listener_with_callback("mousedown", handler.as_ref().unchecked_ref());
        handler.forget();
    });

    view! {
        <div
            node_ref=root_ref
            class=cn(&[
                "relative flex min-w-0 flex-col gap-1.5 text-sm font-medium",
                if class.is_empty() { "w-auto" } else { class },
            ])
        >
            {(!label.is_empty()).then(|| view! { <span>{label}</span> })}
            <button
                type="button"
                class="tap-target flex w-full items-center justify-between gap-2 rounded-xl border border-[var(--border)] bg-[var(--bg-elevated)] px-3 py-2.5 text-left text-base focus:border-[var(--brand)] focus:outline-none focus:ring-2 focus:ring-[color-mix(in_srgb,var(--brand)_35%,transparent)]"
                aria-expanded=move || open.get()
                on:click=move |_| open.update(|o| *o = !*o)
            >
                <span class=move || {
                    if value.get().is_empty() {
                        "truncate text-[var(--muted)]"
                    } else {
                        "truncate"
                    }
                }>
                    {move || summary.get()}
                </span>
                <span class="shrink-0 text-[var(--muted)]">"▾"</span>
            </button>

            <Show when=move || open.get()>
                <div class="absolute left-0 right-0 top-[calc(100%-0.15rem)] z-50 mt-1 overflow-hidden rounded-xl border border-[var(--border)] bg-[var(--surface)] shadow-[var(--shadow)]">
                    <div class="border-b border-[var(--border)] px-2 py-1.5">
                        <button
                            type="button"
                            class="w-full rounded-lg px-2 py-1.5 text-left text-sm font-semibold text-[var(--brand)] hover:bg-[color-mix(in_srgb,var(--brand)_10%,transparent)]"
                            on:click=move |_| {
                                on_change.run(Vec::new());
                            }
                        >
                            {empty_label}
                        </button>
                    </div>
                    <ul class="max-h-56 overflow-y-auto py-1" role="listbox">
                        <For
                            each=move || options.get()
                            key=|o| o.value.clone()
                            children=move |o| {
                                let val = o.value.clone();
                                let val_chk = o.value.clone();
                                let label_v = o.label.clone();
                                view! {
                                    <li>
                                        <label class="flex cursor-pointer items-center gap-2 px-3 py-2 text-sm hover:bg-[color-mix(in_srgb,var(--fg)_6%,transparent)]">
                                            <input
                                                type="checkbox"
                                                class="h-4 w-4 accent-[var(--brand)]"
                                                prop:checked=move || {
                                                    value.get().iter().any(|v| v == &val_chk)
                                                }
                                                on:change=move |_| {
                                                    let mut next = value.get_untracked();
                                                    if let Some(i) = next.iter().position(|v| v == &val) {
                                                        next.remove(i);
                                                    } else {
                                                        next.push(val.clone());
                                                    }
                                                    on_change.run(next);
                                                }
                                            />
                                            <span class="truncate">{label_v}</span>
                                        </label>
                                    </li>
                                }
                            }
                        />
                    </ul>
                </div>
            </Show>
        </div>
    }
}
