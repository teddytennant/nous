import SwiftUI

/// Custom 16×16, 1pt-stroke nav glyphs. Drawn with SwiftUI `Path`, no images.
/// Stroke colour follows `foregroundStyle` (currentColor).
///
/// Names match the editorial nav surface shared with web and Android.
enum NousIconKind: Hashable {
    case dashboard
    case messages
    case wallet
    case identity
    case social
    case governance
    case ai
    case network
    case files
    case marketplace
    case settings
}

/// A single 16×16 stroke shape, scaled to whatever frame the caller provides.
/// We use a `Shape` (rather than `Canvas`) so SwiftUI's `foregroundStyle` /
/// `foregroundColor` chain tints the stroke naturally.
private struct NousIconShape: Shape {
    let kind: NousIconKind

    func path(in rect: CGRect) -> Path {
        var path = Path()
        let u = min(rect.width, rect.height) / 16
        let ox = rect.minX
        let oy = rect.minY
        func pt(_ x: CGFloat, _ y: CGFloat) -> CGPoint {
            CGPoint(x: ox + x * u, y: oy + y * u)
        }
        func line(_ a: CGPoint, _ b: CGPoint) {
            path.move(to: a); path.addLine(to: b)
        }

        switch kind {
        case .dashboard:
            line(pt(2, 4), pt(14, 4))
            line(pt(2, 8), pt(14, 8))
            line(pt(2, 12), pt(9, 12))

        case .messages:
            path.addRect(CGRect(x: ox + 2 * u, y: oy + 3 * u, width: 12 * u, height: 8 * u))
            path.move(to: pt(5, 11))
            path.addLine(to: pt(5, 14))
            path.addLine(to: pt(8, 11))

        case .wallet:
            path.addRect(CGRect(x: ox + 2 * u, y: oy + 4 * u, width: 12 * u, height: 9 * u))
            line(pt(2, 8), pt(14, 8))

        case .identity:
            path.addEllipse(in: CGRect(x: ox + 6 * u, y: oy + 3 * u, width: 4 * u, height: 4 * u))
            line(pt(2, 13), pt(14, 13))
            line(pt(8, 7), pt(8, 13))

        case .social:
            path.addEllipse(in: CGRect(x: ox + 2 * u, y: oy + 4 * u, width: 8 * u, height: 8 * u))
            path.addEllipse(in: CGRect(x: ox + 6 * u, y: oy + 4 * u, width: 8 * u, height: 8 * u))

        case .governance:
            line(pt(2, 4), pt(14, 4))
            line(pt(2, 13), pt(14, 13))
            line(pt(4, 4), pt(4, 13))
            line(pt(8, 4), pt(8, 13))
            line(pt(12, 4), pt(12, 13))

        case .ai:
            path.move(to: pt(8, 3))
            path.addLine(to: pt(14, 13))
            path.addLine(to: pt(2, 13))
            path.closeSubpath()
            line(pt(5, 9), pt(11, 9))

        case .network:
            line(pt(8, 3), pt(3, 12))
            line(pt(8, 3), pt(13, 12))
            line(pt(3, 12), pt(13, 12))
            path.addEllipse(in: CGRect(x: ox + 7 * u, y: oy + 2 * u, width: 2 * u, height: 2 * u))
            path.addEllipse(in: CGRect(x: ox + 2 * u, y: oy + 11 * u, width: 2 * u, height: 2 * u))
            path.addEllipse(in: CGRect(x: ox + 12 * u, y: oy + 11 * u, width: 2 * u, height: 2 * u))

        case .files:
            path.move(to: pt(3, 2))
            path.addLine(to: pt(10, 2))
            path.addLine(to: pt(13, 5))
            path.addLine(to: pt(13, 14))
            path.addLine(to: pt(3, 14))
            path.closeSubpath()
            path.move(to: pt(10, 2))
            path.addLine(to: pt(10, 5))
            path.addLine(to: pt(13, 5))

        case .marketplace:
            line(pt(2, 5), pt(14, 5))
            path.move(to: pt(3, 5))
            path.addLine(to: pt(3, 13))
            path.addLine(to: pt(13, 13))
            path.addLine(to: pt(13, 5))
            path.move(to: pt(6, 5))
            path.addLine(to: pt(6, 3))
            path.addLine(to: pt(10, 3))
            path.addLine(to: pt(10, 5))

        case .settings:
            line(pt(2, 5), pt(14, 5))
            path.addEllipse(in: CGRect(x: ox + 9 * u, y: oy + 4 * u, width: 2 * u, height: 2 * u))
            line(pt(2, 11), pt(14, 11))
            path.addEllipse(in: CGRect(x: ox + 5 * u, y: oy + 10 * u, width: 2 * u, height: 2 * u))
        }

        return path
    }
}

struct NousIcon: View {
    let kind: NousIconKind
    var size: CGFloat = 16
    var lineWidth: CGFloat = 1

    var body: some View {
        NousIconShape(kind: kind)
            .stroke(style: StrokeStyle(lineWidth: lineWidth, lineCap: .round, lineJoin: .round))
            .frame(width: size, height: size)
            .accessibilityHidden(true)
    }
}

#Preview {
    let kinds: [NousIconKind] = [
        .dashboard, .messages, .wallet, .identity, .social,
        .governance, .ai, .network, .files, .marketplace, .settings,
    ]
    return ScrollView {
        LazyVGrid(columns: [GridItem(.adaptive(minimum: 64))], spacing: 24) {
            ForEach(kinds, id: \.self) { kind in
                VStack(spacing: 8) {
                    NousIcon(kind: kind, size: 24)
                        .foregroundStyle(NousTheme.ivory)
                    MetaLabel(text: String(describing: kind))
                }
            }
        }
        .padding(32)
    }
    .background(NousTheme.background)
}
