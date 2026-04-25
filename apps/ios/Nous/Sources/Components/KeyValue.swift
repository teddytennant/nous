import SwiftUI

/// Key in meta caps (`--stone`), value in mono (`--ivory`). The default
/// stat-display primitive — used in dashboards, identity, wallet.
struct KeyValue: View {
    let key: String
    let value: String
    var alignment: HorizontalAlignment = .leading
    var valueColor: Color = NousTheme.ivory

    var body: some View {
        VStack(alignment: alignment, spacing: NousTheme.s1) {
            MetaLabel(text: key)
            Text(value)
                .font(NousTheme.mono)
                .foregroundColor(valueColor)
                .lineLimit(1)
                .truncationMode(.middle)
        }
        .frame(maxWidth: .infinity, alignment: alignment == .leading ? .leading : .trailing)
        .accessibilityElement(children: .combine)
    }
}

/// Inline horizontal variant — key on the left, value on the right.
struct KeyValueRow: View {
    let key: String
    let value: String
    var valueColor: Color = NousTheme.ivory

    var body: some View {
        HStack(alignment: .firstTextBaseline) {
            MetaLabel(text: key)
            Spacer()
            Text(value)
                .font(NousTheme.mono)
                .foregroundColor(valueColor)
                .lineLimit(1)
                .truncationMode(.middle)
        }
        .accessibilityElement(children: .combine)
    }
}

#Preview {
    VStack(alignment: .leading, spacing: 24) {
        KeyValue(key: "Peers", value: "127")
        KeyValue(key: "Sync", value: "head 0x4a91…f0", valueColor: NousTheme.sage)
        Hairline()
        KeyValueRow(key: "Block Height", value: "2,481,902")
        KeyValueRow(key: "Reachability", value: "direct", valueColor: NousTheme.sage)
    }
    .padding(32)
    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
    .background(NousTheme.background)
}
