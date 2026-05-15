package com.nous.app.ui.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.nous.app.ui.theme.NousMono
import com.nous.app.ui.theme.NousTheme

// ─── KeyValue ────────────────────────────────────────────────────────────────
//
// Editorial fact row: meta-caps key on the left in stone, mono value on the
// right in ivory. No icons, no arrow, no chevron. The mono value carries the
// "this is a fact" cue — counts, balances, addresses, hashes, versions.

@Composable
fun KeyValue(
    label: String,
    value: String,
    modifier: Modifier = Modifier,
) {
    Row(
        modifier = modifier
            .fillMaxWidth()
            .padding(vertical = 10.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        MetaLabel(text = label)
        Text(
            text = value,
            style = NousMono,
            color = MaterialTheme.colorScheme.onSurface,
        )
    }
}

@Preview(name = "dark", showBackground = true, backgroundColor = 0xFF0E0E0C)
@Preview(name = "light", showBackground = true, backgroundColor = 0xFFF4F1EA)
@Composable
private fun KeyValuePreview() {
    NousTheme {
        KeyValue(label = "Peers", value = "127")
    }
}
