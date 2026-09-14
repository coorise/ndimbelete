//! App settings (organisation, theme, receipt templates).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptField {
    pub id: String,
    /// Template fragment (may contain placeholders).
    pub content: String,
    /// Display order (ascending).
    pub position: i32,
    pub font_size: f64,
    pub bold: bool,
    pub italic: bool,
    /// `left` | `center` | `right`
    #[serde(default = "default_align")]
    pub align: String,
    /// Hex color (`#000000`). Ignored for Mini(POS) print.
    #[serde(default = "default_color")]
    pub color: String,
}

fn default_align() -> String {
    "left".into()
}

fn default_color() -> String {
    "#000000".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub org_name: String,
    pub org_description: String,
    pub org_address: String,
    pub theme_color: String,
    pub font_scale: f64,
    pub logo_path: String,
    #[serde(default = "default_currency_unit")]
    pub currency_unit: String,
    pub receipt_template_a4: String,
    pub receipt_template_mini: String,
    pub receipt_mini_width_mm: f64,
    #[serde(default = "default_editor_mode")]
    pub receipt_editor_mode_a4: String,
    #[serde(default = "default_editor_mode")]
    pub receipt_editor_mode_mini: String,
    #[serde(default = "default_a4_fields")]
    pub receipt_fields_a4: Vec<ReceiptField>,
    #[serde(default = "default_mini_fields")]
    pub receipt_fields_mini: Vec<ReceiptField>,
    /// `intuitive` = debt shown negative / surplus positive; `excel` = Excel-style (surplus negative).
    #[serde(default = "default_debt_display_sign")]
    pub debt_display_sign: String,
}

fn default_debt_display_sign() -> String {
    "intuitive".into()
}

fn default_currency_unit() -> String {
    "EUR".into()
}

fn default_editor_mode() -> String {
    "template".into()
}

fn parse_markup_line(raw: &str) -> (String, bool, bool) {
    let t = raw.trim();
    if let Some(inner) = t.strip_prefix("**").and_then(|s| s.strip_suffix("**")) {
        return (inner.to_string(), true, false);
    }
    if let Some(inner) = t.strip_prefix('*').and_then(|s| s.strip_suffix('*')) {
        if !inner.contains('*') {
            return (inner.to_string(), false, true);
        }
    }
    (t.to_string(), false, false)
}

pub fn template_to_fields(tpl: &str) -> Vec<ReceiptField> {
    // Strip simple HTML to plain lines when converting.
    let plain = tpl
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("</p>", "\n")
        .replace("</div>", "\n");
    let stripped = strip_tags(&plain);
    stripped
        .lines()
        .enumerate()
        .map(|(i, line)| {
            let (content, bold, italic) = parse_markup_line(line);
            ReceiptField {
                id: Uuid::new_v4().to_string(),
                content,
                position: i as i32,
                font_size: if i == 0 { 12.0 } else { 10.0 },
                bold: bold || i == 0,
                italic,
                align: default_align(),
                color: default_color(),
            }
        })
        .collect()
}

fn strip_tags(s: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    html_unescape(&out)
}

fn html_unescape(s: &str) -> String {
    s.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
}

pub fn default_a4_fields() -> Vec<ReceiptField> {
    template_to_fields(&default_a4_template_plain())
}

pub fn default_mini_fields() -> Vec<ReceiptField> {
    template_to_fields(&default_mini_template_plain())
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            org_name: "NDIMBELENTÉ:  Association Sénégalaise d'Entraide.".into(),
            org_description: "Association d'entraide à but non lucratif — gestion des cotisations".into(),
            org_address: "59 rue Fontaine au roi, 75011 PARIS".into(),
            theme_color: "#0B7A3E".into(),
            font_scale: 1.0,
            logo_path: String::new(),
            currency_unit: default_currency_unit(),
            receipt_template_a4: default_a4_template(),
            receipt_template_mini: default_mini_template(),
            receipt_mini_width_mm: 58.0,
            receipt_editor_mode_a4: default_editor_mode(),
            receipt_editor_mode_mini: default_editor_mode(),
            receipt_fields_a4: default_a4_fields(),
            receipt_fields_mini: default_mini_fields(),
            debt_display_sign: default_debt_display_sign(),
        }
    }
}

/// Long readable default (plain / markdown markers).
pub fn default_mini_template_plain() -> String {
    "{{ORG.NAME}}\nReçu de cotisation — {{COTISATION.YEAR}}\n{{DATE}}\n------------------------------\nMembre : {{USER.NAME}}\nN° carte : {{USER.CARD}}\nDate de paiement : {{PAYMENT_DATE}}\nMontant total pour {{COTISATION.YEAR}} : {{YEAR_TOTAL_DUE}}\nMode de paiement : {{PAYMENT_METHOD}}\n**Montant Reçu : {{RECEIVED_AMOUNT}}**\nRestant à payer pour {{COTISATION.YEAR}} : {{REMAINING_DEBT}}\n{{SURPLUS_LINE}}\nNouveau Solde : {{NEW_BALANCE}}\n\nMerci pour votre solidarité\nNDIMBELENTÉ"
        .into()
}

pub fn default_a4_template_plain() -> String {
    "{{ORG.NAME}}\n{{ORG.ADDRESS}}\n\nReçu de cotisation — {{COTISATION.YEAR}}\n{{DATE}}\n------------------------------\nMembre : {{USER.NAME}}\nN° carte : {{USER.CARD}}\nDate de paiement : {{PAYMENT_DATE}}\n\nMontant total pour {{COTISATION.YEAR}} : {{YEAR_TOTAL_DUE}}\nMode de paiement : {{PAYMENT_METHOD}}\n**Montant Reçu : {{RECEIVED_AMOUNT}}**\nRestant à payer pour {{COTISATION.YEAR}} : {{REMAINING_DEBT}}\n{{SURPLUS_LINE}}\nNouveau Solde : {{NEW_BALANCE}}\n\nMerci pour votre solidarité\nNDIMBELENTÉ"
        .into()
}

