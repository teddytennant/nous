//! Spacing tokens.
//!
//! The web/native scale is `4 8 12 16 24 32 48 64 96 128` (in pixels).
//! For terminal surfaces, where the horizontal cell is roughly 8–10px and
//! vertical is roughly 16–22px, we expose a parallel scale measured in cell
//! units. Two horizontal cells ≈ 16–20px ≈ one `S2` step.
//!
//! ## Mapping (cells → web)
//!
//! | const | cells | web (px) |
//! |-------|-------|----------|
//! | `S1`  | 1     | 4        |
//! | `S2`  | 2     | 8        |
//! | `S3`  | 3     | 12       |
//! | `S4`  | 4     | 16       |
//! | `S6`  | 6     | 24       |
//! | `S8`  | 8     | 32       |
//! | `S12` | 12    | 48       |
//! | `S16` | 16    | 64       |
//! | `S24` | 24    | 96       |
//! | `S32` | 32    | 128      |
//!
//! Use these for ratatui `Constraint::Length(...)`, gutters between blocks,
//! and pad-around values.

/// 1 cell ≈ 4px web.
pub const S1: u16 = 1;
/// 2 cells ≈ 8px web.
pub const S2: u16 = 2;
/// 3 cells ≈ 12px web.
pub const S3: u16 = 3;
/// 4 cells ≈ 16px web.
pub const S4: u16 = 4;
/// 6 cells ≈ 24px web.
pub const S6: u16 = 6;
/// 8 cells ≈ 32px web.
pub const S8: u16 = 8;
/// 12 cells ≈ 48px web.
pub const S12: u16 = 12;
/// 16 cells ≈ 64px web.
pub const S16: u16 = 16;
/// 24 cells ≈ 96px web.
pub const S24: u16 = 24;
/// 32 cells ≈ 128px web.
pub const S32: u16 = 32;

/// Border radii. Terminals can't draw rounded corners, so this is informational
/// — but `PANEL` (4px) is the maximum allowed by the design language. Anything
/// rounder reads as software, not editorial.
pub mod radius {
    /// Sharp — square corners.
    pub const SHARP: u16 = 0;
    /// Soft — 2px (subtle).
    pub const SOFT: u16 = 2;
    /// Panel — 4px (maximum for content).
    pub const PANEL: u16 = 4;
    /// Pill — fully round (controls only, never panels).
    pub const PILL: u16 = 999;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_is_well_ordered() {
        assert!(S1 < S2);
        assert!(S2 < S3);
        assert!(S3 < S4);
        assert!(S4 < S6);
        assert!(S6 < S8);
        assert!(S32 == 32);
    }

    #[test]
    fn radii_match_spec() {
        assert_eq!(radius::SHARP, 0);
        assert_eq!(radius::SOFT, 2);
        assert_eq!(radius::PANEL, 4);
        assert_eq!(radius::PILL, 999);
    }
}
