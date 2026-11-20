//! Shape of the PDF template configuration (`src/types/pdf-template.ts`) and its default
//! (`getDefaultConfig`, `src/lib/pdf-template.ts`). The typed form is used to *validate*
//! a request (unknown fonts, missing sections, silly sizes -> 400 instead of a broken
//! PDF); the stored text is the client's JSON, so fields this version does not know
//! survive a round trip.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::validate::Validator;

/// Longest text of one section field (characters).
const TEXT_MAX: usize = 2000;
/// Most free text entries per section.
const TEXTS_MAX: usize = 10;

/// Fonts of the built-in PDF fonts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum Font {
    Helvetica,
    #[serde(rename = "Times-Roman")]
    TimesRoman,
    Courier,
}

/// Text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Align {
    Left,
    Center,
    Right,
}

/// The sections of the statement, in the key spelling of the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum SectionId {
    Header,
    SenderLine,
    Recipient,
    Meta,
    Title,
    ObjectLine,
    CostTable,
    Prepayment,
    Result,
    DistributionKeys,
    BankInfo,
    Closing,
}

/// Page settings (points).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Page {
    font_family: Font,
    font_size: f64,
    padding_top: f64,
    padding_bottom: f64,
    padding_left: f64,
    padding_right: f64,
}

/// Style of one section.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Style {
    font_family: Font,
    font_size: f64,
    bold: bool,
    italic: bool,
    text_align: Align,
    color: String,
    margin_top: f64,
    margin_bottom: f64,
}

/// One section: visibility, style and free texts.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Section {
    visible: bool,
    style: Style,
    texts: BTreeMap<String, String>,
}

/// The whole configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateConfig {
    page: Page,
    sections: BTreeMap<SectionId, Section>,
}

impl Section {
    /// A visible, left-aligned black Helvetica section with the given size and margins
    /// `(top, bottom)`.
    fn new(font_size: f64, margins: (f64, f64)) -> Self {
        Self {
            visible: true,
            style: Style {
                font_family: Font::Helvetica,
                font_size,
                bold: false,
                italic: false,
                text_align: Align::Left,
                color: "#000000".to_owned(),
                margin_top: margins.0,
                margin_bottom: margins.1,
            },
            texts: BTreeMap::new(),
        }
    }

    /// Bold.
    fn bold(mut self) -> Self {
        self.style.bold = true;
        self
    }

    /// Centered.
    fn centered(mut self) -> Self {
        self.style.text_align = Align::Center;
        self
    }

    /// Text color.
    fn color(mut self, color: &str) -> Self {
        self.style.color = color.to_owned();
        self
    }

    /// Free texts.
    fn texts(mut self, texts: &[(&str, &str)]) -> Self {
        self.texts = texts
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        self
    }
}

impl TemplateConfig {
    /// Parses and checks a request value; the error is the message for the field
    /// `config`. All 12 sections must be present.
    pub fn check(value: &Value) -> Result<(), String> {
        let config = Self::deserialize(value)
            .map_err(|_| "Die Vorlage hat ein ungültiges Format".to_owned())?;
        if config.sections.len() != 12 {
            return Err("Die Vorlage muss alle Abschnitte enthalten".to_owned());
        }
        let mut v = Validator::new();
        let page = &config.page;
        v.int_in("c", "Schriftgröße", size_of(page.font_size), (1, 72));
        for pad in [
            page.padding_top,
            page.padding_bottom,
            page.padding_left,
            page.padding_right,
        ] {
            v.int_in("c", "Seitenrand", size_of(pad), (0, 200));
        }
        for section in config.sections.values() {
            let style = &section.style;
            v.int_in("c", "Schriftgröße", size_of(style.font_size), (1, 72));
            v.int_in("c", "Abstand", size_of(style.margin_top), (0, 200));
            v.int_in("c", "Abstand", size_of(style.margin_bottom), (0, 200));
            if !is_color(&style.color) {
                v.fail("c", "Farbe ist ungültig");
            }
            let too_long = section.texts.values().any(|t| t.chars().count() > TEXT_MAX);
            if section.texts.len() > TEXTS_MAX || too_long {
                v.fail("c", "Ein Text der Vorlage ist zu lang");
            }
        }
        v.finish().map_err(|e| e.message().to_owned())
    }

