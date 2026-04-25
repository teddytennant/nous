import SwiftUI
import UIKit

/// Identity — DID display in mono on its own line, with a small oxblood copy
/// mark beside it. KeyValue rows for verification methods, services,
/// controllers. `EditorialSection` per logical group.
struct IdentityView: View {
    @Environment(NousStore.self) private var store
    @State private var displayName = ""
    @State private var creating = false
    @State private var copyConfirmed = false

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: NousTheme.s12) {
                header

                if store.loading {
                    loadingState
                } else if let id = store.identity {
                    identityBody(id: id)
                } else if !store.connected {
                    EmptyState(
                        title: "API offline",
                        subtitle: "Unable to reach the Nous node. Identity will load when the API is available."
                    )
                } else {
                    createIdentityForm
                }
            }
            .padding(.horizontal, NousTheme.s6)
            .padding(.vertical, NousTheme.s8)
        }
        .background(NousTheme.background)
    }

    // MARK: - Header

    private var header: some View {
        VStack(alignment: .leading, spacing: NousTheme.s2) {
            Text("Identity")
                .font(NousTheme.display)
                .foregroundColor(NousTheme.ivory)
                .tracking(-1)
            MetaLabel(text: "DID:key, self-sovereign")
        }
    }

    // MARK: - Loaded body

    @ViewBuilder
    private func identityBody(id: IdentityResponse) -> some View {
        VStack(alignment: .leading, spacing: NousTheme.s12) {
            EditorialSection(title: id.displayName?.isEmpty == false ? id.displayName! : "Your DID",
                             subtitle: id.displayName?.isEmpty == false ? "Decentralised Identifier" : nil) {
                didBlock(id: id)
            }

            EditorialSection(title: "Verification Methods", subtitle: "Cryptographic keys") {
                VStack(spacing: 0) {
                    Hairline()
                    keyRow(type: id.signingKeyType, purpose: "Signing")
                    keyRow(type: id.exchangeKeyType, purpose: "Key Exchange")
                }
            }

            EditorialSection(title: "Services", subtitle: "Endpoints") {
                Text("None registered.")
                    .font(NousTheme.body)
                    .foregroundColor(NousTheme.ivoryDim)
                    .padding(.vertical, NousTheme.s2)
            }

            EditorialSection(title: "Controllers", subtitle: "Authoritative parties") {
                VStack(spacing: 0) {
                    Hairline()
                    HStack {
                        Text("Self")
                            .font(NousTheme.body)
                            .foregroundColor(NousTheme.ivory)
                        Spacer()
                        MetaLabel(text: "Sole Controller", active: true)
                    }
                    .padding(.vertical, NousTheme.s3)
                    Hairline()
                }
            }
        }
    }

    private func didBlock(id: IdentityResponse) -> some View {
        VStack(alignment: .leading, spacing: NousTheme.s2) {
            Text(id.did)
                .font(NousTheme.mono)
                .foregroundColor(NousTheme.ivory)
                .textSelection(.enabled)
                .fixedSize(horizontal: false, vertical: true)
            HStack(spacing: NousTheme.s2) {
                Button(action: { copyDID(id.did) }) {
                    HStack(spacing: NousTheme.s1) {
                        OxbloodMark(style: copyConfirmed ? .disc : .copy, size: 10)
                        Text(copyConfirmed ? "Copied" : "Copy")
                            .font(NousTheme.label)
                            .tracking(NousTheme.metaTracking)
                            .foregroundColor(NousTheme.oxblood)
                    }
                    .padding(.vertical, NousTheme.s1)
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
            }
            .padding(.top, NousTheme.s1)
        }
    }

    private func keyRow(type: String, purpose: String) -> some View {
        VStack(spacing: 0) {
            HStack(alignment: .firstTextBaseline) {
                VStack(alignment: .leading, spacing: NousTheme.s1) {
                    MetaLabel(text: purpose)
                    Text(type)
                        .font(NousTheme.mono)
                        .foregroundColor(NousTheme.ivory)
                }
                Spacer()
            }
            .padding(.vertical, NousTheme.s3)
            Hairline()
        }
    }

    // MARK: - Loading

    private var loadingState: some View {
        VStack(alignment: .leading, spacing: NousTheme.s4) {
            ProgressView()
                .tint(NousTheme.oxblood)
            MetaLabel(text: "Connecting to node…")
        }
        .padding(.vertical, NousTheme.s8)
    }

    // MARK: - Creation form

    private var createIdentityForm: some View {
        EditorialSection(title: "Generate Identity", subtitle: "New DID:key") {
            VStack(alignment: .leading, spacing: NousTheme.s4) {
                Text("Create a fresh decentralised identifier rooted in keys held only on this device.")
                    .font(NousTheme.body)
                    .foregroundColor(NousTheme.ivoryDim)
                    .fixedSize(horizontal: false, vertical: true)

                VStack(alignment: .leading, spacing: NousTheme.s2) {
                    MetaLabel(text: "Display Name (Optional)")
                    TextField("", text: $displayName, prompt: Text("Anonymous").foregroundColor(NousTheme.stone))
                        .font(NousTheme.body)
                        .foregroundColor(NousTheme.ivory)
                        .padding(.vertical, NousTheme.s2)
                        .textInputAutocapitalization(.words)
                    Hairline()
                }

                Button(action: { Task { await createIdentity() } }) {
                    HStack(spacing: NousTheme.s2) {
                        Text(creating ? "Generating…" : "Generate DID")
                            .font(NousTheme.body)
                            .foregroundColor(NousTheme.oxblood)
                        OxbloodMark(style: .arrow, size: 10)
                    }
                    .padding(.vertical, NousTheme.s2)
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .disabled(creating)
            }
        }
    }

    // MARK: - Actions

    private func copyDID(_ did: String) {
        UIPasteboard.general.string = did
        withAnimation(NousTheme.easing) { copyConfirmed = true }
        Task { @MainActor in
            try? await Task.sleep(nanoseconds: 1_400_000_000)
            withAnimation(NousTheme.easing) { copyConfirmed = false }
        }
    }

    private func createIdentity() async {
        creating = true
        let name = displayName.trimmingCharacters(in: .whitespacesAndNewlines)
        do {
            let id = try await NousAPI.shared.createIdentity(displayName: name.isEmpty ? nil : name)
            store.identity = id
            store.did = id.did
            displayName = ""
        } catch {
            // ignore — the empty/error state is handled by the parent body
        }
        creating = false
    }
}
