package com.nous.app.ui.screens

import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import com.nous.app.data.NousViewModel

data class Subsystem(val name: String, val description: String, val status: String)

@Composable
fun NetworkScreen(viewModel: NousViewModel) {
    val node by viewModel.node.collectAsState()

    val subsystems = listOf(
        Subsystem("Identity", "DID:key generation and verification", if (node.connected) "operational" else "offline"),
        Subsystem("Crypto", "Ed25519 signing, X25519 key exchange", if (node.connected) "operational" else "offline"),
        Subsystem("Messaging", "E2E encrypted channels", if (node.connected) "operational" else "offline"),
        Subsystem("Social", "Nostr-compatible feeds", if (node.connected) "operational" else "offline"),
        Subsystem("Governance", "DAO proposals and voting", if (node.connected) "operational" else "offline"),
        Subsystem("Payments", "Multi-chain wallets", if (node.connected) "operational" else "offline"),
        Subsystem("Storage", "SQLite + CRDTs", if (node.connected) "operational" else "offline"),
        Subsystem("AI", "Local inference engine", "standby"),
    )

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(24.dp),
    ) {
        Text(
            text = "Network",
            style = MaterialTheme.typography.headlineLarge,
            modifier = Modifier.padding(bottom = 4.dp),
        )
        Text(
            text = "P2P mesh · libp2p transport",
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(bottom = 24.dp),
        )

        // Overview stats
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(bottom = 24.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            NetworkStat(
                label = "STATUS",
                value = if (node.connected) "Online" else "Offline",
                valueColor = if (node.connected)
                    MaterialTheme.colorScheme.primary
                else
                    MaterialTheme.colorScheme.error,
                modifier = Modifier.weight(1f),
            )
            NetworkStat(
                label = "PEERS",
                value = "0",
                modifier = Modifier.weight(1f),
            )
            NetworkStat(
                label = "VERSION",
                value = node.version,
                modifier = Modifier.weight(1f),
            )
        }

        // Subsystems
        Text(
            text = "SUBSYSTEMS",
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
            Column(modifier = Modifier.padding(4.dp)) {
                subsystems.forEach { subsystem ->
                    SubsystemRow(subsystem = subsystem)
                }
            }
        }

        Spacer(modifier = Modifier.height(24.dp))

        // Protocol info
        Text(
            text = "PROTOCOL",
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
                ProtocolRow(label = "Transport", value = "libp2p TCP")
                ProtocolRow(label = "Discovery", value = "Kademlia DHT + mDNS")
                ProtocolRow(label = "Pubsub", value = "gossipsub")
                ProtocolRow(label = "P2P Port", value = "9000")
                ProtocolRow(label = "Nostr Relay", value = "9735")
            }
        }
    }
}

@Composable
private fun NetworkStat(
    label: String,
    value: String,
    modifier: Modifier = Modifier,
    valueColor: androidx.compose.ui.graphics.Color = MaterialTheme.colorScheme.onSurface,
) {
    Surface(
        modifier = modifier
            .border(1.dp, MaterialTheme.colorScheme.outline, RoundedCornerShape(8.dp)),
        color = MaterialTheme.colorScheme.surface,
        shape = RoundedCornerShape(8.dp),
    ) {
        Column(modifier = Modifier.padding(12.dp)) {
            Text(
                text = label,
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(modifier = Modifier.height(4.dp))
            Text(
                text = value,
                style = MaterialTheme.typography.bodyLarge,
                fontFamily = FontFamily.Monospace,
                color = valueColor,
            )
        }
    }
}

@Composable
private fun SubsystemRow(subsystem: Subsystem) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 12.dp, vertical = 10.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = subsystem.name,
                style = MaterialTheme.typography.bodyMedium,
            )
            Text(
                text = subsystem.description,
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.outline,
            )
        }
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Box(
                modifier = Modifier
                    .size(6.dp)
                    .then(
                        Modifier.border(
                            0.dp,
                            when (subsystem.status) {
                                "operational" -> MaterialTheme.colorScheme.primary
                                "standby" -> MaterialTheme.colorScheme.secondary
                                else -> MaterialTheme.colorScheme.error
                            },
                            CircleShape,
                        )
                    ),
            ) {
                Surface(
                    modifier = Modifier.size(6.dp),
                    shape = CircleShape,
                    color = when (subsystem.status) {
                        "operational" -> MaterialTheme.colorScheme.primary
                        "standby" -> MaterialTheme.colorScheme.secondary
                        else -> MaterialTheme.colorScheme.error
                    },
                ) {}
            }
            Text(
                text = subsystem.status,
                style = MaterialTheme.typography.labelSmall,
                fontFamily = FontFamily.Monospace,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
private fun ProtocolRow(label: String, value: String) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 6.dp),
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
