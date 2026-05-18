use std::collections::HashMap;

use log::{info, warn};

use crate::Result;
use crate::api::trello;
use crate::sync::TOML_PATH;
use crate::sync::board::VCSBoard;
use crate::sync::diff::{self, DiffOp, RemoteSnapshot};
use crate::ui::confirm::{ConfirmState, ConfirmViewer};
use crate::utils;

pub struct PushOptions {
    pub board_id_override: Option<String>,
    pub auto_confirm: bool,
    pub dry_run: bool,
}

pub async fn run(opts: PushOptions) -> Result<()> {
    let mut local = VCSBoard::load().await?;

    let board_id = match opts.board_id_override.or(local.metadata.board_id.clone()) {
        Some(id) => id,
        None => {
            info!(
                "no board_id set — creating new board \"{}\"",
                local.metadata.board_name
            );
            if opts.dry_run {
                anyhow::bail!("--dry-run cannot create a new board");
            }
            let board = trello::create_board(&local.metadata.board_name).await?;
            local.metadata.board_id = Some(board.id.clone());
            write_local(&local).await?;
            board.id
        }
    };

    let snapshot = fetch_snapshot(&board_id).await?;
    let ops = diff::compute_ops(&local, &snapshot);

    if ops.is_empty() {
        info!("✓ board already in sync — nothing to do");
        return Ok(());
    }

    print_ops(&ops);

    if opts.dry_run {
        info!("--dry-run: not applying changes");
        return Ok(());
    }

    if !opts.auto_confirm {
        let prompt = format!("Apply {} changes to Trello?", ops.len());
        let state = ConfirmViewer::show_prompt(prompt).await;
        if state != ConfirmState::Confirmed {
            info!("Cancelled. No changes pushed.");
            return Ok(());
        }
    }

    apply_ops(&board_id, &mut local, ops, &snapshot).await?;
    write_local(&local).await?;
    info!("✓ sync complete");
    Ok(())
}

async fn fetch_snapshot(board_id: &str) -> Result<RemoteSnapshot> {
    info!("Fetching board state...");
    let labels = trello::list_board_labels(board_id).await?;
    let lists = trello::list_board_lists(board_id).await?;

    let mut snapshot = RemoteSnapshot {
        labels_by_name: HashMap::new(),
        lists_by_name: HashMap::new(),
        cards_by_list: HashMap::new(),
    };

    for label in labels {
        if !label.name.is_empty() {
            snapshot.labels_by_name.insert(label.name.clone(), label);
        }
    }

    for list in lists {
        if list.closed {
            continue;
        }
        let cards = trello::list_cards(&list.id).await?;
        let open_cards: Vec<_> = cards.into_iter().filter(|c| !c.closed).collect();
        snapshot.cards_by_list.insert(list.id.clone(), open_cards);
        snapshot.lists_by_name.insert(list.name.clone(), list);
    }

    Ok(snapshot)
}

fn print_ops(ops: &[DiffOp]) {
    info!("Planned changes ({}):", ops.len());
    for op in ops {
        info!("  {}", op.human());
    }
}

async fn apply_ops(
    board_id: &str,
    local: &mut VCSBoard,
    ops: Vec<DiffOp>,
    snapshot: &RemoteSnapshot,
) -> Result<()> {
    // Track newly-created label IDs so card creates that follow can reference them.
    let mut label_id_by_slug: HashMap<String, String> = HashMap::new();
    for (slug, def) in &local.labels {
        let name = diff::label_display_name(slug, def);
        if let Some(rl) = snapshot.labels_by_name.get(&name) {
            label_id_by_slug.insert(slug.clone(), rl.id.clone());
        }
    }
    let mut list_id_by_slug: HashMap<String, String> = HashMap::new();
    for (slug, def) in &local.lists {
        if let Some(rl) = snapshot.lists_by_name.get(&def.name) {
            list_id_by_slug.insert(slug.clone(), rl.id.clone());
        }
    }

    for op in ops {
        match op {
            DiffOp::CreateLabel { slug, name, color } => {
                let label = trello::create_label(board_id, &name, &color).await?;
                label_id_by_slug.insert(slug.clone(), label.id.clone());
                if let Some(def) = local.labels.get_mut(&slug) {
                    def.id = Some(label.id);
                }
                info!("  + label {slug}");
            }
            DiffOp::UpdateLabel {
                id,
                slug,
                new_name,
                new_color,
                ..
            } => {
                trello::update_label(&id, &new_name, &new_color).await?;
                info!("  ~ label {slug}");
            }
            DiffOp::CreateList {
                slug,
                name,
                position,
            } => {
                let pos = position
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| "bottom".to_string());
                let list = trello::create_list(board_id, &name, &pos).await?;
                list_id_by_slug.insert(slug.clone(), list.id.clone());
                if let Some(def) = local.lists.get_mut(&slug) {
                    def.id = Some(list.id);
                }
                info!("  + list {slug}");
            }
            DiffOp::UpdateList {
                id, slug, new_name, ..
            } => {
                trello::update_list(&id, &new_name, "bottom").await?;
                info!("  ~ list {slug}");
            }
            DiffOp::CreateCard {
                list_slug,
                card_slug,
                name,
                desc,
                label_slugs,
            } => {
                let Some(list_id) = list_id_by_slug.get(&list_slug).cloned() else {
                    warn!("  skipping card {card_slug} — list {list_slug} has no id yet");
                    continue;
                };
                let label_ids: Vec<String> = label_slugs
                    .iter()
                    .filter_map(|s| label_id_by_slug.get(s).cloned())
                    .collect();
                let card = trello::create_card(&list_id, &name, &desc, &label_ids).await?;
                if let Some(list) = local.lists.get_mut(&list_slug) {
                    if let Some(def) = list.cards.get_mut(&card_slug) {
                        def.id = Some(card.id);
                    }
                }
                info!("  + card {list_slug}/{card_slug}");
            }
            DiffOp::UpdateCard {
                id,
                list_slug,
                card_slug,
                new_name,
                new_desc,
                new_label_slugs,
                ..
            } => {
                let label_ids: Vec<String> = new_label_slugs
                    .iter()
                    .filter_map(|s| label_id_by_slug.get(s).cloned())
                    .collect();
                trello::update_card(&id, &new_name, &new_desc, &label_ids).await?;
                info!("  ~ card {list_slug}/{card_slug}");
            }
            DiffOp::ArchiveCard { id, list_slug, name } => {
                trello::archive_card(&id).await?;
                info!("  - archived {list_slug}: \"{name}\"");
            }
        }
    }

    Ok(())
}

async fn write_local(board: &VCSBoard) -> Result<()> {
    let serialized = toml::to_string_pretty(board)?;
    utils::write_string(TOML_PATH, &serialized).await?;
    Ok(())
}
