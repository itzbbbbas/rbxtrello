use std::collections::{BTreeMap, HashMap, HashSet};

use crate::api::model::{Card as RCard, Label as RLabel, List as RList};
use crate::sync::board::{LabelDef, ListDef, VCSBoard};

/// One actionable change to apply to Trello.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffOp {
    CreateLabel {
        slug: String,
        name: String,
        color: String,
    },
    UpdateLabel {
        id: String,
        slug: String,
        old_name: String,
        new_name: String,
        old_color: String,
        new_color: String,
    },

    CreateList {
        slug: String,
        name: String,
        position: Option<u32>,
    },
    UpdateList {
        id: String,
        slug: String,
        old_name: String,
        new_name: String,
    },

    CreateCard {
        list_slug: String,
        card_slug: String,
        name: String,
        desc: String,
        label_slugs: Vec<String>,
    },
    UpdateCard {
        id: String,
        list_slug: String,
        card_slug: String,
        old_name: String,
        new_name: String,
        old_desc: String,
        new_desc: String,
        old_label_slugs: Vec<String>,
        new_label_slugs: Vec<String>,
    },
    ArchiveCard {
        id: String,
        list_slug: String,
        name: String,
    },
}

impl DiffOp {
    pub fn human(&self) -> String {
        match self {
            DiffOp::CreateLabel { slug, color, .. } => format!("+ label {slug} ({color})"),
            DiffOp::UpdateLabel { slug, .. } => format!("~ label {slug}"),
            DiffOp::CreateList { slug, name, .. } => format!("+ list {slug} \"{name}\""),
            DiffOp::UpdateList { slug, new_name, .. } => format!("~ list {slug} → \"{new_name}\""),
            DiffOp::CreateCard {
                list_slug,
                card_slug,
                name,
                ..
            } => format!("+ card {list_slug}/{card_slug} \"{name}\""),
            DiffOp::UpdateCard {
                list_slug,
                card_slug,
                new_name,
                ..
            } => format!("~ card {list_slug}/{card_slug} \"{new_name}\""),
            DiffOp::ArchiveCard {
                list_slug, name, ..
            } => format!("- card {list_slug} \"{name}\" (archive orphan)"),
        }
    }
}

/// Snapshot of remote board state, keyed for diff lookup.
pub struct RemoteSnapshot {
    pub labels_by_name: HashMap<String, RLabel>,
    pub lists_by_name: HashMap<String, RList>,
    /// list_id → cards on that list
    pub cards_by_list: HashMap<String, Vec<RCard>>,
}

pub fn compute_ops(local: &VCSBoard, remote: &RemoteSnapshot) -> Vec<DiffOp> {
    let mut ops = Vec::new();

    diff_labels(local, remote, &mut ops);
    diff_lists_and_cards(local, remote, &mut ops);

    ops
}

fn diff_labels(local: &VCSBoard, remote: &RemoteSnapshot, ops: &mut Vec<DiffOp>) {
    for (slug, def) in &local.labels {
        let display_name = label_display_name(slug, def);
        // Match remote labels by their display name (Trello labels lack a stable slug).
        if let Some(existing) = remote.labels_by_name.get(&display_name) {
            let old_color = existing.color.clone().unwrap_or_default();
            if old_color != def.color || existing.name != display_name {
                ops.push(DiffOp::UpdateLabel {
                    id: existing.id.clone(),
                    slug: slug.clone(),
                    old_name: existing.name.clone(),
                    new_name: display_name,
                    old_color,
                    new_color: def.color.clone(),
                });
            }
        } else {
            ops.push(DiffOp::CreateLabel {
                slug: slug.clone(),
                name: display_name,
                color: def.color.clone(),
            });
        }
    }
}

