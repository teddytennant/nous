package com.nous.app.ui

import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.AccountBalanceWallet
import androidx.compose.material.icons.outlined.Chat
import androidx.compose.material.icons.outlined.Dashboard
import androidx.compose.material.icons.outlined.Fingerprint
import androidx.compose.material.icons.outlined.People
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.NavigationBarItemDefaults
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.navigation.NavDestination.Companion.hierarchy
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import androidx.lifecycle.viewmodel.compose.viewModel
import com.nous.app.data.NousViewModel
import com.nous.app.ui.screens.AIScreen
import com.nous.app.ui.screens.DashboardScreen
import com.nous.app.ui.screens.FilesScreen
import com.nous.app.ui.screens.GovernanceScreen
import com.nous.app.ui.screens.IdentityScreen
import com.nous.app.ui.screens.MarketplaceScreen
import com.nous.app.ui.screens.MessagesScreen
import com.nous.app.ui.screens.NetworkScreen
import com.nous.app.ui.screens.SettingsScreen
import com.nous.app.ui.screens.SocialScreen
import com.nous.app.ui.screens.WalletScreen

sealed class Screen(val route: String, val label: String, val icon: ImageVector) {
    data object Dashboard : Screen("dashboard", "Home", Icons.Outlined.Dashboard)
    data object Social : Screen("social", "Social", Icons.Outlined.People)
    data object Messages : Screen("messages", "Messages", Icons.Outlined.Chat)
    data object Wallet : Screen("wallet", "Wallet", Icons.Outlined.AccountBalanceWallet)
    data object Identity : Screen("identity", "Identity", Icons.Outlined.Fingerprint)
}

val screens = listOf(
    Screen.Dashboard,
    Screen.Social,
    Screen.Messages,
    Screen.Wallet,
    Screen.Identity,
)

@Composable
fun NousApp() {
    val navController = rememberNavController()
    val navBackStackEntry by navController.currentBackStackEntryAsState()
    val currentDestination = navBackStackEntry?.destination
    val sharedViewModel: NousViewModel = viewModel()

    Scaffold(
        containerColor = Color.Black,
        bottomBar = {
            NavigationBar(
                containerColor = MaterialTheme.colorScheme.surface,
                contentColor = MaterialTheme.colorScheme.onSurface,
            ) {
                screens.forEach { screen ->
                    val selected = currentDestination?.hierarchy?.any { it.route == screen.route } == true
                    NavigationBarItem(
                        selected = selected,
                        onClick = {
                            navController.navigate(screen.route) {
                                popUpTo(navController.graph.startDestinationId) {
                                    saveState = true
                                }
                                launchSingleTop = true
                                restoreState = true
                            }
                        },
                        label = {
                            Text(
                                text = screen.label,
                                style = MaterialTheme.typography.labelSmall,
                            )
                        },
                        icon = {
                            Icon(
                                imageVector = screen.icon,
                                contentDescription = screen.label,
                            )
                        },
                        colors = NavigationBarItemDefaults.colors(
                            selectedTextColor = MaterialTheme.colorScheme.primary,
                            selectedIconColor = MaterialTheme.colorScheme.primary,
                            unselectedTextColor = MaterialTheme.colorScheme.onSurfaceVariant,
                            unselectedIconColor = MaterialTheme.colorScheme.onSurfaceVariant,
                            indicatorColor = MaterialTheme.colorScheme.primaryContainer,
                        ),
                    )
                }
            }
        },
    ) { padding ->
        NavHost(
            navController = navController,
            startDestination = Screen.Dashboard.route,
            modifier = Modifier.padding(padding),
        ) {
            composable(Screen.Dashboard.route) { DashboardScreen(viewModel = sharedViewModel) }
            composable(Screen.Social.route) { SocialScreen(viewModel = sharedViewModel) }
            composable(Screen.Messages.route) { MessagesScreen(viewModel = sharedViewModel) }
            composable("governance") { GovernanceScreen(viewModel = sharedViewModel) }
            composable(Screen.Wallet.route) { WalletScreen(viewModel = sharedViewModel) }
            composable(Screen.Identity.route) { IdentityScreen(viewModel = sharedViewModel) }
            composable("ai") { AIScreen(viewModel = sharedViewModel) }
            composable("marketplace") { MarketplaceScreen(viewModel = sharedViewModel) }
            composable("files") { FilesScreen(viewModel = sharedViewModel) }
            composable("network") { NetworkScreen(viewModel = sharedViewModel) }
            composable("settings") { SettingsScreen(viewModel = sharedViewModel) }
        }
    }
}
