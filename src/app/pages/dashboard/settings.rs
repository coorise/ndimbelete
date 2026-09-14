use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::app::components::settings::ReceiptFormBuilder;
use crate::app::components::ui::{
    Button, ButtonVariant, Input, RichTextEditor, TabItem, Tabs, TextArea,
};
use crate::app::i18n::use_i18n;
use crate::app::lib::{
    api, apply_font_scale, apply_theme_color, AppSettings, ReceiptField,
};

fn html_to_preview_text(html: &str) -> String {
    let with_breaks = html
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("</p>", "\n")
        .replace("</div>", "\n");
    let mut out = String::new();
    let mut in_tag = false;
    for c in with_breaks.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

fn fields_to_html(fields: &[ReceiptField]) -> String {
    let mut sorted = fields.to_vec();
    sorted.sort_by(|a, b| a.position.cmp(&b.position));
    sorted
        .into_iter()
        .map(|f| {
            let mut inner = f.content.replace('&', "&amp;").replace('<', "&lt;");
            if f.bold {
                inner = format!("<b>{inner}</b>");
            }
            if f.italic {
                inner = format!("<i>{inner}</i>");
            }
            if inner.is_empty() {
                "<div><br></div>".into()
            } else {
                format!(
                    "<div style=\"font-size:{}pt;text-align:{};color:{}\">{inner}</div>",
                    f.font_size,
                    f.align,
                    f.color
                )
            }
        })
        .collect()
}

fn render_template_preview(tpl: &str, settings: &AppSettings) -> String {
    let now = js_sys::Date::new_0();
    let date = format!(
        "{:02}/{:02}/{} {:02}:{:02}",
        now.get_date(),
        now.get_month() + 1,
        now.get_full_year(),
        now.get_hours(),
        now.get_minutes()
    );
    let filled = tpl
        .replace("{{ORG.NAME}}", &settings.org_name)
        .replace("{{ORG.ADDRESS}}", &settings.org_address)
        .replace("{{USER.NAME}}", "AIDARA THEMOKHO")
        .replace("{{USER.CARD}}", "AT2")
        .replace("{{USER.UID}}", "demo-uid")
        .replace("{{COTISATION.DAY}}", "15")
        .replace("{{COTISATION.MONTH}}", "07")
        .replace("{{COTISATION.YEAR}}", "2026")
        .replace("{{PAYMENT_DATE}}", "14/09/2026")
        .replace("{{YEAR_TOTAL_DUE}}", "120.00 €")
        .replace("{{PAYMENT_METHOD}}", "Espèces")
        .replace("{{RECEIVED_AMOUNT}}", "20.00 €")
        .replace("{{REMAINING_DEBT}}", "220.00 €")
        .replace("{{PREVIOUS_DEBT}}", "220.00 €")
        .replace("{{NEW_BALANCE}}", "160.00 €")
        .replace("{{SURPLUS_LINE}}", "")
        .replace("{{DATE}}", &date);
    html_to_preview_text(&filled)
}

#[component]
pub fn SettingsPage() -> impl IntoView {
    let i18n = use_i18n();
    let settings = RwSignal::new(AppSettings::default());
    let message = RwSignal::new(Option::<String>::None);
    let error = RwSignal::new(Option::<String>::None);
    let tab = RwSignal::new("general");
    let receipt_mode = RwSignal::new("mini".to_string());

    Effect::new(move |_| {
        spawn_local(async move {
            match api::get_settings().await {
                Ok(s) => {
                    apply_font_scale(s.font_scale);
                    apply_theme_color(&s.theme_color);
                    settings.set(s);
                }
                Err(e) => error.set(Some(e)),
            }
        });
    });

    let org_name = Signal::derive(move || settings.get().org_name);
    let org_description = Signal::derive(move || settings.get().org_description);
    let org_address = Signal::derive(move || settings.get().org_address);
    let theme_color = Signal::derive(move || settings.get().theme_color);
    let logo_path = Signal::derive(move || settings.get().logo_path);
    let currency_unit = Signal::derive(move || settings.get().currency_unit);
    let font_scale_label = Signal::derive(move || format!("{:.1}", settings.get().font_scale));
    let mini_width = Signal::derive(move || format!("{}", settings.get().receipt_mini_width_mm));

    let editor_mode = Signal::derive(move || {
        let s = settings.get();
        if receipt_mode.get() == "mini" {
            s.receipt_editor_mode_mini
        } else {
            s.receipt_editor_mode_a4
        }
    });

    let template_html = Signal::derive(move || {
        let s = settings.get();
        if receipt_mode.get() == "mini" {
            s.receipt_template_mini
        } else {
            s.receipt_template_a4
        }
    });

    let fields_list = Signal::derive(move || {
        let s = settings.get();
        if receipt_mode.get() == "mini" {
            s.receipt_fields_mini
        } else {
            s.receipt_fields_a4
        }
    });

    let allow_color = Signal::derive(move || receipt_mode.get() != "mini");

    let tab_items = Signal::derive(move || {
        vec![
            TabItem {
                id: "general",
                label: "Générale".into(),
            },
            TabItem {
                id: "receipt",
                label: "Modèle de Reçu".into(),
            },
        ]
    });

    let preview = Signal::derive(move || {
        let s = settings.get();
        let is_mini = receipt_mode.get() == "mini";
        let mode = if is_mini {
            s.receipt_editor_mode_mini.as_str()
        } else {
            s.receipt_editor_mode_a4.as_str()
        };
        let tpl = if mode.eq_ignore_ascii_case("fields") {
            if is_mini {
                fields_to_html(&s.receipt_fields_mini)
            } else {
                fields_to_html(&s.receipt_fields_a4)
            }
        } else if is_mini {
            s.receipt_template_mini.clone()
        } else {
            s.receipt_template_a4.clone()
        };
        render_template_preview(&tpl, &s)
    });

    let set_editor_mode = move |mode: &str| {
        let m = mode.to_string();
        settings.update(|s| {
            let is_mini = receipt_mode.get_untracked() == "mini";
            if is_mini {
                s.receipt_editor_mode_mini = m;
            } else {
                s.receipt_editor_mode_a4 = m;
            }
        });
    };

    view! {
        <div class="flex min-h-0 w-full flex-1 flex-col gap-4 overflow-hidden">
            <div class="shrink-0">
                <h1 class="font-display text-3xl font-semibold">{move || i18n.t("settings.title")}</h1>
                <p class="text-[var(--muted)]">"Organisation, sauvegarde et modèles de reçu."</p>
            </div>

            <div class="shrink-0">
                <Tabs items=tab_items active=tab />
            </div>

            <Show when=move || message.get().is_some()>
                <p class="shrink-0 text-[var(--brand)]">{move || message.get().unwrap_or_default()}</p>
            </Show>
            <Show when=move || error.get().is_some()>
                <p class="shrink-0 text-[var(--brand-red)]">{move || error.get().unwrap_or_default()}</p>
            </Show>

            <div class="min-h-0 flex-1 overflow-y-auto">
            <Show when=move || tab.get() == "general">
                <div class="flex flex-col gap-4 rounded-2xl border border-[var(--border)] bg-[var(--surface)] p-5 shadow-[var(--shadow)]">
                    <Input
                        label="Nom de l’organisation"
                        value=org_name
                        on_input=Callback::new(move |v| settings.update(|s| s.org_name = v))
                    />
                    <TextArea
                        label="Description"
                        value=org_description
                        on_input=Callback::new(move |v| settings.update(|s| s.org_description = v))
                    />
                    <Input
                        label="Adresse"
                        value=org_address
                        on_input=Callback::new(move |v| settings.update(|s| s.org_address = v))
                    />
                    <Input
                        label="Unité / devise"
                        value=currency_unit
                        on_input=Callback::new(move |v| settings.update(|s| s.currency_unit = v))
                        placeholder="EUR"
                    />
                    <Input
                        label="Couleur du thème"
                        r#type="color"
                        value=theme_color
                        on_input=Callback::new(move |v: String| {
                            apply_theme_color(&v);
                            settings.update(|s| s.theme_color = v);
                        })
                    />
                    <label class="flex flex-col gap-1.5 text-sm font-medium">
                        <span>
                            "Taille du texte ("
                            {move || font_scale_label.get()}
                            ")"
                        </span>
                        <input
                            type="range"
                            min="0.9"
                            max="1.4"
                            step="0.05"
                            class="tap-target w-full"
                            prop:value=move || settings.get().font_scale
                            on:input=move |ev| {
                                if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                    settings.update(|s| s.font_scale = v);
                                    apply_font_scale(v);
                                }
                            }
                        />
                    </label>
                    <Input
                        label="Chemin du logo"
                        value=logo_path
                        on_input=Callback::new(move |v| settings.update(|s| s.logo_path = v))
                        placeholder="C:/chemin/logo.png"
                    />

                    <div class="mt-2 flex flex-wrap justify-end gap-2 border-t border-[var(--border)] pt-4">
                        <Button
                            variant=ButtonVariant::Secondary
                            on_click=Callback::new(move |_| {
                                spawn_local(async move {
                                    let default = "ndimbelente.bak".to_string();
                                    let path = match api::pick_save_file(&default).await {
                                        Some(p) => p,
                                        None => return,
                                    };
                                    match api::export_backup(&path).await {
                                        Ok(()) => {
                                            message.set(Some(format!("Sauvegarde exportée : {path}")))
                                        }
                                        Err(e) => error.set(Some(e)),
                                    }
                                });
                            })
                        >
                            "Exporter les données"
                        </Button>
                        <Button
                            variant=ButtonVariant::Secondary
                            on_click=Callback::new(move |_| {
                                spawn_local(async move {
                                    let path = match api::pick_backup_file().await {
                                        Some(p) => p,
                                        None => return,
                                    };
                                    match api::import_backup(&path).await {
                                        Ok(()) => {
                                            message.set(Some(
                                                "Import réussi — redémarrage de l’application…".into(),
                                            ));
                                            let _ = api::restart_app().await;
                                        }
                                        Err(e) => error.set(Some(e)),
                                    }
                                });
                            })
                        >
                            "Importer les données"
                        </Button>
                        <Button on_click=Callback::new(move |_| {
                            message.set(None);
                            error.set(None);
                            let s = settings.get_untracked();
                            spawn_local(async move {
                                match api::update_settings(&s).await {
                                    Ok(saved) => {
                                        settings.set(saved.clone());
                                        apply_font_scale(saved.font_scale);
                                        apply_theme_color(&saved.theme_color);
                                        message.set(Some("Paramètres enregistrés.".into()));
                                    }
                                    Err(e) => error.set(Some(e)),
                                }
                            });
                        })>
                            "Enregistrer"
                        </Button>
                    </div>
                </div>
            </Show>

            <Show when=move || tab.get() == "receipt">
                <div class="flex flex-col gap-4 rounded-2xl border border-[var(--border)] bg-[var(--surface)] p-5 shadow-[var(--shadow)]">
                    <p class="text-sm text-[var(--muted)]">
                        "Placeholders : {{USER.NAME}}, {{USER.CARD}}, {{DATE}}, {{PAYMENT_DATE}}, {{PAYMENT_METHOD}}, {{COTISATION.MONTH}} / {{COTISATION.YEAR}}, {{YEAR_TOTAL_DUE}}, {{RECEIVED_AMOUNT}}, {{REMAINING_DEBT}}, {{NEW_BALANCE}}, {{ORG.NAME}}, {{ORG.ADDRESS}}. Couleur : Format A4 uniquement — Mini(POS) en noir & blanc."
                    </p>
                    <div class="flex flex-wrap gap-2">
                        <button
                            type="button"
                            class=move || {
                                if receipt_mode.get() == "a4" {
                                    "tap-target rounded-xl bg-[var(--brand)] px-4 py-2 font-semibold text-white"
                                } else {
                                    "tap-target rounded-xl border border-[var(--border)] px-4 py-2 font-semibold"
                                }
                            }
                            on:click=move |_| receipt_mode.set("a4".into())
                        >
                            "Format A4"
                        </button>
                        <button
                            type="button"
                            class=move || {
                                if receipt_mode.get() == "mini" {
                                    "tap-target rounded-xl bg-[var(--brand)] px-4 py-2 font-semibold text-white"
                                } else {
                                    "tap-target rounded-xl border border-[var(--border)] px-4 py-2 font-semibold"
                                }
                            }
                            on:click=move |_| receipt_mode.set("mini".into())
                        >
                            "Format Mini(POS)"
                        </button>
                    </div>

                    <div class="flex flex-wrap gap-2">
                        <button
                            type="button"
                            class=move || {
                                if editor_mode.get() == "template" {
                                    "tap-target rounded-lg bg-[color-mix(in_srgb,var(--brand)_18%,transparent)] px-3 py-1.5 text-sm font-semibold"
                                } else {
                                    "tap-target rounded-lg border border-[var(--border)] px-3 py-1.5 text-sm"
                                }
                            }
                            on:click=move |_| set_editor_mode("template")
                        >
                            "Éditeur modèle"
                        </button>
                        <button
                            type="button"
                            class=move || {
                                if editor_mode.get() == "fields" {
                                    "tap-target rounded-lg bg-[color-mix(in_srgb,var(--brand)_18%,transparent)] px-3 py-1.5 text-sm font-semibold"
                                } else {
                                    "tap-target rounded-lg border border-[var(--border)] px-3 py-1.5 text-sm"
                                }
                            }
                            on:click=move |_| set_editor_mode("fields")
                        >
                            "Champs (formulaire)"
                        </button>
                    </div>

                    <Show when=move || receipt_mode.get() == "mini">
                        <Input
                            label="Largeur Mini(POS) (mm)"
                            r#type="number"
                            value=mini_width
                            on_input=Callback::new(move |v: String| {
                                if let Ok(n) = v.replace(',', ".").parse::<f64>() {
                                    settings.update(|s| s.receipt_mini_width_mm = n);
                                }
                            })
                        />
                    </Show>

                    <Show when=move || editor_mode.get() == "template">
                        <RichTextEditor
                            value=template_html
                            allow_color=Signal::derive(move || allow_color.get())
                            on_change=Callback::new(move |html: String| {
                                settings.update(|s| {
                                    if receipt_mode.get_untracked() == "mini" {
                                        s.receipt_template_mini = html;
                                    } else {
                                        s.receipt_template_a4 = html;
                                    }
                                });
                            })
                        />
                    </Show>

                    <Show when=move || editor_mode.get() == "fields">
                        <ReceiptFormBuilder
                            fields=fields_list
                            allow_color=Signal::derive(move || allow_color.get())
                            on_change=Callback::new(move |list: Vec<ReceiptField>| {
                                settings.update(|s| {
                                    if receipt_mode.get_untracked() == "mini" {
                                        s.receipt_fields_mini = list;
                                        s.receipt_template_mini = fields_to_html(&s.receipt_fields_mini);
                                    } else {
                                        s.receipt_fields_a4 = list;
                                        s.receipt_template_a4 = fields_to_html(&s.receipt_fields_a4);
                                    }
                                });
                            })
                        />
                    </Show>

                    <div>
                        <p class="mb-2 text-sm font-semibold">"Aperçu"</p>
                        <pre
                            class="whitespace-pre-wrap rounded-xl border border-[var(--border)] bg-white p-4 text-sm text-black"
                            style=move || {
                                if receipt_mode.get() == "mini" {
                                    format!(
                                        "width:{}mm;max-width:100%;font-size:11px",
                                        settings.get().receipt_mini_width_mm
                                    )
                                } else {
                                    "max-width:210mm".into()
                                }
                            }
                        >
                            {move || preview.get()}
                        </pre>
                    </div>

                    <div class="flex flex-wrap justify-end gap-2">
                        <Button
                            variant=ButtonVariant::Secondary
                            on_click=Callback::new(move |_| {
                                settings.update(|s| {
                                    let d = AppSettings::default();
                                    if receipt_mode.get_untracked() == "mini" {
                                        s.receipt_template_mini = d.receipt_template_mini;
                                        s.receipt_fields_mini = d.receipt_fields_mini;
                                    } else {
                                        s.receipt_template_a4 = d.receipt_template_a4;
                                        s.receipt_fields_a4 = d.receipt_fields_a4;
                                    }
                                });
                                message.set(Some("Modèle réinitialisé (non enregistré).".into()));
                            })
                        >
                            "Réinitialiser le modèle"
                        </Button>
                        <Button on_click=Callback::new(move |_| {
                            let s = settings.get_untracked();
                            spawn_local(async move {
                                match api::update_settings(&s).await {
                                    Ok(saved) => {
                                        settings.set(saved);
                                        message.set(Some("Modèle de reçu enregistré.".into()));
                                    }
                                    Err(e) => error.set(Some(e)),
                                }
                            });
                        })>
                            "Enregistrer le modèle"
                        </Button>
                    </div>
                </div>
            </Show>
            </div>
        </div>
    }
}
