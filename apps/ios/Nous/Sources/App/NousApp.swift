import SwiftUI
import UIKit

public struct NousAppView: View {
    @State private var selectedTab = 0
    @State private var store = NousStore()

    public init() {
        Self.configureChrome()
    }

    public var body: some View {
        TabView(selection: $selectedTab) {
            DashboardView()
                .tabItem { tabLabel("Home", kind: .dashboard, selected: selectedTab == 0) }
                .tag(0)

            SocialView()
                .tabItem { tabLabel("Social", kind: .social, selected: selectedTab == 1) }
                .tag(1)

            MessagesView()
                .tabItem { tabLabel("Messages", kind: .messages, selected: selectedTab == 2) }
                .tag(2)

            GovernanceView()
                .tabItem { tabLabel("Govern", kind: .governance, selected: selectedTab == 3) }
                .tag(3)

            WalletView()
                .tabItem { tabLabel("Wallet", kind: .wallet, selected: selectedTab == 4) }
                .tag(4)

            IdentityView()
                .tabItem { tabLabel("Identity", kind: .identity, selected: selectedTab == 5) }
                .tag(5)
        }
        .tint(NousTheme.accent)
        .preferredColorScheme(.dark)
        .environment(store)
    }

    @ViewBuilder
    private func tabLabel(_ title: String, kind: NousIconKind, selected: Bool) -> some View {
        // The tab-bar is rendered by UIKit under the hood, so this Label only
        // controls the text glyph; actual icon stroke comes from a UIImage
        // generated below in `configureChrome()`. The Image here is a placeholder
        // that gets replaced via UITabBarItem when the tab bar appears.
        Label {
            Text(title)
        } icon: {
            NousIcon(kind: kind, size: 22)
        }
    }

    /// Configure UIKit-backed tab bar and nav bar so they read `--ink` with a
    /// 1pt `--rule` hairline above. No translucent material — the editorial
    /// language is flat ink, not glass.
    private static func configureChrome() {
        let ink = UIColor(NousTheme.ink)
        let rule = UIColor(NousTheme.rule)
        let ivory = UIColor(NousTheme.ivory)
        let stone = UIColor(NousTheme.stone)
        let oxblood = UIColor(NousTheme.oxblood)

        // Tab bar
        let tabAppearance = UITabBarAppearance()
        tabAppearance.configureWithOpaqueBackground()
        tabAppearance.backgroundColor = ink
        tabAppearance.shadowColor = rule
        tabAppearance.shadowImage = UIImage()  // no system shadow — we use shadowColor as a hairline
        for item in [
            tabAppearance.stackedLayoutAppearance,
            tabAppearance.inlineLayoutAppearance,
            tabAppearance.compactInlineLayoutAppearance,
        ] {
            item.normal.iconColor = stone
            item.normal.titleTextAttributes = [
                .foregroundColor: stone,
                .font: UIFont.systemFont(ofSize: 10, weight: .regular),
                .kern: 0.5,
            ]
            item.selected.iconColor = oxblood
            item.selected.titleTextAttributes = [
                .foregroundColor: oxblood,
                .font: UIFont.systemFont(ofSize: 10, weight: .regular),
                .kern: 0.5,
            ]
        }
        UITabBar.appearance().standardAppearance = tabAppearance
        UITabBar.appearance().scrollEdgeAppearance = tabAppearance

        // Nav bar
        let navAppearance = UINavigationBarAppearance()
        navAppearance.configureWithOpaqueBackground()
        navAppearance.backgroundColor = ink
        navAppearance.shadowColor = rule
        navAppearance.titleTextAttributes = [
            .foregroundColor: ivory,
        ]
        navAppearance.largeTitleTextAttributes = [
            .foregroundColor: ivory,
        ]
        UINavigationBar.appearance().standardAppearance = navAppearance
        UINavigationBar.appearance().scrollEdgeAppearance = navAppearance
        UINavigationBar.appearance().compactAppearance = navAppearance
    }
}
