import SwiftUI

/// Uppercase tracked label. Used everywhere we need a column heading,
/// section caps, or a status sigil. `--stone` at rest, `--oxblood` when active.
struct MetaLabel: View {
    let text: String
    var active: Bool = false

    var body: some View {
        Text(text.uppercased())
            .font(NousTheme.label)
            .tracking(NousTheme.metaTracking)
            .foregroundColor(active ? NousTheme.oxblood : NousTheme.stone)
            .accessibilityLabel(text)
    }
}

#Preview {
    VStack(alignment: .leading, spacing: 16) {
        MetaLabel(text: "Network")
        MetaLabel(text: "Active", active: true)
        MetaLabel(text: "Block Height")
    }
    .padding(32)
    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
    .background(NousTheme.background)
}
