import SwiftUI

struct MessagesView: View {
    var body: some View {
        VStack(alignment: .leading, spacing: NousTheme.spacingXL) {
            VStack(alignment: .leading, spacing: NousTheme.spacingXS) {
                Text("Messages")
                    .font(NousTheme.headlineLarge)
                    .foregroundColor(NousTheme.text)
                Text("End-to-end encrypted via Double Ratchet")
                    .font(NousTheme.bodySmall)
                    .foregroundColor(NousTheme.textMuted)
            }
            .padding(.horizontal, NousTheme.spacingLG)
            .padding(.top, NousTheme.spacingLG)

            Spacer()

            Text("No conversations yet.")
                .font(NousTheme.bodySmall)
                .foregroundColor(NousTheme.textMuted)
                .frame(maxWidth: .infinity)

            Spacer()
        }
        .background(NousTheme.background)
    }
}
