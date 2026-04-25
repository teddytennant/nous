import SwiftUI

/// 1pt hairline rule. Replaces every `Divider`/`stroke` border in the app.
/// Editorial language: panels are bounded by type, whitespace, or hairlines.
struct Hairline: View {
    enum Axis { case horizontal, vertical }

    var axis: Axis = .horizontal
    var color: Color = NousTheme.rule

    var body: some View {
        Rectangle()
            .fill(color)
            .frame(
                maxWidth: axis == .horizontal ? .infinity : 1,
                maxHeight: axis == .horizontal ? 1 : .infinity
            )
            .frame(
                width: axis == .vertical ? 1 : nil,
                height: axis == .horizontal ? 1 : nil
            )
            .accessibilityHidden(true)
    }
}

#Preview {
    VStack(spacing: 24) {
        Text("above")
            .foregroundColor(NousTheme.ivory)
        Hairline()
        Text("below")
            .foregroundColor(NousTheme.ivory)
        HStack(spacing: 16) {
            Text("left").foregroundColor(NousTheme.ivory)
            Hairline(axis: .vertical).frame(height: 24)
            Text("right").foregroundColor(NousTheme.ivory)
        }
    }
    .padding(32)
    .frame(maxWidth: .infinity, maxHeight: .infinity)
    .background(NousTheme.background)
}
