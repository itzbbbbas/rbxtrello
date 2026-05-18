//! rbxtrello library — exposes parsing/diff internals so integration tests
//! and downstream tools can drive sync logic without the CLI entry point.

pub mod api;
pub mod sync;
pub mod ui;
pub mod utils;

pub type Result<T> = anyhow::Result<T>;
