package com.nous.app.ui.theme

import androidx.compose.ui.unit.dp

// ─── Spacing scale ────────────────────────────────────────────────────────────
//
// 4-px base from docs/design/tokens.json: 4 / 8 / 12 / 16 / 24 / 32 / 48 / 64.
// Components use these via `NousSpacing.gutter` etc. instead of inlining numbers.

object NousSpacing {
    val xs = 4.dp
    val sm = 8.dp
    val md = 12.dp
    val lg = 16.dp
    val xl = 24.dp
    val xxl = 32.dp
    val gutter = 24.dp        // page horizontal gutter
    val sectionGap = 32.dp    // between sections
}
