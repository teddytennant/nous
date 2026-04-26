package com.nous.app.ui.screens

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
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.ClipboardManager
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.nous.app.data.NousViewModel
import com.nous.app.ui.components.EmptyState
import com.nous.app.ui.components.Hairline
import com.nous.app.ui.components.KeyValue
import com.nous.app.ui.components.MetaLabel
import com.nous.app.ui.components.OxbloodMark
import com.nous.app.ui.components.Section
import com.nous.app.ui.theme.NousMono
import com.nous.app.ui.theme.NousSpacing

@Composable
fun IdentityScreen(viewModel: NousViewModel = viewModel()) {
    val identity by viewModel.identity.collectAsState()
    val node by viewModel.node.collectAsState()
    val clipboard: ClipboardManager = LocalClipboardManager.current

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = NousSpacing.gutter, vertical = NousSpacing.xl),
    ) {
        Text(
            text = "Identity",
            style = MaterialTheme.typography.displayMedium,
            color = MaterialTheme.colorScheme.onSurface,
        )
        Spacer(Modifier.height(8.dp))
        MetaLabel(text = "Self-sovereign · DID:key")

        Spacer(Modifier.height(NousSpacing.xxl))

        when {
            identity == null && !node.connected -> {
                EmptyState(
                    headline = "API offline",
                    body = "Identity will load when the node is reachable.",
                )
            }
            identity == null -> {
                LinearProgressIndicator(
                    modifier = Modifier.fillMaxWidth(),
                    color = MaterialTheme.colorScheme.primary,
                    trackColor = MaterialTheme.colorScheme.outline,
                )
                Spacer(Modifier.height(12.dp))
                MetaLabel(text = "Loading…")
            }
            else -> {
                val id = identity!!
                MetaLabel(text = "DID")
                Spacer(Modifier.height(8.dp))
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween,
                ) {
                    Text(
                        text = id.did,
                        style = NousMono,
                        color = MaterialTheme.colorScheme.onSurface,
                        modifier = Modifier.weight(1f),
                    )
                    IconButton(
                        onClick = { clipboard.setText(AnnotatedString(id.did)) },
                    ) { OxbloodMark() }
                }
                id.display_name?.let {
                    Spacer(Modifier.height(4.dp))
                    Text(
                        text = it,
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }

                Spacer(Modifier.height(NousSpacing.xxl))

                Section(title = "Methods") {
                    KeyValue(label = "Signing", value = id.signing_key_type)
                    Hairline()
                    KeyValue(label = "Key Exchange", value = id.exchange_key_type)
                    Hairline()
                }

                Spacer(Modifier.height(NousSpacing.xxl))

                Section(title = "Services") {
                    KeyValue(label = "Resolver", value = "did:key")
                    Hairline()
                    KeyValue(label = "Storage", value = "local-first")
                    Hairline()
                }

                Spacer(Modifier.height(NousSpacing.xxl))

                Section(title = "Controllers") {
                    KeyValue(label = "Self", value = id.did.takeLast(12))
                    Hairline()
                }
            }
        }

        Spacer(Modifier.height(NousSpacing.xl))
    }
}
