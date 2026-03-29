import SwiftUI

struct KeyDisplay: Identifiable {
    let id = UUID()
    let type: String
    let purpose: String
}

struct IdentityView: View {
    let did = "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK"
    let keys: [KeyDisplay] = [
        .init(type: "ed25519", purpose: "Signing"),
        .init(type: "x25519", purpose: "Key Exchange"),
    ]

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: NousTheme.spacingXL) {
                VStack(alignment: .leading, spacing: NousTheme.spacingXS) {
                    Text("Identity")
                        .font(NousTheme.headlineLarge)
                        .foregroundColor(NousTheme.text)
                    Text("Self-sovereign, DID:key")
                        .font(NousTheme.bodySmall)
                        .foregroundColor(NousTheme.textMuted)
                }

                VStack(alignment: .leading, spacing: NousTheme.spacingLG) {
                    VStack(alignment: .leading, spacing: NousTheme.spacingSM) {
                        Text("YOUR DID")
                            .font(NousTheme.label)
                            .foregroundColor(NousTheme.textMuted)
                            .tracking(0.8)
                        Text(did)
                            .font(NousTheme.mono)
                            .foregroundColor(NousTheme.accent)
                            .textSelection(.enabled)
                    }

                    ForEach(keys) { key in
                        Divider()
                            .background(NousTheme.border)
                        HStack {
                            Text(key.type)
                                .font(NousTheme.mono)
                                .foregroundColor(NousTheme.text)
                            Spacer()
                            Text(key.purpose)
                                .font(NousTheme.bodySmall)
                                .foregroundColor(NousTheme.textMuted)
                        }
                        .padding(.vertical, 4)
                    }
                }
                .padding(NousTheme.spacingLG)
                .background(NousTheme.surface)
                .overlay(
                    RoundedRectangle(cornerRadius: NousTheme.radiusMD)
                        .stroke(NousTheme.border, lineWidth: 1)
                )
                .clipShape(RoundedRectangle(cornerRadius: NousTheme.radiusMD))
            }
            .padding(NousTheme.spacingLG)
        }
        .background(NousTheme.background)
    }
}
