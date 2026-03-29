import SwiftUI

/// Infinite Minimalism design system for Nous.
///
/// Deep black backgrounds, warm gold accent, Swiss typography.
/// Less, but better. Every element earns its place.
enum NousTheme {
    // MARK: - Colors

    static let background = Color.black
    static let surface = Color(hex: 0x0A0A0A)
    static let surfaceElevated = Color(hex: 0x111111)
    static let border = Color(hex: 0x1A1A1A)

    static let text = Color(hex: 0xFAFAFA)
    static let textSecondary = Color(hex: 0xA3A3A3)
    static let textMuted = Color(hex: 0x737373)

    static let accent = Color(hex: 0xD4AF37) // warm gold
    static let accentDim = Color(hex: 0xD4AF37).opacity(0.15)

    static let success = Color(hex: 0x22C55E)
    static let error = Color(hex: 0xEF4444)

    // MARK: - Typography

    static let headlineLarge = Font.system(size: 28, weight: .ultraLight)
    static let headlineMedium = Font.system(size: 22, weight: .light)
    static let titleLarge = Font.system(size: 18, weight: .light)
    static let body = Font.system(size: 14, weight: .light)
    static let bodySmall = Font.system(size: 13, weight: .light)
    static let label = Font.system(size: 11, weight: .regular)
    static let mono = Font.system(size: 13, weight: .light, design: .monospaced)
    static let monoLarge = Font.system(size: 20, weight: .ultraLight, design: .monospaced)

    // MARK: - Spacing

    static let spacingXS: CGFloat = 4
    static let spacingSM: CGFloat = 8
    static let spacingMD: CGFloat = 16
    static let spacingLG: CGFloat = 24
    static let spacingXL: CGFloat = 32

    // MARK: - Radius

    static let radiusSM: CGFloat = 4
    static let radiusMD: CGFloat = 8
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
