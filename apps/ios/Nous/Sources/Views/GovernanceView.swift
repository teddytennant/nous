import SwiftUI

/// Governance — token-level pass. The original two-section structure (DAOs,
/// Proposals) is preserved; bordered cards are replaced with editorial rows
/// bounded by hairlines.
struct GovernanceView: View {
    @Environment(NousStore.self) private var store

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: NousTheme.s12) {
                header

                if !store.daos.isEmpty {
                    daosSection
                }

                proposalsSection
            }
            .padding(.horizontal, NousTheme.s6)
            .padding(.vertical, NousTheme.s8)
        }
        .background(NousTheme.background)
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: NousTheme.s2) {
            Text("Governance")
                .font(NousTheme.display)
                .foregroundColor(NousTheme.ivory)
                .tracking(-1)
            MetaLabel(text: "Quadratic voting · proposals")
        }
    }

    private var daosSection: some View {
        VStack(alignment: .leading, spacing: NousTheme.s4) {
            MetaLabel(text: "DAOs")
            VStack(spacing: 0) {
                Hairline()
                ForEach(store.daos) { dao in
                    daoRow(dao)
                }
            }
        }
    }

    private func daoRow(_ dao: DaoResponse) -> some View {
        VStack(spacing: 0) {
            VStack(alignment: .leading, spacing: NousTheme.s2) {
                HStack(alignment: .firstTextBaseline) {
                    Text(dao.name)
                        .font(NousTheme.headlineMedium)
                        .foregroundColor(NousTheme.ivory)
                    Spacer()
                    Text("\(dao.memberCount)")
                        .font(NousTheme.mono)
                        .foregroundColor(NousTheme.ivory)
                    MetaLabel(text: dao.memberCount == 1 ? "Member" : "Members")
                }
                Text(dao.description)
                    .font(NousTheme.body)
                    .foregroundColor(NousTheme.ivoryDim)
                    .lineLimit(2)
                    .fixedSize(horizontal: false, vertical: true)
            }
            .padding(.vertical, NousTheme.s4)
            Hairline()
        }
    }

    private var proposalsSection: some View {
        VStack(alignment: .leading, spacing: NousTheme.s4) {
            MetaLabel(text: "Proposals")
            if store.proposals.isEmpty {
                Text("No active proposals.")
                    .font(NousTheme.body)
                    .foregroundColor(NousTheme.ivoryDim)
                    .padding(.vertical, NousTheme.s4)
            } else {
                VStack(spacing: 0) {
                    Hairline()
                    ForEach(store.proposals) { proposal in
                        proposalRow(proposal)
                    }
                }
            }
        }
    }

    private func proposalRow(_ proposal: ProposalResponse) -> some View {
        VStack(spacing: 0) {
            VStack(alignment: .leading, spacing: NousTheme.s2) {
                HStack(alignment: .firstTextBaseline) {
                    Text(proposal.title)
                        .font(NousTheme.headlineMedium)
                        .foregroundColor(NousTheme.ivory)
                    Spacer()
                    MetaLabel(
                        text: proposal.status,
                        active: proposal.status.lowercased() == "active"
                    )
                }
                Text(proposal.description)
                    .font(NousTheme.body)
                    .foregroundColor(NousTheme.ivoryDim)
                    .lineLimit(2)
                    .fixedSize(horizontal: false, vertical: true)
                HStack(spacing: NousTheme.s8) {
                    KeyValue(key: "Quorum", value: "\(Int(proposal.quorum * 100))%")
                    KeyValue(key: "Threshold", value: "\(Int(proposal.threshold * 100))%")
                    Spacer()
                }
                .padding(.top, NousTheme.s1)
            }
            .padding(.vertical, NousTheme.s4)
            Hairline()
        }
    }
}
