package com.nous.app.ui.theme

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.Typography
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp

// Infinite Minimalism — deep black, warm gold accent
private val NousColors = darkColorScheme(
    primary = Color(0xFFD4AF37),         // warm gold
    onPrimary = Color.Black,
    primaryContainer = Color(0xFF1A1400),
    secondary = Color(0xFFA3A3A3),       // muted grey
    onSecondary = Color.Black,
    background = Color.Black,
    onBackground = Color(0xFFFAFAFA),
    surface = Color(0xFF0A0A0A),
    onSurface = Color(0xFFFAFAFA),
    surfaceVariant = Color(0xFF111111),
    onSurfaceVariant = Color(0xFF737373),
    outline = Color(0xFF1A1A1A),
    outlineVariant = Color(0xFF111111),
    error = Color(0xFFEF4444),
)

private val NousTypography = Typography(
    headlineLarge = TextStyle(
        fontWeight = FontWeight.ExtraLight,
        fontSize = 28.sp,
        letterSpacing = (-0.02).sp,
    ),
    headlineMedium = TextStyle(
        fontWeight = FontWeight.Light,
        fontSize = 22.sp,
        letterSpacing = (-0.02).sp,
    ),
    titleLarge = TextStyle(
        fontWeight = FontWeight.Light,
        fontSize = 18.sp,
        letterSpacing = (-0.01).sp,
    ),
    titleMedium = TextStyle(
        fontWeight = FontWeight.Normal,
        fontSize = 14.sp,
        letterSpacing = 0.08.sp,
    ),
    bodyLarge = TextStyle(
        fontWeight = FontWeight.Light,
        fontSize = 14.sp,
        lineHeight = 22.sp,
    ),
    bodyMedium = TextStyle(
        fontWeight = FontWeight.Light,
        fontSize = 13.sp,
        lineHeight = 20.sp,
    ),
    labelSmall = TextStyle(
        fontWeight = FontWeight.Normal,
        fontSize = 11.sp,
        letterSpacing = 0.08.sp,
    ),
)

@Composable
fun NousTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = NousColors,
        typography = NousTypography,
        content = content,
    )
}
