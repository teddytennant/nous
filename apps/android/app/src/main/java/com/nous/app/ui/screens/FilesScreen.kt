package com.nous.app.ui.screens

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.nous.app.data.NousViewModel
import com.nous.app.ui.components.EmptyState
import com.nous.app.ui.components.Hairline
import com.nous.app.ui.components.KeyValue
import com.nous.app.ui.components.MetaLabel
import com.nous.app.ui.theme.NousSpacing

@Composable
fun FilesScreen(viewModel: NousViewModel) {
    val node by viewModel.node.collectAsState()

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(horizontal = NousSpacing.gutter, vertical = NousSpacing.xl),
    ) {
        Text(
            text = "Files",
            style = MaterialTheme.typography.displayMedium,
            color = MaterialTheme.colorScheme.onSurface,
        )
        Spacer(Modifier.height(8.dp))
        MetaLabel(text = "Encrypted · Content-addressed")

        Spacer(Modifier.height(NousSpacing.xxl))

        if (!node.connected) {
            Box(modifier = Modifier.fillMaxWidth().weight(1f), contentAlignment = Alignment.Center) {
                EmptyState(headline = "API offline", body = "Connect to the API to manage files.")
            }
        } else {
            Hairline()
            KeyValue(label = "Stored", value = "0")
            Hairline()
            KeyValue(label = "Chunks", value = "0")
            Hairline()
            KeyValue(label = "Dedup", value = "0%")
            Hairline()

            Box(modifier = Modifier.fillMaxWidth().weight(1f), contentAlignment = Alignment.Center) {
                EmptyState(
                    headline = "No files uploaded",
                    body = "Upload files from the web or desktop app.",
                )
            }
        }
    }
}
