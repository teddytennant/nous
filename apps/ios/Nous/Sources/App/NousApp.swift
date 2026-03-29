import SwiftUI

public struct NousAppView: View {
    @State private var selectedTab = 0

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

            WalletView()
                .tabItem {
                    Label("Wallet", systemImage: "creditcard")
                }
                .tag(3)

            IdentityView()
                .tabItem {
                    Label("Identity", systemImage: "person.crop.circle")
                }
                .tag(4)
        }
        .tint(NousTheme.accent)
        .preferredColorScheme(.dark)
    }
}
