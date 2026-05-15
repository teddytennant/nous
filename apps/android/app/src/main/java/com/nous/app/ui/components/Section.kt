package com.nous.app.ui.components

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.nous.app.ui.theme.NousTheme

// ─── Section ─────────────────────────────────────────────────────────────────
//
// A labeled section: display-serif title, optional meta-caps subtitle in
// stone, hairline rule below, and the section content beneath that. Pure
// typography for hierarchy — no card chrome.

@Composable
fun Section(
    title: String,
    modifier: Modifier = Modifier,
    subtitle: String? = null,
    content: @Composable () -> Unit,
) {
    Column(modifier = modifier.fillMaxWidth()) {
        Text(
            text = title,
            style = MaterialTheme.typography.displaySmall,
            color = MaterialTheme.colorScheme.onSurface,
        )
        subtitle?.let {
            MetaLabel(text = it, modifier = Modifier.padding(top = 4.dp))
        }
        Hairline(modifier = Modifier.padding(top = 16.dp, bottom = 16.dp))
        content()
    }
}

@Preview(name = "dark", showBackground = true, backgroundColor = 0xFF0E0E0C)
@Preview(name = "light", showBackground = true, backgroundColor = 0xFFF4F1EA)
@Composable
private fun SectionPreview() {
    NousTheme {
        Section(title = "Identity", subtitle = "Self-sovereign · DID:key") {
            KeyValue(label = "Method", value = "ed25519")
        }
    }
}
