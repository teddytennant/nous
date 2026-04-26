package com.nous.app.ui.components

import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.nous.app.ui.theme.NousTheme
import com.nous.app.ui.theme.OxbloodPressIndication

// ─── EditorialRow ────────────────────────────────────────────────────────────
//
// A clickable row with leading / trailing slots and a press state painted as
// a 12% oxblood overlay (no Material ripple). Use for list items, settings
// rows, channel cells, etc. Renamed from `Row` to avoid clashing with the
// foundation primitive.

@Composable
fun EditorialRow(
    modifier: Modifier = Modifier,
    onClick: (() -> Unit)? = null,
    leading: (@Composable () -> Unit)? = null,
    trailing: (@Composable () -> Unit)? = null,
    content: @Composable () -> Unit,
) {
    val interactionSource = remember { MutableInteractionSource() }
    val clickModifier = if (onClick != null) {
        Modifier.clickable(
            interactionSource = interactionSource,
            indication = OxbloodPressIndication,
            onClick = onClick,
        )
    } else Modifier

    Row(
        modifier = modifier
            .fillMaxWidth()
            .then(clickModifier)
            .padding(vertical = 14.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        leading?.invoke()
        androidx.compose.foundation.layout.Box(modifier = Modifier.weight(1f)) { content() }
        trailing?.invoke()
    }
}

@Preview(name = "dark", showBackground = true, backgroundColor = 0xFF0E0E0C)
@Preview(name = "light", showBackground = true, backgroundColor = 0xFFF4F1EA)
@Composable
private fun EditorialRowPreview() {
    NousTheme {
        EditorialRow(onClick = {}) {
            androidx.compose.material3.Text("List item")
        }
    }
}
