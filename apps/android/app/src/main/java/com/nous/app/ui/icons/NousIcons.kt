package com.nous.app.ui.icons

import androidx.compose.material3.LocalContentColor
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.tooling.preview.Preview
import com.nous.app.ui.theme.NousTheme

// ─── Nous nav icon set ───────────────────────────────────────────────────────
//
// Hand-drawn Compose icons for primary navigation. Match the web/iOS
// silhouettes: simple geometric forms, 1.dp strokes, no fills. Every icon
// fills a 16.dp box; coordinates below are normalised to that box.

private const val Box = 16f

private fun off(x: Float, y: Float) = Offset(x, y)

@Composable
fun IconDashboard(modifier: Modifier = Modifier, tint: Color = LocalContentColor.current) {
    IconBox(modifier, tint) { stroke, color ->
        val s = size.width / Box
        // Two stacked rectangles: a tall left column, a short right block above wide right block.
        drawRect(color, topLeft = off(2f * s, 2f * s), size = Size(5f * s, 12f * s), style = stroke)
        drawRect(color, topLeft = off(9f * s, 2f * s), size = Size(5f * s, 6f * s), style = stroke)
        drawRect(color, topLeft = off(9f * s, 10f * s), size = Size(5f * s, 4f * s), style = stroke)
    }
}

@Composable
fun IconMessages(modifier: Modifier = Modifier, tint: Color = LocalContentColor.current) {
    IconBox(modifier, tint) { stroke, color ->
        val s = size.width / Box
        drawRect(color, topLeft = off(2f * s, 3f * s), size = Size(12f * s, 8f * s), style = stroke)
        // notch
        val path = Path().apply {
            moveTo(5f * s, 11f * s); lineTo(5f * s, 14f * s); lineTo(8f * s, 11f * s)
        }
        drawPath(path, color, style = stroke)
    }
}

@Composable
fun IconWallet(modifier: Modifier = Modifier, tint: Color = LocalContentColor.current) {
    IconBox(modifier, tint) { stroke, color ->
        val s = size.width / Box
        drawRect(color, topLeft = off(2f * s, 4f * s), size = Size(12f * s, 9f * s), style = stroke)
        // dot
        drawCircle(color, radius = 0.75f * s, center = off(11f * s, 8.5f * s))
    }
}

@Composable
fun IconIdentity(modifier: Modifier = Modifier, tint: Color = LocalContentColor.current) {
    IconBox(modifier, tint) { stroke, color ->
        val s = size.width / Box
        // head
        drawCircle(color, radius = 2.5f * s, center = off(8f * s, 6f * s), style = stroke)
        // shoulders arc as a rectangle's top
        val path = Path().apply {
            moveTo(3f * s, 14f * s); lineTo(3f * s, 12f * s)
            cubicTo(3f * s, 10f * s, 13f * s, 10f * s, 13f * s, 12f * s)
            lineTo(13f * s, 14f * s)
        }
        drawPath(path, color, style = stroke)
    }
}

@Composable
fun IconSocial(modifier: Modifier = Modifier, tint: Color = LocalContentColor.current) {
    IconBox(modifier, tint) { stroke, color ->
        val s = size.width / Box
        drawCircle(color, radius = 1.75f * s, center = off(5f * s, 6f * s), style = stroke)
        drawCircle(color, radius = 1.75f * s, center = off(11f * s, 6f * s), style = stroke)
        val path = Path().apply {
            moveTo(2f * s, 14f * s); lineTo(2f * s, 12f * s)
            cubicTo(2f * s, 10f * s, 14f * s, 10f * s, 14f * s, 12f * s)
            lineTo(14f * s, 14f * s)
        }
        drawPath(path, color, style = stroke)
    }
}

@Composable
fun IconGovernance(modifier: Modifier = Modifier, tint: Color = LocalContentColor.current) {
    IconBox(modifier, tint) { stroke, color ->
        val s = size.width / Box
        // pillars + base
        drawLine(color, off(3f * s, 5f * s), off(13f * s, 5f * s), strokeWidth = stroke.width)
        drawLine(color, off(4f * s, 5f * s), off(4f * s, 12f * s), strokeWidth = stroke.width)
        drawLine(color, off(8f * s, 5f * s), off(8f * s, 12f * s), strokeWidth = stroke.width)
        drawLine(color, off(12f * s, 5f * s), off(12f * s, 12f * s), strokeWidth = stroke.width)
        drawLine(color, off(2f * s, 13f * s), off(14f * s, 13f * s), strokeWidth = stroke.width)
    }
}

