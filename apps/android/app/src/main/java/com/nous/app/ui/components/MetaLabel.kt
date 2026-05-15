package com.nous.app.ui.components

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.em
import com.nous.app.ui.theme.NousTheme
import com.nous.app.ui.theme.Stone

// ─── MetaLabel ───────────────────────────────────────────────────────────────
//
// Uppercase, tracked sans label. Default colour is `--stone`; `active = true`
// switches to `--oxblood`. Use for section captions, status pills, table
// headers, "TRANSACTIONS" / "PROPOSALS" etc. Strings are uppercased here so
// callers may pass natural-case copy.

@Composable
fun MetaLabel(
    text: String,
    modifier: Modifier = Modifier,
    active: Boolean = false,
    color: Color? = null,
) {
    val resolved = color
        ?: if (active) MaterialTheme.colorScheme.primary else Stone
    Text(
        text = text.uppercase(),
        style = MaterialTheme.typography.labelSmall.copy(letterSpacing = 0.04.em),
        color = resolved,
        modifier = modifier,
    )
}

@Preview(name = "dark", showBackground = true, backgroundColor = 0xFF0E0E0C)
@Preview(name = "light", showBackground = true, backgroundColor = 0xFFF4F1EA)
@Composable
private fun MetaLabelPreview() {
    NousTheme {
        MetaLabel("Transactions")
    }
}
