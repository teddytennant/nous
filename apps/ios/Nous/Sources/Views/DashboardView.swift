import SwiftUI

/// Dashboard — editorial vertical stack. No card grid, no module tiles.
/// Display-serif greeting → hairline → KeyValue stack of network stats →
/// hairline → recent activity as an editorial list.
struct DashboardView: View {
    @Environment(NousStore.self) private var store

    private var greeting: String {
        let hour = Calendar.current.component(.hour, from: Date())
        switch hour {
        case 5..<12: return "Good morning."
        case 12..<18: return "Good afternoon."
        case 18..<23: return "Good evening."
        default: return "Welcome back."
        }
    }

    private var uptimeDisplay: String {
        let ms = store.uptimeMs
        if ms < 60_000 { return "\(ms / 1000)s" }
        if ms < 3_600_000 { return "\(ms / 60_000)m" }
        return "\(ms / 3_600_000)h"
    }

    private var didDisplay: String {
        let did = store.did
        guard did.count > 24 else { return did }
        return String(did.prefix(12)) + "…" + String(did.suffix(6))
    }

    private var reachability: String {
        store.connected ? "direct" : "offline"
    }

    private var reachabilityColor: Color {
        store.connected ? NousTheme.sage : NousTheme.clay
    }

    /// Recent activity — synthesised from store state. Each row is editorial,
    /// not a card. (Real activity feed plumbing is owned by other agents; this
    /// view shows what data the store currently exposes.)
    private var activity: [ActivityEntry] {
        var entries: [ActivityEntry] = []
        if store.connected {
            entries.append(.init(
                timestamp: "now",
                event: "Node reachable",
                identifier: "v" + store.version
            ))
        }
        if let id = store.identity {
            entries.append(.init(
                timestamp: "session",
                event: "Identity loaded",
                identifier: id.did
            ))
        }
        for tx in store.transactions.prefix(3) {
            entries.append(.init(
                timestamp: shortTimestamp(tx.timestamp),
                event: tx.fromDid == store.did ? "Sent \(tx.token.uppercased())" : "Received \(tx.token.uppercased())",
                identifier: tx.amount
            ))
        }
        for ev in store.feedEvents.prefix(2) {
            entries.append(.init(
                timestamp: shortTimestamp(ev.createdAt),
                event: "Post published",
                identifier: ev.id
            ))
        }
        return entries
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: NousTheme.s8) {
                header
                statsBlock
                activityBlock
            }
            .padding(.horizontal, NousTheme.s6)
            .padding(.vertical, NousTheme.s8)
        }
        .background(NousTheme.background)
        .task { await store.refresh() }
    }

    // MARK: - Header

    private var header: some View {
        VStack(alignment: .leading, spacing: NousTheme.s2) {
            Text(greeting)
                .font(NousTheme.display)
                .foregroundColor(NousTheme.ivory)
                .tracking(-1)
                .lineLimit(1)
                .minimumScaleFactor(0.6)
            HStack(spacing: NousTheme.s2) {
                OxbloodMark(style: store.connected ? .disc : .dot, size: 6)
                MetaLabel(
                    text: store.connected ? "Connected · \(store.version)" : "Offline",
                    active: store.connected
                )
            }
        }
    }

    // MARK: - Stats

    private var statsBlock: some View {
        VStack(alignment: .leading, spacing: NousTheme.s4) {
            Hairline()
            VStack(spacing: NousTheme.s3) {
                KeyValueRow(key: "Identity", value: didDisplay)
                KeyValueRow(
                    key: "Reachability",
                    value: reachability,
                    valueColor: reachabilityColor
                )
                KeyValueRow(key: "Uptime", value: uptimeDisplay)
                KeyValueRow(key: "Peers", value: "0")
                KeyValueRow(key: "Version", value: store.version)
            }
            Hairline()
        }
    }

    // MARK: - Activity

    private var activityBlock: some View {
        VStack(alignment: .leading, spacing: NousTheme.s4) {
            MetaLabel(text: "Recent Activity")
            if activity.isEmpty {
                Text("Nothing has happened yet.")
                    .font(NousTheme.body)
                    .foregroundColor(NousTheme.ivoryDim)
                    .padding(.vertical, NousTheme.s4)
            } else {
                VStack(spacing: 0) {
                    ForEach(Array(activity.enumerated()), id: \.offset) { idx, entry in
                        ActivityRow(entry: entry, isFirst: idx == 0)
                    }
                }
            }
        }
    }

    private func shortTimestamp(_ raw: String) -> String {
        if raw.count >= 10 { return String(raw.prefix(10)) }
        return raw
    }
}

private struct ActivityEntry {
    let timestamp: String
    let event: String
    let identifier: String?
}

private struct ActivityRow: View {
    let entry: ActivityEntry
    let isFirst: Bool

    var body: some View {
        VStack(spacing: 0) {
            if isFirst { Hairline() }
            HStack(alignment: .firstTextBaseline, spacing: NousTheme.s4) {
                MetaLabel(text: entry.timestamp)
                    .frame(width: 80, alignment: .leading)
                VStack(alignment: .leading, spacing: NousTheme.s1) {
                    Text(entry.event)
                        .font(NousTheme.body)
                        .foregroundColor(NousTheme.ivory)
                    if let id = entry.identifier, !id.isEmpty {
                        Text(id)
                            .font(NousTheme.mono)
                            .foregroundColor(NousTheme.stone)
                            .lineLimit(1)
                            .truncationMode(.middle)
                    }
                }
                Spacer(minLength: 0)
            }
            .padding(.vertical, NousTheme.s3)
            Hairline()
        }
    }
}
