//! Distribution keys (Verteilerschlüssel) of cost categories.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// How the total of a cost position is split onto a unit.
///
/// Serialised as the API code (`mea`, `per_notice`, `per_attachment`, `per_unit`);
/// [`DistributionKey::label`] gives the German text shown in documents and the
/// tenant app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DistributionKey {
    /// By co-ownership shares (Miteigentumsanteile): unit shares / property shares.
    Mea,
    /// Amount as stated in the official notice (laut Bescheid), entered per unit.
    PerNotice,
    /// Amount as shown in an attachment (siehe Anlage), entered per unit.
    PerAttachment,
    /// Equal split over all units of the property (#2, decision 26).
    PerUnit,
}

/// Error for an unknown distribution key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("Unbekannter Verteilerschlüssel")]
pub struct UnknownKey;

impl DistributionKey {
    /// All keys in display order.
    pub const ALL: [DistributionKey; 4] = [
        DistributionKey::Mea,
        DistributionKey::PerNotice,
        DistributionKey::PerAttachment,
        DistributionKey::PerUnit,
    ];

    /// Stable code used in the API and the database.
    pub const fn code(self) -> &'static str {
        match self {
            DistributionKey::Mea => "mea",
            DistributionKey::PerNotice => "per_notice",
            DistributionKey::PerAttachment => "per_attachment",
            DistributionKey::PerUnit => "per_unit",
        }
    }

    /// German display text (the stored value of the old application, plus
    /// `nach Einheiten` for the new key).
    pub const fn label(self) -> &'static str {
        match self {
            DistributionKey::Mea => "MEA",
            DistributionKey::PerNotice => "laut Bescheid",
            DistributionKey::PerAttachment => "siehe Anlage",
            DistributionKey::PerUnit => "nach Einheiten",
        }
    }

    /// Whether the key takes amounts entered per unit (`per_notice`, `per_attachment`)
    /// instead of computing the share from the total (`mea`, `per_unit`; decision 5).
    pub const fn uses_unit_amounts(self) -> bool {
        matches!(
            self,
            DistributionKey::PerNotice | DistributionKey::PerAttachment
        )
    }

    /// Reads a key from its API code.
    pub fn from_code(code: &str) -> Result<Self, UnknownKey> {
        Self::ALL
            .into_iter()
            .find(|k| k.code() == code)
            .ok_or(UnknownKey)
    }

    /// Reads a key from the display text stored by the old application (legacy import,
    /// backup format 1). The old data is free text, so surrounding whitespace and
    /// letter case are ignored (the tenant app also compares lower-cased); any other
    /// text is an error.
    pub fn from_label(label: &str) -> Result<Self, UnknownKey> {
        let label = label.trim().to_lowercase();
        Self::ALL
            .into_iter()
            .find(|k| k.label().to_lowercase() == label)
            .ok_or(UnknownKey)
    }
}

impl fmt::Display for DistributionKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl FromStr for DistributionKey {
    type Err = UnknownKey;
    fn from_str(code: &str) -> Result<Self, UnknownKey> {
        Self::from_code(code)
    }
}

/// The key in effect for a cost position: the per-period override wins over the
/// category default (`effectiveDistributionKey` in the old code).
pub fn effective_key(
    override_key: Option<DistributionKey>,
    category_key: DistributionKey,
) -> DistributionKey {
    override_key.unwrap_or(category_key)
}
