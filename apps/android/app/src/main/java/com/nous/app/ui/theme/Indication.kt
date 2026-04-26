package com.nous.app.ui.theme

import androidx.compose.foundation.Indication
import androidx.compose.foundation.IndicationInstance
import androidx.compose.foundation.interaction.FocusInteraction
import androidx.compose.foundation.interaction.HoverInteraction
import androidx.compose.foundation.interaction.InteractionSource
import androidx.compose.foundation.interaction.PressInteraction
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.State
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.ContentDrawScope

// ─── Editorial press indication ───────────────────────────────────────────────
//
// Material's ripple is replaced with a flat 12% oxblood overlay that follows
// press / focus state. No expanding circle, no opacity ramp curve, no shadow.
// The overlay is painted on top of content so existing surface colors show
// through underneath.

private class OxbloodPressIndicationInstance(
    private val isPressed: State<Boolean>,
) : IndicationInstance {
    override fun ContentDrawScope.drawIndication() {
        drawContent()
        if (isPressed.value) {
            drawRect(color = Color(0xFFB23A3A).copy(alpha = 0.12f), size = size)
        }
    }
}

object OxbloodPressIndication : Indication {
    @Composable
    override fun rememberUpdatedInstance(interactionSource: InteractionSource): IndicationInstance {
        val pressed = remember { mutableStateOf(false) }
        LaunchedEffect(interactionSource) {
            interactionSource.interactions.collect { interaction ->
                when (interaction) {
                    is PressInteraction.Press -> pressed.value = true
                    is PressInteraction.Release,
                    is PressInteraction.Cancel,
                    -> pressed.value = false
                    is HoverInteraction.Enter -> pressed.value = true
                    is HoverInteraction.Exit -> pressed.value = false
                    is FocusInteraction.Focus -> pressed.value = true
                    is FocusInteraction.Unfocus -> pressed.value = false
                }
            }
        }
        return remember(interactionSource) { OxbloodPressIndicationInstance(pressed) }
    }
}
