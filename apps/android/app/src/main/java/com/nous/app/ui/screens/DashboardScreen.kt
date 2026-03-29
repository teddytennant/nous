package com.nous.app.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.nous.app.data.NousViewModel
import com.nous.app.ui.components.ModuleCard
import com.nous.app.ui.components.StatCard

data class Module(val name: String, val status: String)

@Composable
fun DashboardScreen(viewModel: NousViewModel = viewModel()) {
    val node by viewModel.node.collectAsState()

    val modules = if (node.connected) {
        listOf(
            Module("Identity", "active"),
            Module("Messaging", "active"),
            Module("Governance", "active"),
            Module("Social", "active"),
            Module("Payments", "active"),
            Module("Storage", "active"),
            Module("AI", "standby"),
            Module("Browser", "standby"),
        )
    } else {
        listOf(
            Module("Identity", "offline"),
            Module("Messaging", "offline"),
            Module("Governance", "offline"),
            Module("Social", "offline"),
            Module("Payments", "offline"),
            Module("Storage", "offline"),
            Module("AI", "offline"),
            Module("Browser", "offline"),
        )
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(24.dp),
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column {
                Text(
                    text = "Dashboard",
                    style = MaterialTheme.typography.headlineLarge,
                    modifier = Modifier.padding(bottom = 4.dp),
                )
                Text(
                    text = if (node.connected) "Connected to API" else "Offline",
                    style = MaterialTheme.typography.bodyMedium,
                    color = if (node.connected)
                        MaterialTheme.colorScheme.primary
                    else
                        MaterialTheme.colorScheme.error,
                    modifier = Modifier.padding(bottom = 24.dp),
                )
            }
        }

        val didDisplay = if (node.did.length > 24) "${node.did.take(20)}..." else node.did
        val uptimeDisplay = when {
            node.uptimeMs < 60_000 -> "${node.uptimeMs / 1000}s"
            node.uptimeMs < 3_600_000 -> "${node.uptimeMs / 60_000}m"
            else -> "${node.uptimeMs / 3_600_000}h"
        }

        LazyVerticalGrid(
            columns = GridCells.Fixed(2),
            contentPadding = PaddingValues(0.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
            modifier = Modifier.padding(bottom = 32.dp),
        ) {
            item { StatCard(label = "Identity", value = didDisplay) }
            item { StatCard(label = "Peers", value = "0") }
            item { StatCard(label = "Uptime", value = uptimeDisplay) }
            item { StatCard(label = "Version", value = node.version) }
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
