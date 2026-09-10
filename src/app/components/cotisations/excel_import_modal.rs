use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::app::components::ui::{
    Button, ButtonVariant, EditableCell, Input, Modal, Progress, Select, SelectOption, Table, TBody,
    Td, Th, THead, Tr,
};
use crate::app::lib::{api, ImportFailure, DEFAULT_VIREMENT_ROLE_VALUES};

fn parse_year_from_sheet(name: &str) -> Option<i32> {
    let t = name.trim();
    if let Ok(y) = t.parse::<i32>() {
        if (1990..=2100).contains(&y) {
            return Some(y);
        }
    }
    // e.g. "Cotisation 2016" / "2016-feuille"
    for part in t.split(|c: char| !c.is_ascii_digit()) {
        if let Ok(y) = part.parse::<i32>() {
            if (1990..=2100).contains(&y) {
                return Some(y);
            }
        }
    }
    None
}

fn year_options_around(center: i32) -> Vec<SelectOption> {
    let start = (center - 8).max(2000);
    let end = (center + 4).max(start + 1);
    (start..=end)
        .map(|y| SelectOption {
            value: y.to_string(),
            label: y.to_string(),
        })
        .collect()
}

fn row_matches_query(cells: &[String], query: &str) -> bool {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return true;
    }
    cells.iter().any(|c| c.to_lowercase().contains(&q))
}

fn default_role_selection(values: &[crate::app::lib::VirementValueCount]) -> Vec<String> {
    values
        .iter()
        .filter(|v| {
            let t = v.value.trim();
            !t.eq_ignore_ascii_case("VIREMENT")
                && DEFAULT_VIREMENT_ROLE_VALUES
                    .iter()
                    .any(|d| d.eq_ignore_ascii_case(t))
        })
        .map(|v| v.value.clone())
        .collect()
}

#[component]
fn EditableGrid(
    headers: Signal<Vec<String>>,
    rows: RwSignal<Vec<Vec<String>>>,
    search: RwSignal<String>,
    /// Optional leading columns (e.g. Ligne / Motif) rendered read-only before cells.
    #[prop(optional)]
    leading: Option<Signal<Vec<Vec<String>>>>,
    #[prop(optional)]
    max_height_class: &'static str,
) -> impl IntoView {
    let page = RwSignal::new(0usize);
    let page_size = 40usize;
    let height = if max_height_class.is_empty() {
        "max-h-64"
    } else {
        max_height_class
    };
    let show_leading = leading.is_some();
    let leading_data = leading.unwrap_or_else(|| Signal::derive(|| Vec::<Vec<String>>::new()));

    Effect::new(move |_| {
        let _ = search.get();
        page.set(0);
    });

    let filtered_indices = Signal::derive(move || {
        let q = search.get();
        rows.get()
            .into_iter()
            .enumerate()
            .filter(|(_, cells)| row_matches_query(cells, &q))
            .map(|(i, _)| i)
            .collect::<Vec<_>>()
    });

    let page_indices = Signal::derive(move || {
        let idxs = filtered_indices.get();
        let start = page.get().saturating_mul(page_size);
        idxs.into_iter().skip(start).take(page_size).collect::<Vec<_>>()
    });

    let total_filtered = Signal::derive(move || filtered_indices.get().len());
    let page_count = Signal::derive(move || {
        let n = total_filtered.get();
        if n == 0 {
            1
        } else {
            (n + page_size - 1) / page_size
        }
    });

    view! {
        <div class="flex flex-col gap-2">
            <Input
                label="Recherche"
                value=search.into()
                on_input=Callback::new(move |v| search.set(v))
                placeholder="Filtrer les lignes…"
            />
            <p class="text-xs text-[var(--muted)]">
                {move || {
                    format!(
                        "{} ligne(s) — page {}/{} — cliquez une cellule pour modifier",
                        total_filtered.get(),
                        page.get() + 1,
                        page_count.get()
                    )
                }}
            </p>
            <div class=format!("{height} overflow-auto rounded-xl border border-[var(--border)]")>
                <Table class="text-xs">
                    <THead>
                        <Show when=move || show_leading>
                            <Th class="whitespace-nowrap">"Ligne"</Th>
                            <Th class="whitespace-nowrap">"Motif"</Th>
                        </Show>
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
                            each=move || page_indices.get()
                            key=|i| *i
                            children=move |row_i| {
                                view! {
                                    <Tr>
                                        <Show when=move || show_leading>
                                            {move || {
                                                let pair = leading_data
                                                    .get()
                                                    .get(row_i)
                                                    .cloned()
                                                    .unwrap_or_else(|| vec![String::new(); 2]);
                                                let ligne = pair.first().cloned().unwrap_or_default();
                                                let motif = pair.get(1).cloned().unwrap_or_default();
                                                view! {
                                                    <>
                                                        <Td class="font-semibold whitespace-nowrap">{ligne}</Td>
                                                        <Td class="whitespace-nowrap text-[var(--brand-red)]">{motif}</Td>
                                                    </>
                                                }
                                            }}
                                        </Show>
                                        {move || {
                                            let ncols = headers.get().len().max(
                                                rows.get().get(row_i).map(|r| r.len()).unwrap_or(0),
                                            );
                                            (0..ncols)
                                                .map(|col_i| {
                                                    let value = Signal::derive(move || {
                                                        rows.get()
                                                            .get(row_i)
                                                            .and_then(|r| r.get(col_i))
                                                            .cloned()
                                                            .unwrap_or_default()
                                                    });
                                                    view! {
                                                        <Td class="whitespace-nowrap align-top">
                                                            <EditableCell
                                                                value=value
                                                                on_commit=Callback::new(move |v| {
                                                                    rows.update(|all| {
                                                                        if let Some(row) = all.get_mut(row_i) {
                                                                            while row.len() <= col_i {
                                                                                row.push(String::new());
                                                                            }
                                                                            row[col_i] = v;
                                                                        }
                                                                    });
                                                                })
                                                                placeholder="—"
                                                            />
                                                        </Td>
                                                    }
                                                })
                                                .collect_view()
                                        }}
                                    </Tr>
                                }
                            }
                        />
                    </TBody>
                </Table>
            </div>
            <div class="flex items-center justify-end gap-2">
                <Button
                    variant=ButtonVariant::Secondary
                    on_click=Callback::new(move |_| {
                        page.update(|p| *p = p.saturating_sub(1));
                    })
                >
                    "‹"
                </Button>
                <Button
                    variant=ButtonVariant::Secondary
                    on_click=Callback::new(move |_| {
                        let max = page_count.get_untracked().saturating_sub(1);
                        page.update(|p| *p = (*p + 1).min(max));
                    })
                >
                    "›"
                </Button>
            </div>
        </div>
    }
}