@Composable
fun IconAi(modifier: Modifier = Modifier, tint: Color = LocalContentColor.current) {
    IconBox(modifier, tint) { stroke, color ->
        val s = size.width / Box
        drawRect(color, topLeft = off(3f * s, 3f * s), size = Size(10f * s, 10f * s), style = stroke)
        // central dot grid
        drawCircle(color, 0.5f * s, off(6f * s, 6f * s))
        drawCircle(color, 0.5f * s, off(10f * s, 6f * s))
        drawCircle(color, 0.5f * s, off(8f * s, 10f * s))
    }
}

@Composable
fun IconNetwork(modifier: Modifier = Modifier, tint: Color = LocalContentColor.current) {
    IconBox(modifier, tint) { stroke, color ->
        val s = size.width / Box
        drawCircle(color, 1.25f * s, off(8f * s, 3f * s), style = stroke)
        drawCircle(color, 1.25f * s, off(3f * s, 13f * s), style = stroke)
        drawCircle(color, 1.25f * s, off(13f * s, 13f * s), style = stroke)
        drawLine(color, off(8f * s, 4.25f * s), off(3f * s, 11.75f * s), strokeWidth = stroke.width)
        drawLine(color, off(8f * s, 4.25f * s), off(13f * s, 11.75f * s), strokeWidth = stroke.width)
        drawLine(color, off(4.25f * s, 13f * s), off(11.75f * s, 13f * s), strokeWidth = stroke.width)
    }
}

@Composable
fun IconFiles(modifier: Modifier = Modifier, tint: Color = LocalContentColor.current) {
    IconBox(modifier, tint) { stroke, color ->
        val s = size.width / Box
        val path = Path().apply {
            moveTo(3f * s, 3f * s); lineTo(9f * s, 3f * s); lineTo(13f * s, 7f * s)
            lineTo(13f * s, 14f * s); lineTo(3f * s, 14f * s); close()
        }
        drawPath(path, color, style = stroke)
        // fold corner
        drawLine(color, off(9f * s, 3f * s), off(9f * s, 7f * s), strokeWidth = stroke.width)
        drawLine(color, off(9f * s, 7f * s), off(13f * s, 7f * s), strokeWidth = stroke.width)
    }
}

@Composable
fun IconMarketplace(modifier: Modifier = Modifier, tint: Color = LocalContentColor.current) {
    IconBox(modifier, tint) { stroke, color ->
        val s = size.width / Box
        // awning
        drawRect(color, topLeft = off(2f * s, 4f * s), size = Size(12f * s, 3f * s), style = stroke)
        // body
        drawRect(color, topLeft = off(3f * s, 7f * s), size = Size(10f * s, 7f * s), style = stroke)
        drawLine(color, off(8f * s, 7f * s), off(8f * s, 14f * s), strokeWidth = stroke.width)
    }
}

@Composable
fun IconSettings(modifier: Modifier = Modifier, tint: Color = LocalContentColor.current) {
    IconBox(modifier, tint) { stroke, color ->
        val s = size.width / Box
        // three horizontal sliders with knobs
        drawLine(color, off(3f * s, 5f * s), off(13f * s, 5f * s), strokeWidth = stroke.width)
        drawCircle(color, 1.25f * s, off(6f * s, 5f * s), style = stroke)
        drawLine(color, off(3f * s, 9f * s), off(13f * s, 9f * s), strokeWidth = stroke.width)
        drawCircle(color, 1.25f * s, off(11f * s, 9f * s), style = stroke)
        drawLine(color, off(3f * s, 13f * s), off(13f * s, 13f * s), strokeWidth = stroke.width)
        drawCircle(color, 1.25f * s, off(8f * s, 13f * s), style = stroke)
    }
}

@Preview(name = "dark", showBackground = true, backgroundColor = 0xFF0E0E0C)
@Preview(name = "light", showBackground = true, backgroundColor = 0xFFF4F1EA)
@Composable
private fun NousIconsPreview() {
    NousTheme { IconDashboard() }
}
