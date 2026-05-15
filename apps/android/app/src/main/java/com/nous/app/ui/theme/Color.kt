package com.nous.app.ui.theme

import androidx.compose.ui.graphics.Color

// ─── Editorial palette ────────────────────────────────────────────────────────
//
// Source of truth: docs/design/tokens.json. Eight surface tokens plus two
// state tokens, oxblood as the single accent. Hex values mirror the dark and
// light blocks of tokens.json verbatim.

// Dark mode — default.
val Ink = Color(0xFF0E0E0C)         // primary surface
val Ink2 = Color(0xFF161613)        // raised surface
val Ivory = Color(0xFFEFEAE0)       // primary text
val IvoryDim = Color(0xFFC9C3B6)    // secondary text
val Stone = Color(0xFF6F6A60)       // tertiary / metadata
val Rule = Color(0xFF1F1D1A)        // 1px hairlines
val Oxblood = Color(0xFFB23A3A)     // single accent
val OxbloodDim = Color(0xFF7A2A2A)  // pressed / visited
val Sage = Color(0xFF8FA48A)        // positive state
val Clay = Color(0xFFC2785A)        // warning state

// Light mode — variant.
val Paper = Color(0xFFF4F1EA)
val InkText = Color(0xFF14130F)
val IvoryDimLight = Color(0xFF5A564E)
val StoneLight = Color(0xFF8A8478)
val Hairline = Color(0xFFDCD7CC)
val SageLight = Color(0xFF6E8468)
val ClayLight = Color(0xFFA4604A)
