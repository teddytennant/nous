package com.nous.app.ui

import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.NavigationBarItemDefaults
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.icons.Icons
import androidx.compose.material3.icons.outlined.AccountCircle
import androidx.compose.material3.icons.outlined.Home
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
import com.nous.app.ui.screens.DashboardScreen
import com.nous.app.ui.screens.GovernanceScreen
import com.nous.app.ui.screens.IdentityScreen
import com.nous.app.ui.screens.MessagesScreen
import com.nous.app.ui.screens.SocialScreen
import com.nous.app.ui.screens.WalletScreen

sealed class Screen(val route: String, val label: String) {
    data object Dashboard : Screen("dashboard", "Home")
    data object Social : Screen("social", "Social")
    data object Messages : Screen("messages", "Messages")
    data object Governance : Screen("governance", "Govern")
    data object Wallet : Screen("wallet", "Wallet")
    data object Identity : Screen("identity", "Identity")
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
                        icon = {},
                        colors = NavigationBarItemDefaults.colors(
                            selectedTextColor = MaterialTheme.colorScheme.primary,
                            unselectedTextColor = MaterialTheme.colorScheme.onSurfaceVariant,
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
            composable(Screen.Governance.route) { GovernanceScreen(viewModel = sharedViewModel) }
            composable(Screen.Wallet.route) { WalletScreen(viewModel = sharedViewModel) }
            composable(Screen.Identity.route) { IdentityScreen() }
        }
    }
}