    /// The default template (as `getDefaultConfig` in the old web code).
    pub fn default_value() -> Value {
        serde_json::to_value(Self::default_config()).unwrap_or(Value::Null)
    }

    /// The default as typed value.
    fn default_config() -> Self {
        use SectionId::*;
        let page = Page {
            font_family: Font::Helvetica,
            font_size: 10.0,
            padding_top: 40.0,
            padding_bottom: 40.0,
            padding_left: 50.0,
            padding_right: 50.0,
        };
        let title = [("title", "Neben-/Betriebskostenabrechnung {{year}}")];
        let payment = [(
            "paymentNote",
            "Nachzahlungen sind, sofern nicht anders vereinbart, einen Monat nach Zugang der Abrechnung fällig.",
        )];
        let closing = [
            (
                "text",
                "Falls Sie Fragen zu der Abrechnung haben oder Belege einsehen möchten, stehe ich Ihnen gerne zur Verfügung.",
            ),
            ("greeting", "Viele Grüße"),
        ];
        let sections = BTreeMap::from([
            (Header, Section::new(12.0, (0.0, 20.0)).bold()),
            (SenderLine, Section::new(7.0, (0.0, 4.0)).color("#555555")),
            (Recipient, Section::new(10.0, (0.0, 10.0))),
            (Meta, Section::new(8.0, (0.0, 20.0))),
            (
                Title,
                Section::new(14.0, (0.0, 12.0))
                    .bold()
                    .centered()
                    .texts(&title),
            ),
            (ObjectLine, Section::new(10.0, (0.0, 12.0)).bold()),
            (CostTable, Section::new(10.0, (0.0, 12.0))),
            (Prepayment, Section::new(10.0, (8.0, 4.0))),
            (Result, Section::new(10.0, (4.0, 0.0)).bold()),
            (DistributionKeys, Section::new(9.0, (20.0, 16.0))),
            (BankInfo, Section::new(10.0, (16.0, 16.0)).texts(&payment)),
            (Closing, Section::new(10.0, (20.0, 0.0)).texts(&closing)),
        ]);
        Self { page, sections }
    }
}

/// A value for the whole-number range check: NaN and infinity map outside every range.
fn size_of(value: f64) -> i64 {
    if value.is_finite() {
        // Rounded up so 0.5 does not pass a lower bound of 1; the range is tiny.
        value.ceil().clamp(-1e9, 1e9) as i64
    } else {
        i64::MIN
    }
}

/// `#rgb` or `#rrggbb`.
fn is_color(value: &str) -> bool {
    value
        .strip_prefix('#')
        .is_some_and(|hex| matches!(hex.len(), 3 | 6) && hex.bytes().all(|b| b.is_ascii_hexdigit()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn default_is_valid_and_complete() {
        let value = TemplateConfig::default_value();
        assert_eq!(TemplateConfig::check(&value), Ok(()));
        assert_eq!(value["sections"].as_object().map(|s| s.len()), Some(12));
        assert_eq!(
            value["sections"]["title"]["texts"]["title"],
            "Neben-/Betriebskostenabrechnung {{year}}"
        );
        assert_eq!(value["sections"]["header"]["style"]["textAlign"], "left");
        assert_eq!(value["page"]["fontFamily"], "Helvetica");
    }

    #[test]
    fn rejects_broken_templates() {
        let good = TemplateConfig::default_value();
        let mut missing = good.clone();
        missing["sections"]
            .as_object_mut()
            .map(|s| s.remove("closing"));
        let mut font = good.clone();
        font["page"]["fontFamily"] = json!("Comic Sans");
        let mut size = good.clone();
        size["sections"]["title"]["style"]["fontSize"] = json!(500);
        let mut color = good.clone();
        color["sections"]["title"]["style"]["color"] = json!("rot");
        for bad in [
            missing,
            font,
            size,
            color,
            json!({}),
            json!("x"),
            Value::Null,
        ] {
            assert!(TemplateConfig::check(&bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn unknown_fields_are_allowed() {
        let mut value = TemplateConfig::default_value();
        value["extra"] = json!(true);
        assert_eq!(TemplateConfig::check(&value), Ok(()));
    }
}
