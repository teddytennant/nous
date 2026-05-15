package com.nous.app.ui.icons

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.size
import androidx.compose.material3.LocalContentColor
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.unit.dp

// ─── Icon base ───────────────────────────────────────────────────────────────
//
// Every nous icon is a 16.dp box, 1.dp stroke, currentColor. Drawing is hand
// rolled in a `Canvas` so we never inherit Material's filled glyphs. The
// `IconBox` helper centralises the size/stroke/colour scaffolding so the
// individual icons are pure shape code.

internal val IconStroke
    get() = Stroke(width = 1.dp.value)

@Composable
internal fun IconBox(
    modifier: Modifier = Modifier,
    tint: Color = LocalContentColor.current,
    draw: DrawScope.(stroke: Stroke, color: Color) -> Unit,
) {
    Canvas(modifier = modifier.size(16.dp)) {
        val stroke = Stroke(width = 1.dp.toPx())
        draw(stroke, tint)
    }
}
