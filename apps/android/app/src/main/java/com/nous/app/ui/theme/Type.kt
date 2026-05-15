package com.nous.app.ui.theme

import androidx.compose.material3.Typography
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.em
import androidx.compose.ui.unit.sp

// ─── Typography ───────────────────────────────────────────────────────────────
//
// Display tier uses Source Serif 4. Heading and body use Geist. Mono uses
// Geist Mono. Until those font families are bundled in `app/src/main/res/font/`
// and registered as `FontFamily` instances, we fall back to platform serif /
// sans / mono — which is correct for the editorial language anyway (Android's
// default sans is closer to Geist than to most alternatives).
//
// TODO(nous-design): bundle Geist + Source Serif 4 as font resources and
// replace `FontFamily.Serif` / `FontFamily.SansSerif` / `FontFamily.Monospace`
// here with the bundled families.

private val DisplayFamily = FontFamily.Serif
private val SansFamily = FontFamily.SansSerif
private val MonoFamily = FontFamily.Monospace

val NousTypography = Typography(
    // Display — serif. Used for hero / featured titles.
    displayLarge = TextStyle(
        fontFamily = DisplayFamily,
        fontWeight = FontWeight.Normal,
        fontSize = 56.sp,
        letterSpacing = (-0.02).em,
        lineHeight = 64.sp,
    ),
    displayMedium = TextStyle(
        fontFamily = DisplayFamily,
        fontWeight = FontWeight.Normal,
        fontSize = 44.sp,
        letterSpacing = (-0.02).em,
        lineHeight = 52.sp,
    ),
    displaySmall = TextStyle(
        fontFamily = DisplayFamily,
        fontWeight = FontWeight.Normal,
        fontSize = 36.sp,
        letterSpacing = (-0.02).em,
        lineHeight = 44.sp,
    ),
    // Headline — sans, medium weight.
    headlineLarge = TextStyle(
        fontFamily = SansFamily,
        fontWeight = FontWeight.Medium,
        fontSize = 28.sp,
        letterSpacing = (-0.01).em,
        lineHeight = 36.sp,
    ),
    headlineMedium = TextStyle(
        fontFamily = SansFamily,
        fontWeight = FontWeight.Medium,
        fontSize = 22.sp,
        letterSpacing = (-0.01).em,
        lineHeight = 28.sp,
    ),
    headlineSmall = TextStyle(
        fontFamily = SansFamily,
        fontWeight = FontWeight.Medium,
        fontSize = 18.sp,
        letterSpacing = (-0.01).em,
        lineHeight = 24.sp,
    ),
    // Title — sans.
    titleLarge = TextStyle(
        fontFamily = SansFamily,
        fontWeight = FontWeight.Medium,
        fontSize = 18.sp,
        letterSpacing = (-0.01).em,
        lineHeight = 24.sp,
    ),
    titleMedium = TextStyle(
        fontFamily = SansFamily,
        fontWeight = FontWeight.Normal,
        fontSize = 16.sp,
        lineHeight = 22.sp,
    ),
    titleSmall = TextStyle(
        fontFamily = SansFamily,
        fontWeight = FontWeight.Normal,
        fontSize = 14.sp,
        lineHeight = 20.sp,
    ),
    // Body — sans, primary reading.
    bodyLarge = TextStyle(
        fontFamily = SansFamily,
        fontWeight = FontWeight.Normal,
        fontSize = 16.sp,
        lineHeight = 24.sp,
    ),
    bodyMedium = TextStyle(
        fontFamily = SansFamily,
        fontWeight = FontWeight.Normal,
        fontSize = 14.sp,
        lineHeight = 20.sp,
    ),
    bodySmall = TextStyle(
        fontFamily = SansFamily,
        fontWeight = FontWeight.Normal,
        fontSize = 12.sp,
        lineHeight = 16.sp,
    ),
    // Label — uppercase, tracked. Callers should `.uppercase()` the string.
    labelLarge = TextStyle(
        fontFamily = SansFamily,
        fontWeight = FontWeight.Normal,
        fontSize = 12.sp,
        letterSpacing = 0.04.em,
        lineHeight = 16.sp,
    ),
    labelMedium = TextStyle(
        fontFamily = SansFamily,
        fontWeight = FontWeight.Normal,
        fontSize = 11.sp,
        letterSpacing = 0.04.em,
        lineHeight = 14.sp,
    ),
    labelSmall = TextStyle(
        fontFamily = SansFamily,
        fontWeight = FontWeight.Normal,
        fontSize = 10.sp,
        letterSpacing = 0.04.em,
        lineHeight = 14.sp,
    ),
)

/**
 * Mono text style for data — DIDs, hashes, peer counts, balances.
 * Not part of Material3 [Typography]; consumers reference it directly:
 * `Text("did:nous:…", style = NousMono)`.
 */
val NousMono = TextStyle(
    fontFamily = MonoFamily,
    fontWeight = FontWeight.Normal,
    fontSize = 14.sp,
    lineHeight = 20.sp,
)

/**
 * Featured mono — large numerals (balances, block heights, peer counts).
 */
val NousMonoLarge = TextStyle(
    fontFamily = MonoFamily,
    fontWeight = FontWeight.Normal,
    fontSize = 24.sp,
    lineHeight = 32.sp,
)
