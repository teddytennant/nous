import SwiftUI

struct ModuleInfo: Identifiable {
    let id = UUID()
    let name: String
    let status: String
}

struct DashboardView: View {
    @Environment(NousStore.self) private var store

    var modules: [ModuleInfo] {
        let status = store.connected ? "active" : "offline"
        return [
            .init(name: "Identity", status: status),
            .init(name: "Messaging", status: status),
            .init(name: "Governance", status: status),
            .init(name: "Social", status: status),
            .init(name: "Payments", status: status),
            .init(name: "Storage", status: status),
            .init(name: "AI", status: "standby"),
            .init(name: "Browser", status: "standby"),
        ]
    }

    var uptimeDisplay: String {
        let ms = store.uptimeMs
        if ms < 60_000 { return "\(ms / 1000)s" }
        if ms < 3_600_000 { return "\(ms / 60_000)m" }
        return "\(ms / 3_600_000)h"
    }

    var didDisplay: String {
        let did = store.did
        if did.count > 24 { return String(did.prefix(20)) + "..." }
        return did
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: NousTheme.spacingXL) {
                VStack(alignment: .leading, spacing: NousTheme.spacingXS) {
                    Text("Dashboard")
                        .font(NousTheme.headlineLarge)
                        .foregroundColor(NousTheme.text)
                    Text(store.connected ? "Connected to API" : "Offline")
                        .font(NousTheme.bodySmall)
                        .foregroundColor(store.connected ? NousTheme.success : NousTheme.error)
                }

                LazyVGrid(columns: [
                    GridItem(.flexible()),
                    GridItem(.flexible()),
                ], spacing: 12) {
                    StatCardView(label: "Identity", value: didDisplay)
                    StatCardView(label: "Peers", value: "0")
                    StatCardView(label: "Uptime", value: uptimeDisplay)
                    StatCardView(label: "Version", value: store.version)
                }

                VStack(alignment: .leading, spacing: 12) {
                    Text("PROTOCOL MODULES")
                        .font(NousTheme.label)
                        .foregroundColor(NousTheme.textMuted)
                        .tracking(0.8)

                    LazyVGrid(columns: [
                        GridItem(.flexible()),
                        GridItem(.flexible()),
                    ], spacing: 8) {
                        ForEach(modules) { module in
                            ModuleCardView(name: module.name, status: module.status)
                        }
                    }
                }
            }
            .padding(NousTheme.spacingLG)
        }
        .background(NousTheme.background)
        .task {
            await store.refresh()
        }
    }
}

struct StatCardView: View {
    let label: String
    let value: String

    var body: some View {
        VStack(alignment: .leading, spacing: NousTheme.spacingSM) {
            Text(label.uppercased())
                .font(NousTheme.label)
                .foregroundColor(NousTheme.textMuted)
                .tracking(0.8)
            Text(value)
                .font(NousTheme.mono)
                .foregroundColor(NousTheme.text)
                .lineLimit(1)
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

struct ModuleCardView: View {
    let name: String
    let status: String

    var body: some View {
        HStack {
            Text(name)
                .font(NousTheme.bodySmall)
                .foregroundColor(NousTheme.text)
            Spacer()
            Text(status.uppercased())
                .font(.system(size: 10, weight: .regular))
                .tracking(0.6)
                .foregroundColor(status == "active" ? NousTheme.success : NousTheme.textMuted)
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 14)
        .background(NousTheme.surface)
        .overlay(
            RoundedRectangle(cornerRadius: NousTheme.radiusSM)
                .stroke(NousTheme.border, lineWidth: 1)
        )
        .clipShape(RoundedRectangle(cornerRadius: NousTheme.radiusSM))
    }
}
