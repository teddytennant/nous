import SwiftUI

/// Editorial section header: display-serif title, optional meta-caps subtitle,
/// hairline rule below. The body content slots in beneath.
struct EditorialSection<Content: View>: View {
    let title: String
    var subtitle: String? = nil
    var trailing: AnyView? = nil
    @ViewBuilder var content: () -> Content

    var body: some View {
        VStack(alignment: .leading, spacing: NousTheme.s4) {
            HStack(alignment: .firstTextBaseline) {
                VStack(alignment: .leading, spacing: NousTheme.s1) {
                    Text(title)
                        .font(NousTheme.headlineLarge)
                        .foregroundColor(NousTheme.ivory)
                        .tracking(-0.4)
                    if let subtitle, !subtitle.isEmpty {
                        MetaLabel(text: subtitle)
                    }
                }
                Spacer(minLength: NousTheme.s4)
                if let trailing { trailing }
            }
            Hairline()
            content()
        }
    }
}

#Preview {
    ScrollView {
        VStack(alignment: .leading, spacing: 48) {
            EditorialSection(title: "Identity", subtitle: "DID:key, self-sovereign") {
                KeyValueRow(key: "Method", value: "did:key")
                KeyValueRow(key: "Signing", value: "Ed25519")
                KeyValueRow(key: "Exchange", value: "X25519")
            }
            EditorialSection(title: "Network") {
                KeyValueRow(key: "Peers", value: "127")
                KeyValueRow(key: "Sync", value: "head 0x4a91…f0")
            }
        }
        .padding(32)
    }
    .background(NousTheme.background)
}
