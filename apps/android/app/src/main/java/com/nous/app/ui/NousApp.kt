package com.nous.app.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavDestination.Companion.hierarchy
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import com.nous.app.data.NousViewModel
import com.nous.app.ui.components.Hairline
import com.nous.app.ui.components.OxbloodMark
import com.nous.app.ui.icons.IconDashboard
import com.nous.app.ui.icons.IconIdentity
import com.nous.app.ui.icons.IconMessages
import com.nous.app.ui.icons.IconSocial
import com.nous.app.ui.icons.IconWallet
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

sealed class Screen(
    val route: String,
    val label: String,
    val icon: @Composable (tint: androidx.compose.ui.graphics.Color) -> Unit,
) {
    data object Dashboard : Screen("dashboard", "Home", { tint -> IconDashboard(tint = tint) })
    data object Social : Screen("social", "Social", { tint -> IconSocial(tint = tint) })
    data object Messages : Screen("messages", "Messages", { tint -> IconMessages(tint = tint) })
    data object Wallet : Screen("wallet", "Wallet", { tint -> IconWallet(tint = tint) })
    data object Identity : Screen("identity", "Identity", { tint -> IconIdentity(tint = tint) })
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
        containerColor = MaterialTheme.colorScheme.background,
        bottomBar = {
            EditorialNavBar(
                screens = screens,
                isSelected = { screen ->
                    currentDestination?.hierarchy?.any { it.route == screen.route } == true
                },
                onSelect = { screen ->
                    navController.navigate(screen.route) {
                        popUpTo(navController.graph.startDestinationId) { saveState = true }
                        launchSingleTop = true
                        restoreState = true
                    }
                },
            )
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

@Composable
private fun EditorialNavBar(
    screens: List<Screen>,
    isSelected: (Screen) -> Boolean,
    onSelect: (Screen) -> Unit,
) {
    Column {
        Hairline()
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .height(56.dp)
                .padding(horizontal = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.SpaceBetween,
        ) {
            screens.forEach { screen ->
                val active = isSelected(screen)
                Column(
                    modifier = Modifier
                        .weight(1f)
                        .clickable(onClick = { onSelect(screen) })
                        .padding(vertical = 8.dp),
                    horizontalAlignment = Alignment.CenterHorizontally,
                ) {
                    val tint = if (active)
                        MaterialTheme.colorScheme.primary
                    else
                        MaterialTheme.colorScheme.onSurfaceVariant
                    Box(contentAlignment = Alignment.Center) {
                        screen.icon(tint)
                    }
                    Spacer(modifier = Modifier.height(4.dp))
                    if (active) {
                        OxbloodMark(filled = true)
                    } else {
                        Text(
                            text = screen.label,
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }
        }
    }
}
