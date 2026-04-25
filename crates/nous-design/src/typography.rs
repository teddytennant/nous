//! Typography tokens.
//!
//! The terminal cannot vary weight or tracking the way HTML/SwiftUI/Compose
//! can — but the design language treats typography as the primary instrument,
//! so these constants exist as documentation and as a contract for future
//! pixel-grade renderers (Sixel/Kitty graphics, embedded WebViews).
//!
//! **Mapping for ratatui surfaces:**
//! - `display` / `heading` → bold modifier, where supported by the terminal.
//! - `body` → no modifier.
//! - `meta` → dim modifier, optionally uppercase by the caller.
//! - `mono` → terminals are mono by default; this is the conceptual default.

/// Typography role — one of the seven canonical text styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Serif display headings (Source Serif 4, weight 400, tracking -0.02em).
    Display,
    /// Sans-serif headings (Geist, weight 500, tracking -0.01em).
    Heading,
    /// Body copy (Geist, weight 400, tracking 0).
    Body,
    /// Metadata: uppercase, +0.04em tracking (Geist, weight 400).
    Meta,
    /// Monospace for data: DIDs, hashes, peer counts (Geist Mono, weight 400).
    Mono,
}

/// Font family declarations, in CSS-equivalent strings. Surface code may
/// translate these to platform-native font descriptors.
pub mod family {
    /// Source Serif 4 — display only.
    pub const DISPLAY: &str = "Source Serif 4";
    /// Geist — heading and body.
    pub const SANS: &str = "Geist";
    /// Geist Mono — data.
    pub const MONO: &str = "Geist Mono";
}

/// Font weights, as numeric values (CSS / OpenType convention).
pub mod weight {
    /// Display weight — 400.
    pub const DISPLAY: u16 = 400;
    /// Heading weight — 500.
    pub const HEADING: u16 = 500;
    /// Body weight — 400.
    pub const BODY: u16 = 400;
    /// Meta weight — 400.
    pub const META: u16 = 400;
    /// Mono weight — 400.
    pub const MONO: u16 = 400;
}

/// Letter-spacing (tracking) in em units, expressed as f32.
pub mod tracking {
    /// `-0.02em` — display.
    pub const DISPLAY: f32 = -0.02;
    /// `-0.01em` — heading.
    pub const HEADING: f32 = -0.01;
    /// `0` — body.
    pub const BODY: f32 = 0.0;
    /// `+0.04em` — meta.
    pub const META: f32 = 0.04;
}

/// Type scale in rem.
///
/// Indices are the canonical step number; 0 is the smallest. Body sits at
/// step 2 (`1.0rem`).
pub const SCALE_REM: [f32; 9] = [0.75, 0.875, 1.0, 1.125, 1.375, 1.75, 2.25, 3.0, 4.5];

/// Line heights as multipliers of font size.
pub mod line_height {
    /// Tight: 1.15. Used by display.
    pub const TIGHT: f32 = 1.15;
    /// Normal: 1.5. Used by body.
    pub const NORMAL: f32 = 1.5;
    /// Mono: 1.4. Compromise between density and breathing.
    pub const MONO: f32 = 1.4;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_is_monotonic() {
        for window in SCALE_REM.windows(2) {
            assert!(window[0] < window[1]);
        }
    }

    #[test]
    fn body_is_one_rem() {
        assert_eq!(SCALE_REM[2], 1.0);
    }
}
