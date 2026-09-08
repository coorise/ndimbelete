//! Receipt template rendering (placeholders + styled field / HTML lines).

use chrono::Local;

use crate::models::ReceiptField;
use super::receipt_pdf::ReceiptPrintPayload;

#[derive(Debug, Clone)]
pub struct RenderedLine {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub font_size: f64,
    pub align: String,
    /// Hex `#RRGGBB` — ignored for Mini thermal.
    pub color: String,
}

impl RenderedLine {
    fn plain(text: impl Into<String>, font_size: f64) -> Self {
        Self {
            text: text.into(),
            bold: false,
            italic: false,
            font_size,
            align: "left".into(),
            color: "#000000".into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReceiptContext {
    pub org_name: String,
    pub org_address: String,
    pub member_name: String,
    pub card_number: String,
    pub member_uid: String,
    pub day: String,
    pub month: String,
    pub year: String,
    pub period_label: String,
    pub received_amount: String,
    pub previous_debt: String,
    pub new_balance: String,
    pub printed_at: String,
}

fn money(v: f64) -> String {
    format!("{v:.2} €")
}

impl ReceiptContext {
    pub fn from_payload(p: &ReceiptPrintPayload, org_address: &str) -> Self {
        let printed_at = Local::now().format("%d/%m/%Y %H:%M").to_string();
        let (day, month) = split_period(&p.period_label, p.year);
        Self {
            org_name: p.org_name.clone(),
            org_address: if p.org_address.is_empty() {
                org_address.to_string()
            } else {
                p.org_address.clone()
            },
            member_name: p.member_name.clone(),
            card_number: p.card_number.clone(),
            member_uid: p.member_uid.clone(),
            day,
            month,
            year: p.year.to_string(),
            period_label: p.period_label.clone(),
            received_amount: money(p.amount),
            previous_debt: money(p.debt_before),
            // Nouveau Solde = cumulative amount paid this year (not remaining debt).
            new_balance: money(p.total_paid_year),
            printed_at,
        }
    }

    pub fn substitute(&self, tpl: &str) -> String {
        tpl.replace("{{ORG.NAME}}", &self.org_name)
            .replace("{{ORG.ADDRESS}}", &self.org_address)
            .replace("{{USER.NAME}}", &self.member_name)
            .replace("{{USER.CARD}}", &self.card_number)
            .replace("{{USER.UID}}", &self.member_uid)
            .replace("{{COTISATION.DAY}}", &self.day)
            .replace("{{COTISATION.MONTH}}", &self.month)
            .replace("{{COTISATION.YEAR}}", &self.year)
            .replace("{{COTISATION.PERIOD}}", &self.period_label)
            .replace("{{RECEIVED_AMOUNT}}", &self.received_amount)
            .replace("{{REMAINING_DEBT}}", &self.previous_debt)
            .replace("{{PREVIOUS_DEBT}}", &self.previous_debt)
            .replace("{{NEW_BALANCE}}", &self.new_balance)
            .replace("{{DATE}}", &self.printed_at)
    }
}

fn split_period(label: &str, _year: i32) -> (String, String) {
    let months_fr = [
        ("janvier", "01"),
        ("février", "02"),
        ("fevrier", "02"),
        ("mars", "03"),
        ("avril", "04"),
        ("mai", "05"),
        ("juin", "06"),
        ("juillet", "07"),
        ("août", "08"),
        ("aout", "08"),
        ("septembre", "09"),
        ("octobre", "10"),
        ("novembre", "11"),
        ("décembre", "12"),
        ("decembre", "12"),
    ];
    let lower = label.to_lowercase();
    for (name, num) in months_fr {
        if lower.contains(name) {
            return ("01".into(), num.into());
        }
    }
    if let Some((a, b)) = label.split_once('/') {
        let a = a.trim();
        let b = b.trim();
        if a.len() <= 2 && b.len() == 4 {
            let m = a.parse::<i32>().unwrap_or(1);
            return ("01".into(), format!("{m:02}"));
        }
        if a.len() <= 2 && b.len() <= 2 {
            return (a.into(), b.into());
        }
    }
    ("01".into(), "01".into())
}

fn html_unescape(s: &str) -> String {
    s.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
}

/// Very small HTML → lines parser (div/p/br + b/i/span styles).
pub fn html_to_lines(html: &str, default_size: f64, allow_color: bool) -> Vec<RenderedLine> {
    let mut lines: Vec<RenderedLine> = Vec::new();
    let mut cur = RenderedLine::plain(String::new(), default_size);
    let mut buf = String::new();
    let chars: Vec<char> = html.chars().collect();
    let mut i = 0;

    let flush_buf = |buf: &mut String, cur: &mut RenderedLine| {
        if !buf.is_empty() {
            cur.text.push_str(&html_unescape(buf));
            buf.clear();
        }
    };

    let push_line = |lines: &mut Vec<RenderedLine>, cur: &mut RenderedLine, default_size: f64| {
        let text = cur.text.trim_end().to_string();
        lines.push(RenderedLine {
            text,
            bold: cur.bold,
            italic: cur.italic,
            font_size: cur.font_size,
            align: cur.align.clone(),
            color: if allow_color {
                cur.color.clone()
            } else {
                "#000000".into()
            },
        });
        *cur = RenderedLine::plain(String::new(), default_size);
    };

    while i < chars.len() {
        if chars[i] == '<' {
            flush_buf(&mut buf, &mut cur);
            let end = chars[i..].iter().position(|c| *c == '>').map(|p| i + p);
            let Some(end) = end else { break };
            let tag: String = chars[i + 1..end].iter().collect::<String>().to_lowercase();
            let tag_name = tag.split_whitespace().next().unwrap_or("");
            match tag_name {
                "br" | "br/" => push_line(&mut lines, &mut cur, default_size),
                "/p" | "/div" | "/li" => {
                    if !cur.text.is_empty() || lines.is_empty() {
                        push_line(&mut lines, &mut cur, default_size);
                    } else if cur.text.is_empty() {
                        // keep empty line from <div><br></div>
                    }
                }
                "b" | "strong" => cur.bold = true,
                "/b" | "/strong" => cur.bold = false,
                "i" | "em" => cur.italic = true,
                "/i" | "/em" => cur.italic = false,
                "p" | "div" => {
                    if let Some(align) = extract_style_align(&tag) {
                        cur.align = align;
                    }
                    if let Some(size) = extract_style_font_size(&tag) {
                        cur.font_size = size;
                    }
                    if allow_color {
                        if let Some(color) = extract_style_color(&tag) {
                            cur.color = color;
                        }
                    }
                }
                "span" => {
                    if let Some(size) = extract_style_font_size(&tag) {
                        cur.font_size = size;
                    }
                    if allow_color {
                        if let Some(color) = extract_style_color(&tag) {
                            cur.color = color;
                        }
                    }
                }
                _ => {}
            }
            i = end + 1;
            continue;
        }
        buf.push(chars[i]);
        i += 1;
    }
    flush_buf(&mut buf, &mut cur);
    if !cur.text.is_empty() || lines.is_empty() {
        push_line(&mut lines, &mut cur, default_size);
    }
    // Drop trailing empties beyond one
    while lines.len() > 1 && lines.last().map(|l| l.text.is_empty()).unwrap_or(false) {
        lines.pop();
    }
    lines
}

fn extract_style_color(tag: &str) -> Option<String> {
    let lower = tag.to_lowercase();
    let key = "color:";
    let idx = lower.find(key)?;
    let rest = &tag[idx + key.len()..];
    let val = rest
        .split([';', '"', '\'', ' '])
        .next()?
        .trim()
        .to_string();
    if val.starts_with('#') {
        Some(val)
    } else {
        None
    }
}

fn extract_style_font_size(tag: &str) -> Option<f64> {
    let lower = tag.to_lowercase();
    let key = "font-size:";
    let idx = lower.find(key)?;
    let rest = &tag[idx + key.len()..];
    let raw = rest.split([';', '"', '\'', ' ']).next()?.trim();
    let num: String = raw.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    num.parse().ok()
}

fn extract_style_align(tag: &str) -> Option<String> {
    let lower = tag.to_lowercase();
    if lower.contains("text-align:center") || lower.contains("align=\"center\"") {
        Some("center".into())
    } else if lower.contains("text-align:right") || lower.contains("align=\"right\"") {
        Some("right".into())
    } else if lower.contains("text-align:left") {
        Some("left".into())
    } else {
        None
    }
}

fn parse_markup_line(raw: &str, default_size: f64) -> RenderedLine {
    let t = raw.trim_end();
    let trimmed = t.trim();
    if trimmed.is_empty() {
        return RenderedLine::plain(String::new(), default_size);
    }
    if let Some(inner) = trimmed
        .strip_prefix("**")
        .and_then(|s| s.strip_suffix("**"))
    {
        return RenderedLine {
            text: inner.to_string(),
            bold: true,
            italic: false,
            font_size: default_size,
            align: "left".into(),
            color: "#000000".into(),
        };
    }
    if let Some(inner) = trimmed.strip_prefix('*').and_then(|s| s.strip_suffix('*')) {
        if !inner.contains('*') {
            return RenderedLine {
                text: inner.to_string(),
                bold: false,
                italic: true,
                font_size: default_size,
                align: "left".into(),
                color: "#000000".into(),
            };
        }
    }
    RenderedLine::plain(t.to_string(), default_size)
}

pub fn render_template(
    tpl: &str,
    ctx: &ReceiptContext,
    default_size: f64,
    allow_color: bool,
) -> Vec<RenderedLine> {
    let filled = ctx.substitute(tpl);
    if filled.contains('<') && (filled.contains("<div") || filled.contains("<p") || filled.contains("<br"))
    {
        html_to_lines(&filled, default_size, allow_color)
    } else {
        filled
            .lines()
            .map(|line| parse_markup_line(line, default_size))
            .collect()
    }
}

pub fn render_fields(
    fields: &[ReceiptField],
    ctx: &ReceiptContext,
    allow_color: bool,
) -> Vec<RenderedLine> {
    let mut sorted = fields.to_vec();
    sorted.sort_by_key(|f| f.position);
    sorted
        .into_iter()
        .map(|f| {
            let text = ctx.substitute(&f.content);
            RenderedLine {
                text,
                bold: f.bold,
                italic: f.italic,
                font_size: if f.font_size > 0.0 { f.font_size } else { 10.0 },
                align: if f.align.is_empty() {
                    "left".into()
                } else {
                    f.align
                },
                color: if allow_color && !f.color.is_empty() {
                    f.color
                } else {
                    "#000000".into()
                },
            }
        })
        .collect()
}

pub fn fields_to_template(fields: &[ReceiptField]) -> String {
    let mut sorted = fields.to_vec();
    sorted.sort_by_key(|f| f.position);
    sorted
        .into_iter()
        .map(|f| {
            let style = format!(
                "font-size:{}pt;text-align:{};color:{}",
                f.font_size,
                if f.align.is_empty() { "left" } else { &f.align },
                if f.color.is_empty() { "#000000" } else { &f.color }
            );
            let mut inner = html_escape(&f.content);
            if f.bold {
                inner = format!("<b>{inner}</b>");
            }
            if f.italic {
                inner = format!("<i>{inner}</i>");
            }
            if inner.is_empty() {
                format!("<div style=\"{style}\"><br></div>")
            } else {
                format!("<div style=\"{style}\">{inner}</div>")
            }
        })
        .collect::<Vec<_>>()
        .join("")
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub fn resolve_lines(
    mode: &str,
    template: &str,
    fields: &[ReceiptField],
    ctx: &ReceiptContext,
    default_size: f64,
    allow_color: bool,
) -> Vec<RenderedLine> {
    if mode.eq_ignore_ascii_case("fields") {
        render_fields(fields, ctx, allow_color)
    } else {
        render_template(template, ctx, default_size, allow_color)
    }
}

pub fn to_thermal_ascii(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ä' | 'ã' => 'a',
            'À' | 'Á' | 'Â' | 'Ä' | 'Ã' => 'A',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'È' | 'É' | 'Ê' | 'Ë' => 'E',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'Ì' | 'Í' | 'Î' | 'Ï' => 'I',
            'ò' | 'ó' | 'ô' | 'ö' | 'õ' => 'o',
            'Ò' | 'Ó' | 'Ô' | 'Ö' | 'Õ' => 'O',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'Ù' | 'Ú' | 'Û' | 'Ü' => 'U',
            'ç' => 'c',
            'Ç' => 'C',
            'ñ' => 'n',
            'Ñ' => 'N',
            '€' => 'E',
            '—' | '–' => '-',
            '’' | '‘' | '`' => '\'',
            '«' | '»' => '"',
            '°' => 'o',
            c if c.is_ascii() => c,
            _ => '?',
        })
        .collect()
}
