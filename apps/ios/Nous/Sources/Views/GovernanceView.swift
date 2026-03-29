import SwiftUI

struct GovernanceView: View {
    @Environment(NousStore.self) private var store

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

                if !store.daos.isEmpty {
                    VStack(alignment: .leading, spacing: 12) {
                        Text("DAOS")
                            .font(NousTheme.label)
                            .foregroundColor(NousTheme.textMuted)
                            .tracking(0.8)

                        ForEach(store.daos) { dao in
                            VStack(alignment: .leading, spacing: 6) {
                                HStack {
                                    Text(dao.name)
                                        .font(NousTheme.bodySmall)
                                        .foregroundColor(NousTheme.text)
                                    Spacer()
                                    Text("\(dao.memberCount) member\(dao.memberCount == 1 ? "" : "s")")
                                        .font(NousTheme.label)
                                        .foregroundColor(NousTheme.accent)
                                }
                                Text(dao.description)
                                    .font(.system(size: 12))
                                    .foregroundColor(NousTheme.textMuted)
                                    .lineLimit(2)
                            }
                            .padding(16)
                            .background(NousTheme.surface)
                            .overlay(
                                RoundedRectangle(cornerRadius: NousTheme.radiusSM)
                                    .stroke(NousTheme.border, lineWidth: 1)
                            )
                            .clipShape(RoundedRectangle(cornerRadius: NousTheme.radiusSM))
                        }
                    }
                }

                VStack(alignment: .leading, spacing: 12) {
                    Text("PROPOSALS")
                        .font(NousTheme.label)
                        .foregroundColor(NousTheme.textMuted)
                        .tracking(0.8)

                    if store.proposals.isEmpty {
                        Text("No active proposals.")
                            .font(NousTheme.bodySmall)
                            .foregroundColor(NousTheme.textMuted)
                            .padding(.top, 8)
                    } else {
                        ForEach(store.proposals) { proposal in
                            VStack(alignment: .leading, spacing: 6) {
                                HStack {
                                    Text(proposal.title)
                                        .font(NousTheme.bodySmall)
                                        .foregroundColor(NousTheme.text)
                                    Spacer()
                                    Text(proposal.status.uppercased())
                                        .font(.system(size: 10, weight: .regular))
                                        .tracking(0.6)
                                        .foregroundColor(
                                            proposal.status.lowercased() == "active"
                                                ? NousTheme.accent
                                                : NousTheme.textMuted
                                        )
                                }
                                Text(proposal.description)
                                    .font(.system(size: 12))
                                    .foregroundColor(NousTheme.textMuted)
                                    .lineLimit(2)
                                Text("Quorum: \(Int(proposal.quorum * 100))% | Threshold: \(Int(proposal.threshold * 100))%")
                                    .font(NousTheme.label)
                                    .foregroundColor(NousTheme.textMuted)
                            }
                            .padding(16)
                            .background(NousTheme.surface)
                            .overlay(
                                RoundedRectangle(cornerRadius: NousTheme.radiusSM)
                                    .stroke(NousTheme.border, lineWidth: 1)
                            )
                            .clipShape(RoundedRectangle(cornerRadius: NousTheme.radiusSM))
                        }
                    }
                }
            }
            .padding(NousTheme.spacingLG)
        }
        .background(NousTheme.background)
    }
}
