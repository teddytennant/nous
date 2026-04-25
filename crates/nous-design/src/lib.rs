//! # nous-design
//!
//! Canonical design tokens for the Nous editorial design language.
//!
//! This crate is the source of truth for every Rust-facing surface (TUI, CLI,
//! native binaries). Web, iOS, and Android consume the same values via their
//! own platform-specific files derived from `docs/design/tokens.json`.
//!
//! ## Modules
//!
//! - [`palette`] — color tokens (RGB triples, plus optional `ratatui::Color`s).
//! - [`typography`] — informational weight/tracking constants. Terminals can't
//!   render variable typography, but exporting these documents intent and lets
//!   future renderers (Sixel/Kitty graphics) honor it.
//! - [`spacing`] — scale in cell units, with the documented mapping back to
//!   the 4px web base unit.
//!
//! ## Features
//!
//! - `ratatui` (optional) — pulls in [`ratatui`] and re-exports
//!   [`ratatui::style::Color`] constants and helper [`ratatui::style::Style`]s.
//!
//! No `default` features; consumers opt in. This keeps the crate buildable on
//! `no_std` targets in the future and avoids dragging ratatui into binaries
//! that just want raw RGB.
//!
//! ## Design language summary
//!
//! Editorial minimalism: ink and ivory, a single oxblood accent used like a
//! stamp. Typography carries hierarchy. Hairlines, not borders. See
//! `docs/DESIGN.md` for the full principles.

#![deny(missing_docs)]

pub mod palette;
pub mod spacing;
pub mod typography;
