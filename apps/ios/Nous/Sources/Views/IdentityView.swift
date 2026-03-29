import SwiftUI

struct IdentityView: View {
    @Environment(NousStore.self) private var store
    @State private var displayName = ""
    @State private var creating = false

    private var keys: [(type: String, purpose: String)] {
        guard let id = store.identity else { return [] }
        return [
            (id.signingKeyType, "Signing"),
            (id.exchangeKeyType, "Key Exchange"),
        ]
    }

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

                if store.loading {
                    // Loading state
                    VStack(spacing: NousTheme.spacingMD) {
                        Spacer().frame(height: 40)
                        ProgressView()
                            .tint(NousTheme.accent)
                        Text("Connecting to node...")
                            .font(NousTheme.bodySmall)
                            .foregroundColor(NousTheme.textMuted)
                        Spacer().frame(height: 40)
                    }
                    .frame(maxWidth: .infinity)
                } else if let id = store.identity {
                    // Identity loaded from API
                    identityCard(id: id)
                } else if !store.connected {
                    // Offline state
                    VStack(spacing: NousTheme.spacingMD) {
                        Spacer().frame(height: 40)
                        Text("API Offline")
                            .font(NousTheme.bodySmall)
                            .foregroundColor(NousTheme.error)
                        Text("Unable to reach the Nous node. Identity will load when the API is available.")
                            .font(NousTheme.label)
                            .foregroundColor(NousTheme.textMuted)
                            .multilineTextAlignment(.center)
                        Spacer().frame(height: 40)
                    }
                    .frame(maxWidth: .infinity)
                    .padding(.horizontal, NousTheme.spacingLG)
                } else {
                    // Connected but no identity yet — show creation form
                    createIdentityCard
                }
            }
            .padding(NousTheme.spacingLG)
        }
        .background(NousTheme.background)
    }

    // MARK: - Identity Card

    @ViewBuilder
    private func identityCard(id: IdentityResponse) -> some View {
        VStack(alignment: .leading, spacing: NousTheme.spacingLG) {
            VStack(alignment: .leading, spacing: NousTheme.spacingSM) {
                Text("YOUR DID")
                    .font(NousTheme.label)
                    .foregroundColor(NousTheme.textMuted)
                    .tracking(0.8)
                Text(id.did)
                    .font(NousTheme.mono)
                    .foregroundColor(NousTheme.accent)
                    .textSelection(.enabled)
                if let name = id.displayName, !name.isEmpty {
                    Text(name)
                        .font(NousTheme.bodySmall)
                        .foregroundColor(NousTheme.textSecondary)
                        .padding(.top, 2)
                }
            }

            ForEach(Array(keys.enumerated()), id: \.offset) { _, key in
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

    // MARK: - Create Identity Card

    private var createIdentityCard: some View {
        VStack(alignment: .leading, spacing: NousTheme.spacingLG) {
            VStack(alignment: .leading, spacing: NousTheme.spacingSM) {
                Text("GENERATE IDENTITY")
                    .font(NousTheme.label)
                    .foregroundColor(NousTheme.textMuted)
                    .tracking(0.8)
                Text("Create a new decentralized identity on the Nous network.")
                    .font(NousTheme.bodySmall)
                    .foregroundColor(NousTheme.textMuted)
            }

            TextField("Display name (optional)", text: $displayName)
                .font(NousTheme.body)
                .foregroundColor(NousTheme.text)
                .padding(14)
                .background(NousTheme.surface)
                .overlay(
                    Rectangle()
                        .stroke(NousTheme.border, lineWidth: 1)
                )
                .textInputAutocapitalization(.words)

            Button(action: {
                Task { await createIdentity() }
            }) {
                HStack(spacing: 8) {
                    if creating {
                        ProgressView()
                            .tint(.black)
                            .scaleEffect(0.8)
                    }
                    Text(creating ? "Generating..." : "Generate DID")
                        .font(NousTheme.bodySmall)
                        .foregroundColor(.black)
                }
                .frame(maxWidth: .infinity)
                .padding(.vertical, 12)
                .background(creating ? NousTheme.accent.opacity(0.5) : NousTheme.accent)
            }
            .cornerRadius(0)
            .disabled(creating)
        }
        .padding(NousTheme.spacingLG)
        .background(NousTheme.surface)
        .overlay(
            RoundedRectangle(cornerRadius: NousTheme.radiusMD)
                .stroke(NousTheme.border, lineWidth: 1)
        )
        .clipShape(RoundedRectangle(cornerRadius: NousTheme.radiusMD))
    }

    // MARK: - Actions

    private func createIdentity() async {
        creating = true
        let name = displayName.trimmingCharacters(in: .whitespacesAndNewlines)
        do {
            let id = try await NousAPI.shared.createIdentity(displayName: name.isEmpty ? nil : name)
            store.identity = id
            store.did = id.did
            displayName = ""
        } catch {
            // Failed to create identity
        }
        creating = false
    }
}
