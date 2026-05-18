pub mod board;
pub mod diff;
pub mod pull;
pub mod push;

pub const TOML_PATH: &str = "rbxtrello.toml";

/// Valid Trello label colors (Trello rejects others).
pub const VALID_LABEL_COLORS: &[&str] = &[
    "yellow", "purple", "blue", "red", "green", "orange", "black", "sky", "pink", "lime",
];