#[component]
pub fn ImportFailuresModal(
    open: RwSignal<bool>,
    failures: RwSignal<Vec<ImportFailure>>,
    headers: Signal<Vec<String>>,
    import_year: Signal<i32>,
    sheet_label: Signal<String>,
    role_values: Signal<Vec<String>>,
    #[prop(into)] on_ignore: Callback<()>,
    #[prop(into)] on_retry_done: Callback<crate::app::lib::ImportResult>,
) -> impl IntoView {
    let search = RwSignal::new(String::new());
    let edited_rows = RwSignal::new(Vec::<Vec<String>>::new());
    let leading = RwSignal::new(Vec::<Vec<String>>::new());
    let busy = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);

    Effect::new(move |_| {
        if open.get() {
            let fails = failures.get();
            edited_rows.set(
                fails
                    .iter()
                    .map(|f| f.cells.clone())
                    .collect(),
            );
            leading.set(
                fails
                    .iter()
                    .map(|f| vec![f.row.to_string(), f.reason.clone()])
                    .collect(),
            );
            search.set(String::new());
            error.set(None);
            busy.set(false);
        }
    });

    let leading_sig = Signal::derive(move || leading.get());

    view! {
        <Modal
            open=open.into()
            title="Échecs d'import"
            wide=true
            on_close=Callback::new(move |_| {
                open.set(false);
            })
        >
            <div class="flex flex-col gap-4">
                <p class="text-sm text-[var(--muted)]">
                    {move || format!(
                        "{} ligne(s) en échec — corrigez les cellules puis réimportez uniquement ces lignes.",
                        failures.get().len()
                    )}
                </p>

                <Show when=move || !edited_rows.get().is_empty()>
                    <EditableGrid
                        headers=headers
                        rows=edited_rows
                        search=search
                        leading=leading_sig
                        max_height_class="max-h-80"
                    />
                </Show>

                <Show when=move || error.get().is_some()>
                    <p class="rounded-xl bg-[color-mix(in_srgb,var(--brand-red)_12%,transparent)] px-3 py-2 text-[var(--brand-red)]">
                        {move || error.get().unwrap_or_default()}
                    </p>
                </Show>

                <div class="flex flex-wrap justify-end gap-2">
                    <Button
                        variant=ButtonVariant::Secondary
                        on_click=Callback::new(move |_| {
                            if busy.get_untracked() {
                                return;
                            }
                            let hdrs = headers.get_untracked();
                            let rows = edited_rows.get_untracked();
                            let y = import_year.get_untracked();
                            let sheet = sheet_label.get_untracked();
                            let roles = role_values.get_untracked();
                            busy.set(true);
                            error.set(None);
                            spawn_local(async move {
                                match api::import_excel_grid(
                                    &hdrs,
                                    &rows,
                                    y,
                                    Some(&sheet),
                                    Some(&roles),
                                    false,
                                )
                                .await
                                {
                                    Ok(res) => {
                                        if res.failed.is_empty() {
                                            failures.set(Vec::new());
                                            open.set(false);
                                        } else {
                                            failures.set(res.failed.clone());
                                        }
                                        on_retry_done.run(res);
                                    }
                                    Err(e) => error.set(Some(e)),
                                }
                                busy.set(false);
                            });
                        })
                    >
                        {move || if busy.get() { "Réimport…" } else { "Réimporter les lignes corrigées" }}
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
    let import_year = RwSignal::new(2026_i32);
    let headers = RwSignal::new(Vec::<String>::new());
    let preview_rows = RwSignal::new(Vec::<Vec<String>>::new());
    let preview_search = RwSignal::new(String::new());
    let has_virement = RwSignal::new(false);
    let virement_values = RwSignal::new(Vec::<crate::app::lib::VirementValueCount>::new());
    let virement_empty = RwSignal::new(0usize);
    let selected_roles = RwSignal::new(Vec::<String>::new());
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
        import_year.set(year.get_untracked());
        headers.set(Vec::new());
        preview_rows.set(Vec::new());
        preview_search.set(String::new());
        has_virement.set(false);
        virement_values.set(Vec::new());
        virement_empty.set(0);
        selected_roles.set(Vec::new());
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
        // Always bind the dropdown to the sheet that was actually read.
        let loaded = if !prev.selected_sheet.trim().is_empty() {
            prev.selected_sheet.clone()
        } else {
            preferred_sheet
                .filter(|s| prev.sheets.iter().any(|n| n == s))
                .or_else(|| prev.sheets.first().cloned())
                .unwrap_or_default()
        };
        sheet.set(loaded.clone());
        if let Some(y) = parse_year_from_sheet(&loaded) {
            import_year.set(y);
        } else {
            import_year.set(year.get_untracked());
        }
        headers.set(prev.headers);
        preview_rows.set(prev.sample_rows);
        preview_search.set(String::new());
        has_virement.set(prev.has_virement_column);
        virement_values.set(prev.virement_values.clone());
        virement_empty.set(prev.virement_empty_count);
        selected_roles.set(default_role_selection(&prev.virement_values));
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
            // Prefer a year-named sheet on first open when none selected.
            let preferred = match sheet_opt {
                Some(s) if !s.trim().is_empty() => Some(s),
                _ => None,
            };
            match api::preview_excel(&p, preferred.as_deref()).await {
                Ok(prev) => {
                    let y = year.get_untracked().to_string();
                    let wants_year_sheet = preferred.is_none()
                        && prev.sheets.iter().any(|n| n.trim() == y)
                        && prev.selected_sheet.trim() != y;
                    if wants_year_sheet {
                        match api::preview_excel(&p, Some(&y)).await {
                            Ok(prev2) => apply_preview(prev2, Some(y)),
                            Err(_) => apply_preview(prev, preferred),
                        }
                    } else {
                        apply_preview(prev, preferred);
                    }
                }
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

    let year_opts = Signal::derive(move || {
        let mut opts = year_options_around(import_year.get());
        // Ensure current selection is present.
        let y = import_year.get().to_string();
        if !opts.iter().any(|o| o.value == y) {
            opts.insert(
                0,
                SelectOption {
                    value: y.clone(),
                    label: y,
                },
            );
        }
        opts
    });

    let headers_sig = Signal::derive(move || headers.get());
    let sheet_sig = Signal::derive(move || sheet.get());
    let year_sig = Signal::derive(move || import_year.get());
    let roles_sig = Signal::derive(move || selected_roles.get());

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
                    <div class="grid gap-3 sm:grid-cols-2">
                        <Select
                            label="Feuille"
                            options=sheet_options
                            value=sheet.into()
                            on_change=Callback::new(move |v: String| {
                                sheet.set(v.clone());
                                if let Some(y) = parse_year_from_sheet(&v) {
                                    import_year.set(y);
                                }
                                let p = path.get_untracked();
                                run_preview(p, Some(v));
                            })
                        />
                        <Select
                            label="Année (base / planning)"
                            options=year_opts
                            value=Signal::derive(move || import_year.get().to_string())
                            on_change=Callback::new(move |v: String| {
                                if let Ok(y) = v.parse::<i32>() {
                                    import_year.set(y);
                                }
                            })
                        />
                    </div>
                    <p class="text-xs text-[var(--muted)]">
                        "L'import crée / met à jour l'année choisie et ses périodes de planning (cotisations)."
                    </p>
                </Show>

                <Show when=move || !headers.get().is_empty()>
                    <EditableGrid
                        headers=headers_sig
                        rows=preview_rows
                        search=preview_search
                        max_height_class="max-h-72"
                    />
                </Show>

                <Show when=move || has_virement.get() && !virement_values.get().is_empty()>
                    <div class="rounded-xl border border-[var(--border)] bg-[var(--surface)] p-4">
                        <p class="font-medium">"Colonne VIREMENT BANQUAIRE — rôles"</p>
                        <p class="mt-1 text-sm text-[var(--muted)]">
                            "Cochez les valeurs qui sont des rôles (ex. Commissaire, EXEMPTE). "
                            "« VIREMENT » reste un mode de paiement (virement bancaire) ; "
                            {move || format!("les cellules vides ({}) = espèces / rôle Normal.", virement_empty.get())}
                        </p>
                        <div class="mt-3 max-h-48 space-y-2 overflow-auto">
                            <For
                                each=move || virement_values.get()
                                key=|v| v.value.clone()
                                children=move |v| {
                                    let value = v.value.clone();
                                    let value_check = v.value.clone();
                                    let count = v.count;
                                    let is_virement = value.eq_ignore_ascii_case("VIREMENT");
                                    view! {
                                        <label class="flex items-center gap-2 text-sm">
                                            <input
                                                type="checkbox"
                                                class="h-4 w-4"
                                                prop:disabled=is_virement
                                                prop:checked=move || {
                                                    selected_roles.get().iter().any(|s| s == &value_check)
                                                }
                                                on:change=move |_| {
                                                    if is_virement {
                                                        return;
                                                    }
                                                    selected_roles.update(|sel| {
                                                        if let Some(i) = sel.iter().position(|s| s == &value) {
                                                            sel.remove(i);
                                                        } else {
                                                            sel.push(value.clone());
                                                        }
                                                    });
                                                }
                                            />
                                            <span class="font-medium">{v.value.clone()}</span>
                                            <span class="text-[var(--muted)]">{format!("({count})")}</span>
                                            <Show when=move || is_virement>
                                                <span class="text-xs text-[var(--muted)]">"— paiement virement"</span>
                                            </Show>
                                        </label>
                                    }
                                }
                            />
                        </div>
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
                            let s = sheet.get_untracked();
                            let y = import_year.get_untracked();
                            let roles = selected_roles.get_untracked();
                            let hdrs = headers.get_untracked();
                            let rows = preview_rows.get_untracked();
                            if hdrs.is_empty() || rows.is_empty() {
                                message.set(None);
                                success.set(None);
                                error.set(Some("Prévisualisez une feuille avant d'importer.".into()));
                                return;
                            }
                            busy.set(true);
                            progress.set(0.15);
                            error.set(None);
                            success.set(None);
                            message.set(Some(format!("Import {y} en cours…")));
                            spawn_local(async move {
                                // Ensure year + planning periods exist before grid import.
                                if let Err(e) = api::ensure_year(y).await {
                                    progress.set(0.0);
                                    message.set(None);
                                    success.set(None);
                                    error.set(Some(e));
                                    busy.set(false);
                                    return;
                                }
                                progress.set(0.55);
                                match api::import_excel_grid(
                                    &hdrs,
                                    &rows,
                                    y,
                                    Some(&s),
                                    Some(&roles),
                                    true,
                                )
                                .await
                                {
                                    Ok(res) => {
                                        progress.set(1.0);
                                        message.set(None);
                                        if res.failed.is_empty() {
                                            success.set(Some(format!(
                                                "{} ligne(s) importée(s) pour {}. {} membre(s) absent(s) du fichier retiré(s).",
                                                res.ok,
                                                y,
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
            failures=failures
            headers=headers_sig
            import_year=year_sig
            sheet_label=sheet_sig
            role_values=roles_sig
            on_ignore=Callback::new(move |_| {
                open.set(false);
                on_done.run(());
            })
            on_retry_done=Callback::new(move |res: crate::app::lib::ImportResult| {
                if res.failed.is_empty() {
                    success.set(Some(format!(
                        "Échecs corrigés : {} ligne(s) réimportée(s).",
                        res.ok
                    )));
                    on_done.run(());
                } else {
                    success.set(Some(format!(
                        "Réimport partiel : {} ok, {} encore en échec.",
                        res.ok,
                        res.failed.len()
                    )));
                }
            })
        />
    }
}
