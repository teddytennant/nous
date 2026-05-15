package com.nous.app.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.nous.app.ui.theme.NousTheme

// ─── Hairline ────────────────────────────────────────────────────────────────
//
// 1.dp divider painted at `colorScheme.outline` (= rule). Replaces every
// `HorizontalDivider` and bordered surface in the app. Two flavors:
// horizontal (full-width) and vertical (full-height).

@Composable
fun Hairline(
    modifier: Modifier = Modifier,
    color: Color = MaterialTheme.colorScheme.outline,
) {
    Box(
        modifier = modifier
            .fillMaxWidth()
            .height(1.dp)
            .background(color),
    )
}

@Composable
fun HairlineVertical(
    modifier: Modifier = Modifier,
    color: Color = MaterialTheme.colorScheme.outline,
) {
    Box(
        modifier = modifier
            .fillMaxHeight()
            .width(1.dp)
            .background(color),
    )
}

@Preview(name = "dark", showBackground = true, backgroundColor = 0xFF0E0E0C)
@Preview(name = "light", showBackground = true, backgroundColor = 0xFFF4F1EA)
@Composable
private fun HairlinePreview() {
    NousTheme { Hairline() }
}
