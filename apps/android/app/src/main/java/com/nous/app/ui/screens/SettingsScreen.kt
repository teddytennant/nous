package com.nous.app.ui.screens

import androidx.compose.foundation.border
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
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
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
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import com.nous.app.data.NousViewModel

@Composable
fun SettingsScreen(viewModel: NousViewModel) {
    val node by viewModel.node.collectAsState()
    var notificationsEnabled by remember { mutableStateOf(true) }
    var autoConnect by remember { mutableStateOf(true) }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(24.dp),
    ) {
        Text(
            text = "Settings",
            style = MaterialTheme.typography.headlineLarge,
            modifier = Modifier.padding(bottom = 4.dp),
        )
        Text(
            text = "Configuration & preferences",
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(bottom = 32.dp),
        )

        // Node section
        SettingsSection(title = "NODE") {
            SettingsRow(label = "API Status", value = if (node.connected) "Connected" else "Offline")
            SettingsRow(label = "Version", value = node.version)
            SettingsRow(label = "API URL", value = "localhost:8080")
        }

        Spacer(modifier = Modifier.height(24.dp))

        // Network section
        SettingsSection(title = "NETWORK") {
            SettingsToggle(
                label = "Auto-connect",
                description = "Connect to peers on launch",
                checked = autoConnect,
                onCheckedChange = { autoConnect = it },
            )
            SettingsRow(label = "P2P Port", value = "9000")
            SettingsRow(label = "Max Peers", value = "50")
        }

        Spacer(modifier = Modifier.height(24.dp))

        // Notifications section
        SettingsSection(title = "NOTIFICATIONS") {
            SettingsToggle(
                label = "Push notifications",
                description = "Messages, governance votes, payments",
                checked = notificationsEnabled,
                onCheckedChange = { notificationsEnabled = it },
            )
        }

        Spacer(modifier = Modifier.height(24.dp))

        // Security section
        SettingsSection(title = "SECURITY") {
            SettingsRow(label = "Signing", value = "Ed25519")
            SettingsRow(label = "Key Exchange", value = "X25519")
            SettingsRow(label = "Encryption", value = "AES-256-GCM")
        }

        Spacer(modifier = Modifier.height(24.dp))

        // About section
        SettingsSection(title = "ABOUT") {
            SettingsRow(label = "App Version", value = "0.1.0")
            SettingsRow(label = "License", value = "MIT")
        }
    }
}

@Composable
private fun SettingsSection(title: String, content: @Composable () -> Unit) {
    Text(
        text = title,
        style = MaterialTheme.typography.labelSmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        modifier = Modifier.padding(bottom = 12.dp),
    )
    Surface(
        modifier = Modifier
            .fillMaxWidth()
            .border(1.dp, MaterialTheme.colorScheme.outline, RoundedCornerShape(8.dp)),
        color = MaterialTheme.colorScheme.surface,
        shape = RoundedCornerShape(8.dp),
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            content()
        }
    }
}

@Composable
private fun SettingsRow(label: String, value: String) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 10.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = label,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurface,
        )
        Text(
            text = value,
            style = MaterialTheme.typography.bodyMedium,
            fontFamily = FontFamily.Monospace,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
private fun SettingsToggle(
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
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurface,
            )
            Text(
                text = description,
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.outline,
            )
        }
        Switch(
            checked = checked,
            onCheckedChange = onCheckedChange,
            colors = SwitchDefaults.colors(
                checkedThumbColor = MaterialTheme.colorScheme.onPrimary,
                checkedTrackColor = MaterialTheme.colorScheme.primary,
            ),
        )
    }
}
