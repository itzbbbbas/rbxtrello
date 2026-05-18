use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::sync::VALID_LABEL_COLORS;

/// Top-level deserialization of `rbxtrello.toml`.
///
/// We intentionally use BTreeMap (stable iteration order) so diff output and
/// regenerated toml are deterministic.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VCSBoard {
    pub metadata: Metadata,

    #[serde(default)]
    pub labels: BTreeMap<String, LabelDef>,

    #[serde(default)]
    pub lists: BTreeMap<String, ListDef>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    pub board_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub board_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LabelDef {
    /// Trello color: yellow|purple|blue|red|green|orange|black|sky|pink|lime
    pub color: String,
    /// Display name on Trello; defaults to slug if omitted
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Trello ID — written back by sync/pull. Omit for new labels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListDef {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<u32>,
    /// If false, sync never archives orphan cards on this list.
    #[serde(default = "default_managed")]
    pub managed: bool,
    /// Trello ID — written back by sync/pull.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub cards: BTreeMap<String, CardDef>,
}

fn default_managed() -> bool {
    true
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardDef {
    pub name: String,
    #[serde(default)]
    pub desc: String,
    /// Slugs referencing [labels.<slug>].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
    /// Trello ID — written back by sync/pull. Omit for new cards.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    // Reserved for v0.2 — declared but not yet pushed:
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover: Option<CoverDef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub checklists: Vec<ChecklistDef>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, toml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverDef {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChecklistDef {
    pub name: String,
    #[serde(default)]
    pub items: Vec<String>,
}

impl VCSBoard {
    /// Parse the toml file at [`crate::sync::TOML_PATH`].
    pub async fn load() -> anyhow::Result<Self> {
        let path = crate::sync::TOML_PATH;
        let text = crate::utils::read_to_string(path).await?;
        let board: VCSBoard =
            toml::from_str(&text).map_err(|e| anyhow::anyhow!("failed to parse {path}: {e}"))?;
        board.validate()?;
        Ok(board)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.metadata.board_name.trim().is_empty() {
            anyhow::bail!("[metadata].board_name is required and must be non-empty");
        }

        for (slug, label) in &self.labels {
            if !VALID_LABEL_COLORS.contains(&label.color.as_str()) {
                anyhow::bail!(
                    "[labels.{slug}].color = \"{}\" is invalid. Allowed: {}",
                    label.color,
                    VALID_LABEL_COLORS.join(", ")
                );
            }
        }

        for (list_slug, list) in &self.lists {
            if list.name.trim().is_empty() {
                anyhow::bail!("[lists.{list_slug}].name is required");
            }
            for (card_slug, card) in &list.cards {
                if card.name.trim().is_empty() {
                    anyhow::bail!("[lists.{list_slug}.cards.{card_slug}].name is required");
                }
                for label_ref in &card.labels {
                    if !self.labels.contains_key(label_ref) {
                        anyhow::bail!(
                            "[lists.{list_slug}.cards.{card_slug}] references unknown label \"{label_ref}\". Declare it under [labels.{label_ref}]."
                        );
                    }
                }
            }
        }

        Ok(())
    }
}
