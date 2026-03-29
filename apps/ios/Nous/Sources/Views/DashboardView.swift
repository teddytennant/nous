import SwiftUI

struct ModuleInfo: Identifiable {
    let id = UUID()
    let name: String
    let status: String
}

struct DashboardView: View {
    let modules: [ModuleInfo] = [
        .init(name: "Identity", status: "active"),
        .init(name: "Messaging", status: "active"),
        .init(name: "Governance", status: "active"),
        .init(name: "Social", status: "active"),
        .init(name: "Payments", status: "standby"),
        .init(name: "Storage", status: "active"),
        .init(name: "AI", status: "standby"),
        .init(name: "Browser", status: "standby"),
    ]

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: NousTheme.spacingXL) {
                // Header
                VStack(alignment: .leading, spacing: NousTheme.spacingXS) {
                    Text("Dashboard")
                        .font(NousTheme.headlineLarge)
                        .foregroundColor(NousTheme.text)
                    Text("Sovereign protocol overview")
                        .font(NousTheme.bodySmall)
                        .foregroundColor(NousTheme.textMuted)
                }

                // Stats Grid
                LazyVGrid(columns: [
                    GridItem(.flexible()),
                    GridItem(.flexible()),
                ], spacing: 12) {
                    StatCardView(label: "Identity", value: "did:key:z6Mk...2doK")
                    StatCardView(label: "Peers", value: "0")
                    StatCardView(label: "Uptime", value: "0s")
                    StatCardView(label: "Version", value: "0.1.0")
                }

                // Modules
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
