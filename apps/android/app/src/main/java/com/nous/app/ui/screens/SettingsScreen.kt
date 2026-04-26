package com.nous.app.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.nous.app.data.NousViewModel
import com.nous.app.ui.components.Hairline
import com.nous.app.ui.components.KeyValue
import com.nous.app.ui.components.MetaLabel
import com.nous.app.ui.theme.NousSpacing

@Composable
fun SettingsScreen(viewModel: NousViewModel) {
    val node by viewModel.node.collectAsState()
    var notificationsEnabled by remember { mutableStateOf(true) }
    var autoConnect by remember { mutableStateOf(true) }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = NousSpacing.gutter, vertical = NousSpacing.xl),
    ) {
        Text(
            text = "Settings",
            style = MaterialTheme.typography.displayMedium,
            color = MaterialTheme.colorScheme.onSurface,
        )
        Spacer(Modifier.height(8.dp))
        MetaLabel(text = "Configuration · Preferences")

        Spacer(Modifier.height(NousSpacing.xxl))

        SectionTitle("Node")
        KeyValue(label = "API Status", value = if (node.connected) "Connected" else "Offline")
        Hairline()
        KeyValue(label = "Version", value = node.version)
        Hairline()
        KeyValue(label = "API URL", value = "localhost:8080")
        Hairline()

        Spacer(Modifier.height(NousSpacing.xl))

        SectionTitle("Network")
        ToggleRow(
            label = "Auto-connect",
            description = "Connect to peers on launch",
            checked = autoConnect,
            onCheckedChange = { autoConnect = it },
        )
        Hairline()
        KeyValue(label = "P2P Port", value = "9000")
        Hairline()
        KeyValue(label = "Max Peers", value = "50")
        Hairline()

        Spacer(Modifier.height(NousSpacing.xl))

        SectionTitle("Notifications")
        ToggleRow(
            label = "Push notifications",
            description = "Messages, governance votes, payments",
            checked = notificationsEnabled,
            onCheckedChange = { notificationsEnabled = it },
        )
        Hairline()

        Spacer(Modifier.height(NousSpacing.xl))

        SectionTitle("Security")
        KeyValue(label = "Signing", value = "Ed25519")
        Hairline()
        KeyValue(label = "Key Exchange", value = "X25519")
        Hairline()
        KeyValue(label = "Encryption", value = "AES-256-GCM")
        Hairline()

        Spacer(Modifier.height(NousSpacing.xl))

        SectionTitle("About")
        KeyValue(label = "App Version", value = "0.1.0")
        Hairline()
        KeyValue(label = "License", value = "MIT")
        Hairline()
    }
}

@Composable
private fun SectionTitle(text: String) {
    MetaLabel(text = text)
    Spacer(Modifier.height(NousSpacing.md))
    Hairline()
}

@Composable
private fun ToggleRow(
    label: String,
    description: String,
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clickable { onCheckedChange(!checked) }
            .padding(vertical = 10.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = label,
                style = MaterialTheme.typography.bodyLarge,
                color = MaterialTheme.colorScheme.onSurface,
            )
            Text(
                text = description,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        Switch(
            checked = checked,
            onCheckedChange = onCheckedChange,
            colors = SwitchDefaults.colors(
                checkedThumbColor = MaterialTheme.colorScheme.onPrimary,
                checkedTrackColor = MaterialTheme.colorScheme.primary,
                uncheckedThumbColor = MaterialTheme.colorScheme.onSurfaceVariant,
                uncheckedTrackColor = MaterialTheme.colorScheme.surface,
                uncheckedBorderColor = MaterialTheme.colorScheme.outline,
            ),
        )
    }
}
