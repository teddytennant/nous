import SwiftUI

public struct NousAppView: View {
    @State private var selectedTab = 0
    @State private var store = NousStore()

    public init() {}

    public var body: some View {
        TabView(selection: $selectedTab) {
            DashboardView()
                .tabItem {
                    Label("Home", systemImage: "square.grid.2x2")
                }
                .tag(0)

            SocialView()
                .tabItem {
                    Label("Social", systemImage: "bubble.left")
                }
                .tag(1)

            MessagesView()
                .tabItem {
                    Label("Messages", systemImage: "lock.shield")
                }
                .tag(2)

            GovernanceView()
                .tabItem {
                    Label("Govern", systemImage: "building.columns")
                }
                .tag(3)

            WalletView()
                .tabItem {
                    Label("Wallet", systemImage: "creditcard")
                }
                .tag(4)

            IdentityView()
                .tabItem {
                    Label("Identity", systemImage: "person.crop.circle")
                }
                .tag(5)
        }
        .tint(NousTheme.accent)
        .preferredColorScheme(.dark)
        .environment(store)
    }
}
