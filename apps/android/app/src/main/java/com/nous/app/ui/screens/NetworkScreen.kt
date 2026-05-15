package com.nous.app.ui.screens

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.nous.app.data.NousViewModel
import com.nous.app.ui.components.Hairline
import com.nous.app.ui.components.KeyValue
import com.nous.app.ui.components.MetaLabel
import com.nous.app.ui.components.OxbloodMark
import com.nous.app.ui.theme.NousMono
import com.nous.app.ui.theme.NousSpacing

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
            .padding(horizontal = NousSpacing.gutter, vertical = NousSpacing.xl),
    ) {
        Text(
            text = "Network",
            style = MaterialTheme.typography.displayMedium,
            color = MaterialTheme.colorScheme.onSurface,
        )
        Spacer(Modifier.height(8.dp))
        MetaLabel(text = "P2P mesh · libp2p")

        Spacer(Modifier.height(NousSpacing.xxl))
        Hairline()
        KeyValue(label = "Status", value = if (node.connected) "Online" else "Offline")
        Hairline()
        KeyValue(label = "Peers", value = "0")
        Hairline()
        KeyValue(label = "Version", value = node.version)
        Hairline()
        Spacer(Modifier.height(NousSpacing.xl))

        MetaLabel(text = "Subsystems")
        Spacer(Modifier.height(NousSpacing.md))
        Hairline()
        subsystems.forEach { sub ->
            Column(modifier = Modifier.fillMaxWidth().padding(vertical = NousSpacing.md)) {
                Row(modifier = Modifier.fillMaxWidth(), verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
                    Column(modifier = Modifier.weight(1f)) {
                        Text(
                            text = sub.name,
                            style = MaterialTheme.typography.headlineSmall,
                            color = MaterialTheme.colorScheme.onSurface,
                        )
                        Text(
                            text = sub.description,
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    if (sub.status == "operational") {
                        OxbloodMark(filled = true)
                        Spacer(Modifier.size(6.dp))
                    }
                    Text(
                        text = sub.status,
                        style = NousMono,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
            Hairline()
        }

        Spacer(Modifier.height(NousSpacing.xl))

        MetaLabel(text = "Protocol")
        Spacer(Modifier.height(NousSpacing.md))
        Hairline()
        KeyValue(label = "Transport", value = "libp2p TCP")
        Hairline()
        KeyValue(label = "Discovery", value = "Kademlia · mDNS")
        Hairline()
        KeyValue(label = "Pubsub", value = "gossipsub")
        Hairline()
        KeyValue(label = "P2P Port", value = "9000")
        Hairline()
        KeyValue(label = "Nostr Relay", value = "9735")
        Hairline()
    }
}
