package com.nous.app.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.nous.app.ui.components.ModuleCard
import com.nous.app.ui.components.StatCard

data class Module(val name: String, val status: String)

@Composable
fun DashboardScreen() {
    val modules = listOf(
        Module("Identity", "active"),
        Module("Messaging", "active"),
        Module("Governance", "active"),
        Module("Social", "active"),
        Module("Payments", "standby"),
        Module("Storage", "active"),
        Module("AI", "standby"),
        Module("Browser", "standby"),
    )

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(24.dp),
    ) {
        Text(
            text = "Dashboard",
            style = MaterialTheme.typography.headlineLarge,
            modifier = Modifier.padding(bottom = 4.dp),
        )
        Text(
            text = "Sovereign protocol overview",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(bottom = 24.dp),
        )

        LazyVerticalGrid(
            columns = GridCells.Fixed(2),
            contentPadding = PaddingValues(0.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
            modifier = Modifier.padding(bottom = 32.dp),
        ) {
            item {
                StatCard(label = "Identity", value = "did:key:z6Mk...2doK")
            }
            item {
                StatCard(label = "Peers", value = "0")
            }
            item {
                StatCard(label = "Uptime", value = "0s")
            }
            item {
                StatCard(label = "Version", value = "0.1.0")
            }
        }

        Text(
            text = "PROTOCOL MODULES",
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(bottom = 12.dp),
        )

        LazyVerticalGrid(
            columns = GridCells.Fixed(2),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            items(modules) { module ->
                ModuleCard(
                    name = module.name,
                    status = module.status,
                    modifier = Modifier.fillMaxWidth(),
                )
            }
        }
    }
}
