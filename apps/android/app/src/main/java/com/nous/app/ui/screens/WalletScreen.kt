package com.nous.app.ui.screens

import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

data class Balance(val token: String, val amount: String, val usdValue: String?)

@Composable
fun WalletScreen() {
    val balances = listOf(
        Balance("ETH", "0.000", "$0.00"),
        Balance("NOUS", "0.000", null),
        Balance("USDC", "0.000", "$0.00"),
    )

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(24.dp),
    ) {
        Text(
            text = "Wallet",
            style = MaterialTheme.typography.headlineLarge,
            modifier = Modifier.padding(bottom = 4.dp),
        )
        Text(
            text = "Multi-chain, escrow-backed",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(bottom = 24.dp),
        )

        Row(
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            modifier = Modifier.padding(bottom = 24.dp),
        ) {
            balances.forEach { balance ->
                Surface(
                    modifier = Modifier
                        .weight(1f)
                        .border(1.dp, MaterialTheme.colorScheme.outline, RoundedCornerShape(8.dp)),
                    color = MaterialTheme.colorScheme.surface,
                    shape = RoundedCornerShape(8.dp),
                ) {
                    Column(modifier = Modifier.padding(20.dp)) {
                        Text(
                            text = balance.token,
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        Text(
                            text = balance.amount,
                            fontFamily = FontFamily.Monospace,
                            fontSize = 20.sp,
                            fontWeight = FontWeight.ExtraLight,
                            letterSpacing = (-0.02).sp,
                            modifier = Modifier.padding(top = 8.dp),
                        )
                        balance.usdValue?.let { usd ->
                            Text(
                                text = usd,
                                style = MaterialTheme.typography.bodyMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                modifier = Modifier.padding(top = 4.dp),
                            )
                        }
                    }
                }
            }
        }

        Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            OutlinedButton(
                onClick = {},
                colors = ButtonDefaults.outlinedButtonColors(
                    contentColor = MaterialTheme.colorScheme.onSurface,
                ),
            ) {
                Text("Send")
            }
            OutlinedButton(
                onClick = {},
                colors = ButtonDefaults.outlinedButtonColors(
                    contentColor = MaterialTheme.colorScheme.onSurface,
                ),
            ) {
                Text("Receive")
            }
            OutlinedButton(
                onClick = {},
                colors = ButtonDefaults.outlinedButtonColors(
                    contentColor = MaterialTheme.colorScheme.onSurface,
                ),
            ) {
                Text("Swap")
            }
        }
    }
}