fn diff_lists_and_cards(local: &VCSBoard, remote: &RemoteSnapshot, ops: &mut Vec<DiffOp>) {
    // Resolve label slug → remote label id (for existing labels only — newly
    // created ones get resolved after the CreateLabel op runs, in the executor).
    let label_id_for_slug: BTreeMap<String, String> = local
        .labels
        .iter()
        .filter_map(|(slug, def)| {
            let name = label_display_name(slug, def);
            remote
                .labels_by_name
                .get(&name)
                .map(|l| (slug.clone(), l.id.clone()))
        })
        .collect();

    for (list_slug, list_def) in &local.lists {
        let existing_list = remote.lists_by_name.get(&list_def.name);
        match existing_list {
            None => {
                ops.push(DiffOp::CreateList {
                    slug: list_slug.clone(),
                    name: list_def.name.clone(),
                    position: list_def.position,
                });
                // All cards in a brand-new list are creates.
                for (card_slug, card) in &list_def.cards {
                    ops.push(DiffOp::CreateCard {
                        list_slug: list_slug.clone(),
                        card_slug: card_slug.clone(),
                        name: card.name.clone(),
                        desc: card.desc.clone(),
                        label_slugs: card.labels.clone(),
                    });
                }
            }
            Some(existing) => {
                if existing.name != list_def.name {
                    ops.push(DiffOp::UpdateList {
                        id: existing.id.clone(),
                        slug: list_slug.clone(),
                        old_name: existing.name.clone(),
                        new_name: list_def.name.clone(),
                    });
                }
                diff_cards_for_list(
                    list_slug,
                    list_def,
                    &existing.id,
                    remote,
                    &label_id_for_slug,
                    ops,
                );
            }
        }
    }
}

fn diff_cards_for_list(
    list_slug: &str,
    list_def: &ListDef,
    remote_list_id: &str,
    remote: &RemoteSnapshot,
    label_id_for_slug: &BTreeMap<String, String>,
    ops: &mut Vec<DiffOp>,
) {
    let remote_cards: &[RCard] = remote
        .cards_by_list
        .get(remote_list_id)
        .map(|v| v.as_slice())
        .unwrap_or(&[]);

    let remote_by_name: HashMap<&str, &RCard> =
        remote_cards.iter().map(|c| (c.name.as_str(), c)).collect();

    let wanted_names: HashSet<String> = list_def.cards.values().map(|c| c.name.clone()).collect();

    for (card_slug, card) in &list_def.cards {
        if let Some(existing) = remote_by_name.get(card.name.as_str()) {
            let new_label_ids = resolve_label_ids(&card.labels, label_id_for_slug);
            let mut sorted_new = new_label_ids.clone();
            sorted_new.sort();
            let mut sorted_old = existing.id_labels.clone();
            sorted_old.sort();

            if existing.name != card.name || existing.desc != card.desc || sorted_old != sorted_new
            {
                ops.push(DiffOp::UpdateCard {
                    id: existing.id.clone(),
                    list_slug: list_slug.to_string(),
                    card_slug: card_slug.clone(),
                    old_name: existing.name.clone(),
                    new_name: card.name.clone(),
                    old_desc: existing.desc.clone(),
                    new_desc: card.desc.clone(),
                    old_label_slugs: sorted_old,
                    new_label_slugs: card.labels.clone(),
                });
            }
        } else {
            ops.push(DiffOp::CreateCard {
                list_slug: list_slug.to_string(),
                card_slug: card_slug.clone(),
                name: card.name.clone(),
                desc: card.desc.clone(),
                label_slugs: card.labels.clone(),
            });
        }
    }

    if list_def.managed {
        for card in remote_cards {
            if !wanted_names.contains(&card.name) {
                ops.push(DiffOp::ArchiveCard {
                    id: card.id.clone(),
                    list_slug: list_slug.to_string(),
                    name: card.name.clone(),
                });
            }
        }
    }
}

fn resolve_label_ids(
    slugs: &[String],
    label_id_for_slug: &BTreeMap<String, String>,
) -> Vec<String> {
    slugs
        .iter()
        .filter_map(|s| label_id_for_slug.get(s).cloned())
        .collect()
}

pub fn label_display_name(slug: &str, def: &LabelDef) -> String {
    def.name.clone().unwrap_or_else(|| slug.to_string())
}

