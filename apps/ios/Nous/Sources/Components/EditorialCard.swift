import SwiftUI

/// Not a card with chrome. A content block bounded by optional hairline rules
/// at top and/or bottom. Padding from `NousTheme` spacing tokens.
struct EditorialCard<Content: View>: View {
    var topRule: Bool = false
    var bottomRule: Bool = true
    var horizontalPadding: CGFloat = 0
    var verticalPadding: CGFloat = NousTheme.s4
    @ViewBuilder var content: () -> Content

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            if topRule { Hairline() }
            content()
                .padding(.horizontal, horizontalPadding)
                .padding(.vertical, verticalPadding)
                .frame(maxWidth: .infinity, alignment: .leading)
            if bottomRule { Hairline() }
        }
    }
}

#Preview {
    ScrollView {
        VStack(alignment: .leading, spacing: 0) {
            EditorialCard(topRule: true) {
                VStack(alignment: .leading, spacing: 8) {
                    Text("Block 2,481,902 sealed")
                        .font(NousTheme.body)
                        .foregroundColor(NousTheme.ivory)
                    MetaLabel(text: "12 minutes ago")
                }
            }
            EditorialCard {
                VStack(alignment: .leading, spacing: 8) {
                    Text("Peer joined network")
                        .font(NousTheme.body)
                        .foregroundColor(NousTheme.ivory)
                    Text("did:key:z6MkpTHR…J9q")
                        .font(NousTheme.mono)
                        .foregroundColor(NousTheme.stone)
                }
            }
        }
        .padding(.horizontal, 32)
    }
    .background(NousTheme.background)
}
