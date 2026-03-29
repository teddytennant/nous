import SwiftUI

struct BalanceInfo: Identifiable {
    let id = UUID()
    let token: String
    let amount: String
    let usdValue: String?
}

struct WalletView: View {
    let balances: [BalanceInfo] = [
        .init(token: "ETH", amount: "0.000", usdValue: "$0.00"),
        .init(token: "NOUS", amount: "0.000", usdValue: nil),
        .init(token: "USDC", amount: "0.000", usdValue: "$0.00"),
    ]

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: NousTheme.spacingXL) {
                VStack(alignment: .leading, spacing: NousTheme.spacingXS) {
                    Text("Wallet")
                        .font(NousTheme.headlineLarge)
                        .foregroundColor(NousTheme.text)
                    Text("Multi-chain, escrow-backed")
                        .font(NousTheme.bodySmall)
                        .foregroundColor(NousTheme.textMuted)
                }

                HStack(spacing: 12) {
                    ForEach(balances) { balance in
                        VStack(alignment: .leading, spacing: NousTheme.spacingSM) {
                            Text(balance.token)
                                .font(NousTheme.label)
                                .foregroundColor(NousTheme.textMuted)
                                .tracking(0.8)
                            Text(balance.amount)
                                .font(NousTheme.monoLarge)
                                .foregroundColor(NousTheme.text)
                            if let usd = balance.usdValue {
                                Text(usd)
                                    .font(NousTheme.bodySmall)
                                    .foregroundColor(NousTheme.textMuted)
                            }
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(20)
                        .background(NousTheme.surface)
                        .overlay(
                            RoundedRectangle(cornerRadius: NousTheme.radiusMD)
                                .stroke(NousTheme.border, lineWidth: 1)
                        )
                        .clipShape(RoundedRectangle(cornerRadius: NousTheme.radiusMD))
                    }
                }

                HStack(spacing: 12) {
                    NousOutlinedButton(title: "Send") {}
                    NousOutlinedButton(title: "Receive") {}
                    NousOutlinedButton(title: "Swap") {}
                }
            }
            .padding(NousTheme.spacingLG)
        }
        .background(NousTheme.background)
    }
}

struct NousOutlinedButton: View {
    let title: String
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Text(title)
                .font(NousTheme.bodySmall)
                .foregroundColor(NousTheme.textSecondary)
                .padding(.horizontal, 20)
                .padding(.vertical, 8)
                .overlay(
                    RoundedRectangle(cornerRadius: NousTheme.radiusSM)
                        .stroke(NousTheme.border, lineWidth: 1)
                )
        }
    }
}
