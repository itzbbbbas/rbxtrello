//! Integration tests for rbxtrello.toml schema parsing + validation.

use pretty_assertions::assert_eq;

#[test]
fn parses_minimal_toml() {
    let src = r#"
[metadata]
board_name = "Test"
"#;
    let board: rbxtrello::sync::board::VCSBoard = toml::from_str(src).expect("minimal toml parses");
    assert_eq!(board.metadata.board_name, "Test");
    assert!(board.labels.is_empty());
    assert!(board.lists.is_empty());
    board.validate().expect("minimal is valid");
}

#[test]
fn parses_full_example() {
    let src = r#"
[metadata]
board_name = "Wiki"
board_id = "abc123"

[labels.common]
color = "sky"
name = "Common"

[labels.tradable]
color = "green"

[lists.brainrots]
name = "Brainrots"
position = 2
managed = true

[lists.brainrots.cards.noobini]
name = "Noobini"
desc = "Common pet"
labels = ["common", "tradable"]

[lists.mechanics]
name = "Mechanics"
managed = false

[lists.mechanics.cards.round_flow]
name = "Round Flow"
desc = "Intermission → ..."
"#;
    let board: rbxtrello::sync::board::VCSBoard = toml::from_str(src).unwrap();
    board.validate().unwrap();
    assert_eq!(board.labels.len(), 2);
    assert_eq!(board.lists.len(), 2);
    assert_eq!(board.lists["brainrots"].cards.len(), 1);
    assert_eq!(
        board.lists["brainrots"].cards["noobini"].labels,
        vec!["common".to_string(), "tradable".to_string()]
    );
    assert!(!board.lists["mechanics"].managed);
    assert!(board.lists["brainrots"].managed);
}

#[test]
fn rejects_invalid_color() {
    let src = r#"
[metadata]
board_name = "T"

[labels.bad]
color = "magenta"
"#;
    let board: rbxtrello::sync::board::VCSBoard = toml::from_str(src).unwrap();
    let err = board.validate().expect_err("magenta is not a Trello color");
    assert!(err.to_string().contains("magenta"), "got: {err}");
}

#[test]
fn rejects_unknown_label_reference() {
    let src = r#"
[metadata]
board_name = "T"

[lists.x]
name = "X"

[lists.x.cards.c]
name = "C"
labels = ["nonexistent"]
"#;
    let board: rbxtrello::sync::board::VCSBoard = toml::from_str(src).unwrap();
    let err = board.validate().expect_err("unknown label ref must fail");
    assert!(err.to_string().contains("nonexistent"), "got: {err}");
}

#[test]
fn rejects_missing_board_name() {
    let src = r#"
[metadata]
board_name = ""
"#;
    let board: rbxtrello::sync::board::VCSBoard = toml::from_str(src).unwrap();
    let err = board.validate().expect_err("empty board name");
    assert!(err.to_string().contains("board_name"), "got: {err}");
}

#[test]
fn roundtrip_preserves_structure() {
    let original = r#"
[metadata]
board_name = "Roundtrip"
board_id = "abc"

[labels.rare]
color = "blue"
name = "Rare"

[lists.brainrots]
name = "Brainrots"
position = 1
managed = true

[lists.brainrots.cards.x]
name = "X"
desc = "d"
labels = ["rare"]
"#;
    let board: rbxtrello::sync::board::VCSBoard = toml::from_str(original).unwrap();
    let serialized = toml::to_string_pretty(&board).unwrap();
    let board2: rbxtrello::sync::board::VCSBoard = toml::from_str(&serialized).unwrap();
    assert_eq!(board.metadata.board_name, board2.metadata.board_name);
    assert_eq!(board.labels.len(), board2.labels.len());
    assert_eq!(
        board.lists["brainrots"].cards["x"].labels,
        board2.lists["brainrots"].cards["x"].labels
    );
}
