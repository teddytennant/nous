package com.nous.app.ui.components

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.size
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.nous.app.ui.theme.NousTheme

// ─── OxbloodMark ─────────────────────────────────────────────────────────────
//
// The brand stamp. A tiny canvas-drawn glyph painted in oxblood (or current
// `colorScheme.primary`). Two flavors: a hairline circle (default — used in
// nav active state) and a filled dot (used as a read receipt). 8.dp baseline.

@Composable
fun OxbloodMark(
    modifier: Modifier = Modifier,
    size: androidx.compose.ui.unit.Dp = 8.dp,
    filled: Boolean = false,
    color: Color = MaterialTheme.colorScheme.primary,
) {
    Canvas(modifier = modifier.size(size)) {
        val radius = this.size.minDimension / 2f
        if (filled) {
            drawCircle(color = color, radius = radius, center = Offset(this.size.width / 2f, this.size.height / 2f))
        } else {
            drawCircle(
                color = color,
                radius = radius - 0.5f.dp.toPx(),
                center = Offset(this.size.width / 2f, this.size.height / 2f),
                style = Stroke(width = 1.dp.toPx()),
            )
        }
    }
}

@Preview(name = "dark", showBackground = true, backgroundColor = 0xFF0E0E0C)
@Preview(name = "light", showBackground = true, backgroundColor = 0xFFF4F1EA)
@Composable
private fun OxbloodMarkPreview() {
    NousTheme { OxbloodMark() }
}
