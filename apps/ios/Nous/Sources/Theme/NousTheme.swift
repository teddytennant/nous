import SwiftUI

/// Editorial design system for Nous.
///
/// Ink and ivory page surface, oxblood accent used like a stamp, hairline
/// rules in place of borders. The palette is small and intentional — eight
/// surface tokens plus two state tokens. Hierarchy is carried by typography
/// (serif display, sans body, mono for data).
///
/// The token names below preserve the public surface used by existing views
/// (`NousTheme.background`, `.text`, `.border`, `.accent`, `.surface`,
/// `.surfaceElevated`, `.textSecondary`, `.textMuted`, `.accentDim`,
/// `.success`, `.error`). Their *values* are remapped to the new palette.
/// New tokens (`.ink`, `.ivory`, `.oxblood`, `.stone`, `.rule`, `.sage`,
/// `.clay`) are exposed alongside.
///
/// Source of truth: `docs/design/tokens.json`.
enum NousTheme {
    // MARK: - Raw editorial tokens (dark, default)

    /// `#0E0E0C` — primary surface (the page).
    static let ink = Color(hex: 0x0E0E0C)
    /// `#161613` — raised surface (cards, panels).
    static let ink2 = Color(hex: 0x161613)
    /// `#EFEAE0` — primary text.
    static let ivory = Color(hex: 0xEFEAE0)
    /// `#C9C3B6` — secondary text.
    static let ivoryDim = Color(hex: 0xC9C3B6)
    /// `#6F6A60` — tertiary text, metadata.
    static let stone = Color(hex: 0x6F6A60)
    /// `#1F1D1A` — 1px hairline rules.
    static let rule = Color(hex: 0x1F1D1A)
    /// `#B23A3A` — single accent: actions, marks, focus rings.
    static let oxblood = Color(hex: 0xB23A3A)
    /// `#7A2A2A` — pressed / visited oxblood.
    static let oxbloodDim = Color(hex: 0x7A2A2A)
    /// `#8FA48A` — positive state.
    static let sage = Color(hex: 0x8FA48A)
    /// `#C2785A` — warning state.
    static let clay = Color(hex: 0xC2785A)

    // MARK: - Light-mode variants

    /// `#F4F1EA` — light primary surface.
    static let paper = Color(hex: 0xF4F1EA)
    /// `#14130F` — light primary text.
    static let inkText = Color(hex: 0x14130F)
    /// `#DCD7CC` — light hairline rules.
    static let hairline = Color(hex: 0xDCD7CC)

    // MARK: - Semantic aliases (preserve existing call-sites)

    /// Page surface.
    static let background = ink
    /// Card / panel surface.
    static let surface = ink2
    /// Elevated surface — same as `surface` (no Material elevation in this language).
    static let surfaceElevated = ink2
    /// 1px hairline.
    static let border = rule
    /// Primary text.
    static let text = ivory
    /// Secondary text.
    static let textSecondary = ivoryDim
    /// Tertiary text / metadata.
    static let textMuted = stone
    /// Single accent (oxblood).
    static let accent = oxblood
    /// Dimmed accent — pressed / visited.
    static let accentDim = oxbloodDim
    /// Positive state.
    static let success = sage
    /// Warning / error state.
    static let error = clay

    // MARK: - Typography
    //
    // Source Serif 4 for display, Geist for sans, Geist Mono for data. The
    // platform falls back to the system serif / sans / mono when bundles
    // aren't installed — which is fine for development. Production bundles
    // ship the fonts via the asset catalog.

    /// Display — serif, used for hero / section titles. `4.5rem` ≈ 72pt.
    static let display = Font.custom("SourceSerif4", size: 56)
        .weight(.regular)
    /// Headline large — serif, `3rem` ≈ 48pt.
    static let headlineLarge = Font.custom("SourceSerif4", size: 36)
        .weight(.regular)
    /// Headline medium — sans heading.
    static let headlineMedium = Font.system(size: 28, weight: .medium)
    /// Title large — sans.
    static let titleLarge = Font.system(size: 20, weight: .medium)
    /// Body — sans, primary reading size.
    static let body = Font.system(size: 16, weight: .regular)
    /// Body small — sans.
    static let bodySmall = Font.system(size: 14, weight: .regular)
    /// Label / metadata — uppercase, tracked. SwiftUI applies tracking via
    /// `.tracking(_:)` modifier on the `Text` view.
    static let label = Font.system(size: 12, weight: .regular)
    /// Mono small — for DIDs, hashes, counts.
    static let mono = Font.system(size: 14, weight: .regular, design: .monospaced)
    /// Mono large — featured numerals (balances, block heights).
    static let monoLarge = Font.system(size: 24, weight: .regular, design: .monospaced)

    /// Tracking value to apply to `label` text via `.tracking(NousTheme.metaTracking)`.
    static let metaTracking: CGFloat = 0.5

    // MARK: - Spacing — 4px base
    //
    // Names follow the existing call-sites (XS/SM/MD/LG/XL); numbered tokens
    // (`s1`...`s32`) match `tokens.json` directly.

    /// 4pt.
    static let s1: CGFloat = 4
    /// 8pt.
    static let s2: CGFloat = 8
    /// 12pt.
    static let s3: CGFloat = 12
    /// 16pt.
    static let s4: CGFloat = 16
    /// 24pt.
    static let s6: CGFloat = 24
    /// 32pt.
    static let s8: CGFloat = 32
    /// 48pt.
    static let s12: CGFloat = 48
    /// 64pt.
    static let s16: CGFloat = 64

    /// Existing alias — 4pt.
    static let spacingXS: CGFloat = s1
    /// Existing alias — 8pt.
    static let spacingSM: CGFloat = s2
    /// Existing alias — 16pt.
    static let spacingMD: CGFloat = s4
    /// Existing alias — 24pt.
    static let spacingLG: CGFloat = s6
    /// Existing alias — 32pt.
    static let spacingXL: CGFloat = s8

    // MARK: - Radius — sharp by default

    /// Sharp — square corners. The default for content surfaces.
    static let radiusSharp: CGFloat = 0
    /// Soft — 2pt.
    static let radiusSoft: CGFloat = 2
    /// Panel — 4pt. Maximum allowed for content panels.
    static let radiusPanel: CGFloat = 4
    /// Pill — fully round. Reserved for control affordances (toggles, chips).
    static let radiusPill: CGFloat = 999

    /// Existing aliases.
    static let radiusSM: CGFloat = radiusSoft
    static let radiusMD: CGFloat = radiusPanel

    // MARK: - Motion

    /// State change duration: 0.16s.
    static let durationState: Double = 0.16
    /// Page transition duration: 0.24s.
    static let durationPage: Double = 0.24
    /// Canonical easing — single deceleration curve.
    static let easing = Animation.timingCurve(0.2, 0, 0, 1)
}

extension Color {
    init(hex: UInt, alpha: Double = 1.0) {
        self.init(
            .sRGB,
            red: Double((hex >> 16) & 0xFF) / 255.0,
            green: Double((hex >> 8) & 0xFF) / 255.0,
            blue: Double(hex & 0xFF) / 255.0,
            opacity: alpha
        )
    }
}
