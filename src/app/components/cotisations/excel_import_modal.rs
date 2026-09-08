use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::app::components::ui::{
    Button, ButtonVariant, Input, Modal, Progress, Select, SelectOption, Table, TBody, Td, Th, THead,
    Tr,
};
use crate::app::lib::{api, ImportFailure};

#[component]
pub fn ImportFailuresModal(
    open: RwSignal<bool>,
    failures: Signal<Vec<ImportFailure>>,
    #[prop(into)] on_ignore: Callback<()>,
    #[prop(into)] on_retry: Callback<()>,
) -> impl IntoView {
    view! {
        <Modal
            open=open.into()
            title="Échecs d'import"
            wide=true
            on_close=Callback::new(move |_| {
                open.set(false);
                on_retry.run(());
            })
        >
            <div class="flex flex-col gap-4">
                <p class="text-sm text-[var(--muted)]">
                    {move || format!("{} ligne(s) n'ont pas pu être importées.", failures.get().len())}
                </p>
                <div class="max-h-72 overflow-auto rounded-xl border border-[var(--border)]">
                    <Table class="text-sm">
                        <THead>
                            <Th>"Ligne"</Th>
                            <Th>"Motif"</Th>
                        </THead>
                        <TBody>
                            <For
                                each=move || failures.get()
                                key=|f| format!("{}-{}", f.row, f.reason)
                                children=move |f| {
                                    view! {
                                        <Tr>
                                            <Td class="font-semibold">{f.row}</Td>
                                            <Td>{f.reason}</Td>
                                        </Tr>
                                    }
                                }
                            />
                        </TBody>
                    </Table>
                </div>
                <div class="flex flex-wrap justify-end gap-2">
                    <Button
                        variant=ButtonVariant::Secondary
                        on_click=Callback::new(move |_| {
                            open.set(false);
                            on_retry.run(());
                        })
                    >
                        "Relancer l'import"
                    </Button>
                    <Button on_click=Callback::new(move |_| {
                        open.set(false);
                        on_ignore.run(());
                    })>
                        "Ignorer et terminer"
                    </Button>
                </div>
            </div>
        </Modal>
    }
}

