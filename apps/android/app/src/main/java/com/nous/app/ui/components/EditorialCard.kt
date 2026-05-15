package com.nous.app.ui.components

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.nous.app.ui.theme.NousSpacing
import com.nous.app.ui.theme.NousTheme

// ─── EditorialCard ───────────────────────────────────────────────────────────
//
// A content block bounded by hairlines instead of borders or shadows. No
// background fill, no radius, no elevation — just type, whitespace, and
// optional rules above and below. This is the "card" in the editorial
// language: a passage, not a chip.

@Composable
fun EditorialCard(
    modifier: Modifier = Modifier,
    topRule: Boolean = false,
    bottomRule: Boolean = true,
    contentPadding: androidx.compose.foundation.layout.PaddingValues =
        androidx.compose.foundation.layout.PaddingValues(vertical = NousSpacing.lg),
    content: @Composable () -> Unit,
) {
    Column(modifier = modifier.fillMaxWidth()) {
        if (topRule) Hairline()
        Column(modifier = Modifier.padding(contentPadding)) { content() }
        if (bottomRule) Hairline()
    }
}

@Preview(name = "dark", showBackground = true, backgroundColor = 0xFF0E0E0C)
@Preview(name = "light", showBackground = true, backgroundColor = 0xFFF4F1EA)
@Composable
private fun EditorialCardPreview() {
    NousTheme {
        EditorialCard {
            KeyValue(label = "Block", value = "1,294,210")
            KeyValue(label = "Peers", value = "127")
        }
    }
}
