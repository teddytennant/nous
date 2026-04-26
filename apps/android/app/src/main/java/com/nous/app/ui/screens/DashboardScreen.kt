package com.nous.app.ui.screens

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.nous.app.data.NousViewModel
import com.nous.app.ui.components.EditorialRow
import com.nous.app.ui.components.Hairline
import com.nous.app.ui.components.KeyValue
import com.nous.app.ui.components.MetaLabel
import com.nous.app.ui.components.OxbloodMark
import com.nous.app.ui.theme.NousMono
import com.nous.app.ui.theme.NousSpacing

private data class ModuleEntry(val name: String, val status: String)

@Composable
fun DashboardScreen(viewModel: NousViewModel = viewModel()) {
    val node by viewModel.node.collectAsState()

    val modules = if (node.connected) {
        listOf(
            ModuleEntry("Identity", "active"),
            ModuleEntry("Messaging", "active"),
            ModuleEntry("Governance", "active"),
            ModuleEntry("Social", "active"),
            ModuleEntry("Payments", "active"),
            ModuleEntry("Storage", "active"),
            ModuleEntry("AI", "standby"),
            ModuleEntry("Browser", "standby"),
        )
    } else {
        List(8) { ModuleEntry(listOf("Identity", "Messaging", "Governance", "Social", "Payments", "Storage", "AI", "Browser")[it], "offline") }
    }

    val didDisplay = if (node.did.length > 28) "${node.did.take(24)}…" else node.did
    val uptimeDisplay = when {
        node.uptimeMs < 60_000 -> "${node.uptimeMs / 1000}s"
        node.uptimeMs < 3_600_000 -> "${node.uptimeMs / 60_000}m"
        else -> "${node.uptimeMs / 3_600_000}h"
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = NousSpacing.gutter, vertical = NousSpacing.xl),
    ) {
        // Greeting
        Text(
            text = "Dashboard",
            style = MaterialTheme.typography.displayMedium,
            color = MaterialTheme.colorScheme.onSurface,
        )
        Spacer(Modifier.height(8.dp))
        MetaLabel(
            text = if (node.connected) "Connected · ${node.version}" else "Offline",
            active = node.connected,
        )

        Spacer(Modifier.height(NousSpacing.xxl))
        Hairline()

        // Network stats
        KeyValue(label = "Identity", value = didDisplay)
        Hairline()
        KeyValue(label = "Peers", value = "0")
        Hairline()
        KeyValue(label = "Uptime", value = uptimeDisplay)
        Hairline()
        KeyValue(label = "Version", value = node.version)
        Hairline()

        Spacer(Modifier.height(NousSpacing.xxl))

        MetaLabel(text = "Protocol Modules")
        Spacer(Modifier.height(NousSpacing.md))
        Hairline()
        modules.forEach { module ->
            EditorialRow(
                trailing = {
                    if (module.status == "active") OxbloodMark(filled = true)
                    Text(
                        text = module.status,
                        style = NousMono,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(start = 8.dp),
                    )
                },
            ) {
                Text(
                    text = module.name,
                    style = MaterialTheme.typography.bodyLarge,
                    color = MaterialTheme.colorScheme.onSurface,
                )
            }
            Hairline()
        }

        Spacer(Modifier.height(NousSpacing.xl))
    }
}