pub fn card_label_display_names(
    slugs: &[String],
    local_labels: &BTreeMap<String, LabelDef>,
) -> Vec<String> {
    slugs
        .iter()
        .filter_map(|s| local_labels.get(s).map(|def| label_display_name(s, def)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::model::Card;
    use crate::sync::board::*;

    fn empty_remote() -> RemoteSnapshot {
        RemoteSnapshot {
            labels_by_name: HashMap::new(),
            lists_by_name: HashMap::new(),
            cards_by_list: HashMap::new(),
        }
    }

    #[test]
    fn create_all_on_empty_remote() {
        let mut board = VCSBoard {
            metadata: Metadata {
                board_name: "T".into(),
                board_id: None,
            },
            ..Default::default()
        };
        board.labels.insert(
            "common".into(),
            LabelDef {
                color: "sky".into(),
                name: None,
                id: None,
            },
        );
        let mut list = ListDef {
            name: "Brainrots".into(),
            position: Some(1),
            managed: true,
            id: None,
            cards: Default::default(),
        };
        list.cards.insert(
            "noobini".into(),
            CardDef {
                name: "Noobini".into(),
                desc: "".into(),
                labels: vec!["common".into()],
                ..Default::default()
            },
        );
        board.lists.insert("brainrots".into(), list);

        let ops = compute_ops(&board, &empty_remote());
        assert!(matches!(ops[0], DiffOp::CreateLabel { .. }));
        assert!(matches!(ops[1], DiffOp::CreateList { .. }));
        assert!(matches!(ops[2], DiffOp::CreateCard { .. }));
    }

    #[test]
    fn unmanaged_list_keeps_orphans() {
        let mut board = VCSBoard {
            metadata: Metadata {
                board_name: "T".into(),
                board_id: None,
            },
            ..Default::default()
        };
        board.lists.insert(
            "narr".into(),
            ListDef {
                name: "Narrative".into(),
                position: None,
                managed: false,
                id: None,
                cards: Default::default(),
            },
        );

        let mut remote = empty_remote();
        remote.lists_by_name.insert(
            "Narrative".into(),
            crate::api::model::List {
                id: "L1".into(),
                name: "Narrative".into(),
                closed: false,
                pos: 1.0,
                id_board: "B".into(),
            },
        );
        remote.cards_by_list.insert(
            "L1".into(),
            vec![Card {
                id: "C1".into(),
                name: "Orphan card".into(),
                desc: "".into(),
                closed: false,
                id_list: "L1".into(),
                id_labels: vec![],
                pos: 1.0,
            }],
        );

        let ops = compute_ops(&board, &remote);
        assert!(
            !ops.iter()
                .any(|op| matches!(op, DiffOp::ArchiveCard { .. })),
            "unmanaged list must never archive: {ops:#?}"
        );
    }

    #[test]
    fn managed_list_archives_orphans() {
        let mut board = VCSBoard {
            metadata: Metadata {
                board_name: "T".into(),
                board_id: None,
            },
            ..Default::default()
        };
        board.lists.insert(
            "br".into(),
            ListDef {
                name: "Brainrots".into(),
                position: None,
                managed: true,
                id: None,
                cards: Default::default(),
            },
        );

        let mut remote = empty_remote();
        remote.lists_by_name.insert(
            "Brainrots".into(),
            crate::api::model::List {
                id: "L1".into(),
                name: "Brainrots".into(),
                closed: false,
                pos: 1.0,
                id_board: "B".into(),
            },
        );
        remote.cards_by_list.insert(
            "L1".into(),
            vec![Card {
                id: "C9".into(),
                name: "Removed pet".into(),
                desc: "".into(),
                closed: false,
                id_list: "L1".into(),
                id_labels: vec![],
                pos: 1.0,
            }],
        );

        let ops = compute_ops(&board, &remote);
        assert_eq!(ops.len(), 1);
        assert!(matches!(ops[0], DiffOp::ArchiveCard { .. }));
    }
}
