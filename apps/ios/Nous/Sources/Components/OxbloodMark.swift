import SwiftUI

/// The brand stamp. A 1pt-stroke geometric mark used in nav active state,
/// read receipts, copy affordances. Shape varies by `style`; default is the
/// 6pt hairline circle.
struct OxbloodMark: View {
    enum Style {
        /// 6pt hairline circle. Used as the active-nav stamp.
        case dot
        /// 6pt filled disc. Used for read receipts.
        case disc
        /// 8pt circumflex (^). Used as a "you are here" mark.
        case caret
        /// 10pt arrow tail (→). Used for inline links.
        case arrow
        /// 10pt copy glyph (two overlapping squares). Used for copy actions.
        case copy
    }

    var style: Style = .dot
    var color: Color = NousTheme.oxblood
    var size: CGFloat = 8

    var body: some View {
        shape
            .frame(width: size, height: size)
            .accessibilityHidden(true)
    }

    @ViewBuilder
    private var shape: some View {
        switch style {
        case .dot:
            Circle().stroke(color, lineWidth: 1)
        case .disc:
            Circle().fill(color)
        case .caret:
            Path { p in
                p.move(to: CGPoint(x: 0, y: size * 0.7))
                p.addLine(to: CGPoint(x: size * 0.5, y: size * 0.2))
                p.addLine(to: CGPoint(x: size, y: size * 0.7))
            }
            .stroke(color, style: StrokeStyle(lineWidth: 1, lineCap: .round, lineJoin: .miter))
        case .arrow:
            Path { p in
                p.move(to: CGPoint(x: 0, y: size * 0.5))
                p.addLine(to: CGPoint(x: size, y: size * 0.5))
                p.move(to: CGPoint(x: size * 0.55, y: size * 0.15))
                p.addLine(to: CGPoint(x: size, y: size * 0.5))
                p.addLine(to: CGPoint(x: size * 0.55, y: size * 0.85))
            }
            .stroke(color, style: StrokeStyle(lineWidth: 1, lineCap: .round, lineJoin: .round))
        case .copy:
            ZStack {
                Rectangle()
                    .stroke(color, lineWidth: 1)
                    .frame(width: size * 0.7, height: size * 0.7)
                    .offset(x: -size * 0.12, y: -size * 0.12)
                Rectangle()
                    .stroke(color, lineWidth: 1)
                    .frame(width: size * 0.7, height: size * 0.7)
                    .offset(x: size * 0.12, y: size * 0.12)
            }
        }
    }
}

#Preview {
    HStack(spacing: 24) {
        OxbloodMark(style: .dot)
        OxbloodMark(style: .disc)
        OxbloodMark(style: .caret)
        OxbloodMark(style: .arrow)
        OxbloodMark(style: .copy)
    }
    .padding(32)
    .frame(maxWidth: .infinity, maxHeight: .infinity)
    .background(NousTheme.background)
}
