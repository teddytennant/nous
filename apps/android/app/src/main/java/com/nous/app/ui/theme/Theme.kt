package com.nous.app.ui.theme

import androidx.compose.foundation.LocalIndication
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Shapes
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.unit.dp

// ─── Editorial color schemes ──────────────────────────────────────────────────
//
// Material3 names are preserved (primary / onPrimary / surface / outline …)
// but their values are remapped to the new palette so existing `colorScheme`
// references in screens continue to compile and read sensibly. Hairlines map
// to `outline`; tertiary state goes through Material's `tertiary` slot.

private val NousDarkColors = darkColorScheme(
    primary = Oxblood,
    onPrimary = Ivory,
    primaryContainer = OxbloodDim,
    onPrimaryContainer = Ivory,
    secondary = IvoryDim,
    onSecondary = Ink,
    secondaryContainer = Ink2,
    onSecondaryContainer = Ivory,
    tertiary = Sage,
    onTertiary = Ink,
    background = Ink,
    onBackground = Ivory,
    surface = Ink,
    onSurface = Ivory,
    surfaceVariant = Ink2,
    onSurfaceVariant = IvoryDim,
    surfaceTint = Ink,         // disable Material's tonal-elevation tint
    outline = Rule,            // hairlines
    outlineVariant = Rule,
    error = Clay,
    onError = Ink,
    errorContainer = Clay,
    onErrorContainer = Ink,
)

private val NousLightColors = lightColorScheme(
    primary = Oxblood,
    onPrimary = Paper,
    primaryContainer = OxbloodDim,
    onPrimaryContainer = Paper,
    secondary = IvoryDimLight,
    onSecondary = Paper,
    secondaryContainer = Paper,
    onSecondaryContainer = InkText,
    tertiary = SageLight,
    onTertiary = Paper,
    background = Paper,
    onBackground = InkText,
    surface = Paper,
    onSurface = InkText,
    surfaceVariant = Paper,
    onSurfaceVariant = StoneLight,
    surfaceTint = Paper,
    outline = Hairline,
    outlineVariant = Hairline,
    error = ClayLight,
    onError = Paper,
    errorContainer = ClayLight,
    onErrorContainer = Paper,
)

// ─── Shapes — sharp by default ────────────────────────────────────────────────
//
// Editorial radii: 0 / 2 / 4 / pill. Anything rounder reads as software.
// Material3 maps small/medium/large to component categories — buttons, cards,
// dialogs. We keep the hierarchy flat: 2dp / 4dp / 4dp.

private val NousShapes = Shapes(
    extraSmall = RoundedCornerShape(0.dp),
    small = RoundedCornerShape(2.dp),
    medium = RoundedCornerShape(4.dp),
    large = RoundedCornerShape(4.dp),
    extraLarge = RoundedCornerShape(4.dp),
)

@Composable
fun NousTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    content: @Composable () -> Unit,
) {
    val colors = if (darkTheme) NousDarkColors else NousLightColors
    MaterialTheme(
        colorScheme = colors,
        typography = NousTypography,
        shapes = NousShapes,
    ) {
        // Replace Material's expanding ripple with the flat 12% oxblood overlay
        // for every `clickable`, `selectable`, `combinedClickable`, etc., that
        // doesn't pass an explicit indication.
        CompositionLocalProvider(LocalIndication provides OxbloodPressIndication) {
            content()
        }
    }
}
