//! Text expander engine — abbreviation monitoring and snippet expansion.

pub mod buffer;
pub mod engine;
pub mod export;
pub mod import;
pub mod keymap;
pub mod matcher;
pub mod template;

pub use engine::ExpanderEngine;