#[component]
pub fn ExcelImportModal(
    open: RwSignal<bool>,
    year: Signal<i32>,
    #[prop(into)] on_done: Callback<()>,
) -> impl IntoView {
    let path = RwSignal::new(String::new());
    let sheets = RwSignal::new(Vec::<String>::new());
    let sheet = RwSignal::new(String::new());
    let headers = RwSignal::new(Vec::<String>::new());
    let sample_rows = RwSignal::new(Vec::<Vec<String>>::new());
    let progress = RwSignal::new(0.0_f64);
    let message = RwSignal::new(Option::<String>::None);
    let success = RwSignal::new(Option::<String>::None);
    let error = RwSignal::new(Option::<String>::None);
    let busy = RwSignal::new(false);
    let failures_open = RwSignal::new(false);
    let failures = RwSignal::new(Vec::<ImportFailure>::new());

    let reset_state = move || {
        path.set(String::new());
        sheets.set(Vec::new());
        sheet.set(String::new());
        headers.set(Vec::new());
        sample_rows.set(Vec::new());
        progress.set(0.0);
        message.set(None);
        success.set(None);
        error.set(None);
        busy.set(false);
        failures.set(Vec::new());
        failures_open.set(false);
    };

    Effect::new(move |_| {
        if open.get() {
            reset_state();
        }
    });

    let apply_preview = move |prev: crate::app::lib::ExcelPreview, preferred_sheet: Option<String>| {
        sheets.set(prev.sheets.clone());
        let chosen = preferred_sheet
            .filter(|s| prev.sheets.iter().any(|n| n == s))
            .or_else(|| prev.sheets.first().cloned())
            .unwrap_or_default();
        sheet.set(chosen);
        headers.set(prev.headers);
        sample_rows.set(prev.sample_rows);
        error.set(None);
        success.set(None);
        message.set(None);
    };

    let run_preview = move |p: String, sheet_opt: Option<String>| {
        if p.is_empty() {
            error.set(Some("Indiquez un chemin de fichier.".into()));
            return;
        }
        spawn_local(async move {
            match api::preview_excel(&p, sheet_opt.as_deref()).await {
                Ok(prev) => apply_preview(prev, sheet_opt),
                Err(e) => {
                    message.set(None);
                    success.set(None);
                    error.set(Some(e));
                }
            }
        });
    };

    let sheet_options = Signal::derive(move || {
        sheets
            .get()
            .into_iter()
            .map(|s| SelectOption {
                value: s.clone(),
                label: s,
            })
            .collect::<Vec<_>>()
    });

    view! {
        <Modal
            open=open.into()
            title="Importer Excel"
            wide=true
            on_close=Callback::new(move |_| open.set(false))
        >
            <div class="flex flex-col gap-4">
                <div class="flex flex-col gap-2 sm:flex-row">
                    <div class="flex-1">
                        <Input
                            label="Chemin du fichier"
                            value=path.into()
                            on_input=Callback::new(move |v| path.set(v))
                            placeholder="C:\\…\\cotisations.xlsm"
                        />
                    </div>
                    <div class="flex items-end">
                        <Button
                            variant=ButtonVariant::Secondary
                            on_click=Callback::new(move |_| {
                                spawn_local(async move {
                                    if let Some(p) = api::pick_excel_file().await {
                                        path.set(p.clone());
                                        run_preview(p, None);
                                    }
                                });
                            })
                        >
                            "Parcourir…"
                        </Button>
                    </div>
                </div>

                <Button
                    variant=ButtonVariant::Secondary
                    on_click=Callback::new(move |_| {
                        let p = path.get_untracked();
                        let s = sheet.get_untracked();
                        run_preview(p, if s.is_empty() { None } else { Some(s) });
                    })
                >
                    "Prévisualiser"
                </Button>

                <Show when=move || !sheets.get().is_empty()>
                    <Select
                        label="Feuille"
                        options=sheet_options
                        value=sheet.into()
                        on_change=Callback::new(move |v: String| {
                            sheet.set(v.clone());
                            let p = path.get_untracked();
                            run_preview(p, Some(v));
                        })
                    />
                </Show>

                <Show when=move || !headers.get().is_empty()>
                    <div class="max-h-56 overflow-auto rounded-xl border border-[var(--border)]">
                        <Table class="text-xs">
                            <THead>
                                {move || {
                                    headers
                                        .get()
                                        .into_iter()
                                        .map(|h| view! { <Th class="whitespace-nowrap">{h}</Th> })
                                        .collect_view()
                                }}
                            </THead>
                            <TBody>
                                <For
                                    each=move || {
                                        sample_rows
                                            .get()
                                            .into_iter()
                                            .enumerate()
                                            .collect::<Vec<_>>()
                                    }
                                    key=|(i, _)| *i
                                    children=move |(_, row)| {
                                        view! {
                                            <Tr>
                                                {row
                                                    .into_iter()
                                                    .map(|c| view! { <Td class="whitespace-nowrap">{c}</Td> })
                                                    .collect_view()}
                                            </Tr>
                                        }
                                    }
                                />
                            </TBody>
                        </Table>
                    </div>
                </Show>

                <Progress value=progress.into() label="Progression import" />

                <Show when=move || message.get().is_some()>
                    <p class="text-[var(--muted)]">{move || message.get().unwrap_or_default()}</p>
                </Show>
                <Show when=move || success.get().is_some()>
                    <p class="rounded-xl bg-[color-mix(in_srgb,var(--brand)_14%,transparent)] px-3 py-2 font-medium text-[var(--brand)]">
                        {move || success.get().unwrap_or_default()}
                    </p>
                </Show>
                <Show when=move || error.get().is_some()>
                    <p class="rounded-xl bg-[color-mix(in_srgb,var(--brand-red)_12%,transparent)] px-3 py-2 text-[var(--brand-red)]">
                        {move || error.get().unwrap_or_default()}
                    </p>
                </Show>

                <div class="flex justify-end gap-2">
                    <Button variant=ButtonVariant::Secondary on_click=Callback::new(move |_| open.set(false))>
                        "Fermer"
                    </Button>
                    <Button
                        on_click=Callback::new(move |_| {
                            if busy.get_untracked() {
                                return;
                            }
                            let p = path.get_untracked();
                            let s = sheet.get_untracked();
                            let y = year.get_untracked();
                            if p.is_empty() || s.is_empty() {
                                message.set(None);
                                success.set(None);
                                error.set(Some("Fichier et feuille requis.".into()));
                                return;
                            }
                            busy.set(true);
                            progress.set(0.15);
                            error.set(None);
                            success.set(None);
                            message.set(Some("Import en cours…".into()));
                            spawn_local(async move {
                                progress.set(0.55);
                                match api::import_excel(&p, &s, y).await {
                                    Ok(res) => {
                                        progress.set(1.0);
                                        message.set(None);
                                        if res.failed.is_empty() {
                                            success.set(Some(format!(
                                                "{} ligne(s) importée(s) (paiements inclus). {} membre(s) absent(s) du fichier retiré(s).",
                                                res.ok,
                                                res.removed
                                            )));
                                            on_done.run(());
                                            gloo_timers::future::TimeoutFuture::new(900).await;
                                            open.set(false);
                                        } else {
                                            success.set(Some(format!(
                                                "{} ligne(s) importée(s), {} échec(s), {} retiré(s).",
                                                res.ok,
                                                res.failed.len(),
                                                res.removed
                                            )));
                                            failures.set(res.failed);
                                            failures_open.set(true);
                                            on_done.run(());
                                        }
                                    }
                                    Err(e) => {
                                        progress.set(0.0);
                                        message.set(None);
                                        success.set(None);
                                        error.set(Some(e));
                                    }
                                }
                                busy.set(false);
                            });
                        })
                    >
                        {move || if busy.get() { "Import…" } else { "Lancer l'import" }}
                    </Button>
                </div>
            </div>
        </Modal>

        <ImportFailuresModal
            open=failures_open
            failures=failures.into()
            on_ignore=Callback::new(move |_| {
                open.set(false);
                on_done.run(());
            })
            on_retry=Callback::new(move |_| {
                // Keep import modal open for retry; clear transient progress only.
                progress.set(0.0);
                message.set(None);
            })
        />
    }
}