/// HTML defaults for the rich-text editor.
pub fn default_a4_template() -> String {
    plain_to_html_template(&default_a4_template_plain())
}

pub fn default_mini_template() -> String {
    plain_to_html_template(&default_mini_template_plain())
}

fn plain_to_html_template(plain: &str) -> String {
    plain
        .lines()
        .map(|line| {
            let t = line.trim_end();
            if t.is_empty() {
                return "<div><br></div>".to_string();
            }
            if let Some(inner) = t.strip_prefix("**").and_then(|s| s.strip_suffix("**")) {
                return format!("<div><b>{inner}</b></div>");
            }
            format!("<div>{t}</div>")
        })
        .collect::<Vec<_>>()
        .join("")
}

/// Detect legacy receipt templates and upgrade to the year-obligation layout.
pub fn maybe_upgrade_short_template(current: &str, long_html: &str) -> String {
    let compact: String = current
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_lowercase();
    let looks_short = compact.contains("payé:{{received_amount}}")
        || compact.contains("paye:{{received_amount}}")
        || (compact.contains("{{user.name}}({{user.card}})")
            && compact.contains("merci")
            && !compact.contains("membre:"));
    let looks_legacy_mid = compact.contains("période:")
        || compact.contains("periode:")
        || compact.contains("detteantérieure")
        || compact.contains("detteanterieure")
        || (compact.contains("{{remaining_debt}}") && !compact.contains("{{year_total_due}}"));
    let missing_new_fields = current.trim().is_empty()
        || !compact.contains("{{year_total_due}}")
        || !compact.contains("{{payment_date}}")
        || !compact.contains("{{surplus_line}}");
    let upgraded = if looks_short || looks_legacy_mid || missing_new_fields {
        long_html.to_string()
    } else {
        current.to_string()
    };
    ensure_payment_method_in_template(&upgraded)
}

/// Insert `Mode de paiement : {{PAYMENT_METHOD}}` before the received-amount line when missing.
pub fn ensure_payment_method_in_template(tpl: &str) -> String {
    let compact: String = tpl
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_lowercase();
    if compact.contains("{{payment_method}}") {
        return tpl.to_string();
    }
    let insert_html = "<div>Mode de paiement : {{PAYMENT_METHOD}}</div>";
    let insert_plain = "Mode de paiement : {{PAYMENT_METHOD}}\n";
    if tpl.contains("<div") {
        if let Some(pos) = tpl.find("{{RECEIVED_AMOUNT}}") {
            if let Some(div_start) = tpl[..pos].rfind("<div") {
                return format!("{}{}{}", &tpl[..div_start], insert_html, &tpl[div_start..]);
            }
        }
        if let Some(pos) = tpl.find("Montant Reçu") {
            if let Some(div_start) = tpl[..pos].rfind("<div") {
                return format!("{}{}{}", &tpl[..div_start], insert_html, &tpl[div_start..]);
            }
        }
    }
    if let Some(pos) = tpl.find("**Montant Reçu") {
        return format!("{}{}{}", &tpl[..pos], insert_plain, &tpl[pos..]);
    }
    if let Some(pos) = tpl.find("Montant Reçu") {
        return format!("{}{}{}", &tpl[..pos], insert_plain, &tpl[pos..]);
    }
    if let Some(pos) = tpl.find("{{RECEIVED_AMOUNT}}") {
        return format!("{}{}{}", &tpl[..pos], insert_plain, &tpl[pos..]);
    }
    tpl.to_string()
}

/// True when saved field lines still use the old receipt vocabulary.
pub fn fields_need_receipt_upgrade(fields: &[ReceiptField]) -> bool {
    if fields.is_empty() {
        return true;
    }
    let joined = fields
        .iter()
        .map(|f| f.content.to_lowercase())
        .collect::<Vec<_>>()
        .join("\n");
    let compact: String = joined.chars().filter(|c| !c.is_whitespace()).collect();
    compact.contains("detteantérieure")
        || compact.contains("detteanterieure")
        || ((compact.contains("période") || compact.contains("periode"))
            && !compact.contains("datedepaiement"))
        || !compact.contains("{{year_total_due}}")
        || !compact.contains("{{payment_date}}")
        || !compact.contains("{{surplus_line}}")
}

/// Insert payment-method field before received amount when missing.
pub fn ensure_payment_method_in_fields(fields: &[ReceiptField]) -> Vec<ReceiptField> {
    let has = fields
        .iter()
        .any(|f| f.content.to_lowercase().contains("{{payment_method}}"));
    if has {
        return fields.to_vec();
    }
    let mut out = fields.to_vec();
    let insert_at = out
        .iter()
        .position(|f| {
            let c = f.content.to_lowercase();
            c.contains("{{received_amount}}") || c.contains("montant reçu") || c.contains("montant recu")
        })
        .unwrap_or(out.len());
    let font_size = out
        .get(insert_at)
        .map(|f| f.font_size)
        .unwrap_or(10.0);
    out.insert(
        insert_at,
        ReceiptField {
            id: Uuid::new_v4().to_string(),
            content: "Mode de paiement : {{PAYMENT_METHOD}}".into(),
            position: 0,
            font_size,
            bold: false,
            italic: false,
            align: default_align(),
            color: default_color(),
        },
    );
    for (i, f) in out.iter_mut().enumerate() {
        f.position = i as i32;
    }
    out
}
