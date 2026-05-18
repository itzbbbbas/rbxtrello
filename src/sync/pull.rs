use std::collections::BTreeMap;

use log::info;

use crate::Result;
use crate::api::trello;
use crate::sync::TOML_PATH;
use crate::sync::board::{CardDef, LabelDef, ListDef, Metadata, VCSBoard};
use crate::utils::{self, slugify};

pub async fn run(board_id_override: Option<String>) -> Result<()> {
    let existing = if tokio::fs::try_exists(TOML_PATH).await.unwrap_or(false) {
        Some(VCSBoard::load().await?)
    } else {
        None
    };

    let board_id = board_id_override
        .or_else(|| existing.as_ref().and_then(|b| b.metadata.board_id.clone()))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "no board id — set [metadata].board_id in rbxtrello.toml or pass --board-id"
            )
        })?;

    info!("Pulling board {board_id}...");

    let board = trello::get_board(&board_id).await?;
    let labels = trello::list_board_labels(&board_id).await?;
    let lists = trello::list_board_lists(&board_id).await?;

    let mut new_board = VCSBoard {
        metadata: Metadata {
            board_name: board.name.clone(),
            board_id: Some(board.id.clone()),
        },
        labels: BTreeMap::new(),
        lists: BTreeMap::new(),
    };

    for label in labels {
        if label.name.trim().is_empty() {
            // Skip Trello's "color-only" labels — they can't be referenced by slug.
            continue;
        }
        let slug = slugify(&label.name);
        if slug.is_empty() {
            continue;
        }
        new_board.labels.insert(
            slug.clone(),
            LabelDef {
                color: label.color.unwrap_or_default(),
                name: Some(label.name.clone()),
                id: Some(label.id.clone()),
            },
        );
    }

    let label_slug_for_id: BTreeMap<String, String> = new_board
        .labels
        .iter()
        .filter_map(|(slug, def)| def.id.clone().map(|id| (id, slug.clone())))
        .collect();

    // Normalize Trello's huge float positions to 1-based ordinal ranks so the
    // toml stays human-readable. Trello accepts integer positions on push.
    let mut sorted_lists: Vec<_> = lists.into_iter().filter(|l| !l.closed).collect();
    sorted_lists.sort_by(|a, b| {
        a.pos
            .partial_cmp(&b.pos)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for (rank, list) in sorted_lists.into_iter().enumerate() {
        let list_slug = slugify(&list.name);
        let cards = trello::list_cards(&list.id).await?;
        let mut card_map = BTreeMap::new();
        for card in cards {
            if card.closed {
                continue;
            }
            let card_slug = slugify(&card.name);
            if card_slug.is_empty() {
                continue;
            }
            let label_slugs = card
                .id_labels
                .iter()
                .filter_map(|id| label_slug_for_id.get(id).cloned())
                .collect();

            card_map.insert(
                card_slug,
                CardDef {
                    name: card.name,
                    desc: card.desc,
                    labels: label_slugs,
                    id: Some(card.id),
                    cover: None,
                    checklists: vec![],
                    fields: Default::default(),
                },
            );
        }

        new_board.lists.insert(
            list_slug,
            ListDef {
                name: list.name,
                position: Some((rank + 1) as u32),
                managed: true,
                id: Some(list.id),
                cards: card_map,
            },
        );
    }

    let serialized = toml::to_string_pretty(&new_board)?;
    utils::write_string(TOML_PATH, &serialized).await?;
    info!("Wrote {TOML_PATH}");

    if let Some(prev) = existing {
        let prev_lists = prev.lists.len();
        let new_lists = new_board.lists.len();
        let prev_cards: usize = prev.lists.values().map(|l| l.cards.len()).sum();
        let new_cards: usize = new_board.lists.values().map(|l| l.cards.len()).sum();
        info!(
            "lists: {prev_lists} → {new_lists}    cards: {prev_cards} → {new_cards}    labels: {} → {}",
            prev.labels.len(),
            new_board.labels.len()
        );
    } else {
        info!(
            "Seeded {} lists, {} labels, {} cards",
            new_board.lists.len(),
            new_board.labels.len(),
            new_board
                .lists
                .values()
                .map(|l| l.cards.len())
                .sum::<usize>(),
        );
    }

    Ok(())
}
