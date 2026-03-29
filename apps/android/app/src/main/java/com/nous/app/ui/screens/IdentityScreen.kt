package com.nous.app.ui.screens

import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.nous.app.data.NousViewModel

private val Gold = Color(0xFFD4AF37)
private val TextPrimary = Color(0xFFFAFAFA)
private val TextSecondary = Color(0xFF737373)
private val SurfaceColor = Color(0xFF0A0A0A)
private val BorderColor = Color(0xFF1A1A1A)

@Composable
fun IdentityScreen(viewModel: NousViewModel = viewModel()) {
    val identity by viewModel.identity.collectAsState()
    val node by viewModel.node.collectAsState()

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(24.dp),
    ) {
        Text(
            text = "Identity",
            style = MaterialTheme.typography.headlineLarge,
            modifier = Modifier.padding(bottom = 4.dp),
        )
        Text(
            text = "Self-sovereign, DID:key",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(bottom = 24.dp),
        )

        if (identity == null && !node.connected) {
            // Offline state
            Surface(
                modifier = Modifier
                    .fillMaxWidth()
                    .border(1.dp, BorderColor, RoundedCornerShape(8.dp)),
                color = SurfaceColor,
                shape = RoundedCornerShape(8.dp),
            ) {
                Column(
                    modifier = Modifier.padding(24.dp),
                    horizontalAlignment = Alignment.CenterHorizontally,
                ) {
                    Text(
                        text = "API Offline",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.error,
                    )
                    Spacer(modifier = Modifier.height(8.dp))
                    Text(
                        text = "Unable to reach the Nous node. Identity will load when the API is available.",
                        style = MaterialTheme.typography.bodySmall,
                        color = TextSecondary,
                    )
                }
            }
        } else if (identity == null) {
            // Loading state
            LinearProgressIndicator(
                modifier = Modifier.fillMaxWidth(),
                color = Gold,
                trackColor = BorderColor,
            )
            Spacer(modifier = Modifier.height(16.dp))
            Text(
                text = "Loading identity...",
                style = MaterialTheme.typography.bodyMedium,
                color = TextSecondary,
            )
        } else {
            val id = identity!!
            val keys = listOf(
                KeyInfo(id.signing_key_type, "Signing", id.did.takeLast(8)),
                KeyInfo(id.exchange_key_type, "Key Exchange", "derived"),
            )

            Surface(
                modifier = Modifier
                    .fillMaxWidth()
                    .border(1.dp, MaterialTheme.colorScheme.outline, RoundedCornerShape(8.dp)),
                color = MaterialTheme.colorScheme.surface,
                shape = RoundedCornerShape(8.dp),
            ) {
                Column(modifier = Modifier.padding(24.dp)) {
                    Text(
                        text = "YOUR DID",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Spacer(modifier = Modifier.height(8.dp))
                    Text(
                        text = id.did,
                        style = MaterialTheme.typography.bodyMedium,
                        fontFamily = FontFamily.Monospace,
                        color = MaterialTheme.colorScheme.primary,
                    )
                    id.display_name?.let { name ->
                        Spacer(modifier = Modifier.height(4.dp))
                        Text(
                            text = name,
                            style = MaterialTheme.typography.bodySmall,
                            color = TextSecondary,
                        )
                    }

                    Spacer(modifier = Modifier.height(24.dp))

                    keys.forEach { key ->
                        HorizontalDivider(color = MaterialTheme.colorScheme.outline)
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(vertical = 12.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Text(
                                text = key.type,
                                fontFamily = FontFamily.Monospace,
                                style = MaterialTheme.typography.bodyMedium,
                                modifier = Modifier.weight(1f),
                            )
                            Text(
                                text = key.purpose,
                                style = MaterialTheme.typography.bodyMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                }
            }
        }
    }
}

data class KeyInfo(val type: String, val purpose: String, val fingerprint: String)
