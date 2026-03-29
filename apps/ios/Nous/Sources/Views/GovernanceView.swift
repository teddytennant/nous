import SwiftUI

struct GovernanceView: View {
    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: NousTheme.spacingXL) {
                VStack(alignment: .leading, spacing: NousTheme.spacingXS) {
                    Text("Governance")
                        .font(NousTheme.headlineLarge)
                        .foregroundColor(NousTheme.text)
                    Text("Quadratic voting & proposals")
                        .font(NousTheme.bodySmall)
                        .foregroundColor(NousTheme.textMuted)
                }

                Text("No active proposals.")
                    .font(NousTheme.bodySmall)
                    .foregroundColor(NousTheme.textMuted)
                    .padding(.top, NousTheme.spacingXL)
            }
            .padding(NousTheme.spacingLG)
        }
        .background(NousTheme.background)
    }
}
