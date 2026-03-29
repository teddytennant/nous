package com.nous.app.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.nous.app.data.NousViewModel

private val Gold = Color(0xFFD4AF37)
private val TextPrimary = Color(0xFFFAFAFA)
private val TextSecondary = Color(0xFF737373)
private val SurfaceColor = Color(0xFF0A0A0A)
private val BorderColor = Color(0xFF1A1A1A)

@Composable
fun WalletScreen(viewModel: NousViewModel = viewModel()) {
    val walletState by viewModel.wallet.collectAsState()
    val txState by viewModel.transactions.collectAsState()
    var showSendDialog by remember { mutableStateOf(false) }

    if (showSendDialog) {
        SendDialog(
            onDismiss = { showSendDialog = false },
            onSend = { toDid, token, amount, memo ->
                viewModel.sendTransaction(toDid, token, amount, memo)
                showSendDialog = false
            },
        )
    }

    LazyColumn(
        modifier = Modifier
            .fillMaxSize()
            .padding(horizontal = 24.dp),
    ) {
        item {
            Spacer(modifier = Modifier.height(24.dp))

            Text(
                text = "Wallet",
                style = MaterialTheme.typography.headlineLarge,
                color = TextPrimary,
                modifier = Modifier.padding(bottom = 4.dp),
            )
            Text(
                text = "Multi-chain, escrow-backed",
                style = MaterialTheme.typography.bodyMedium,
                color = TextSecondary,
                modifier = Modifier.padding(bottom = 24.dp),
            )
        }

        // Balance cards
        item {
            if (walletState.loading) {
                LinearProgressIndicator(
                    modifier = Modifier.fillMaxWidth(),
                    color = Gold,
                    trackColor = BorderColor,
                )
                Spacer(modifier = Modifier.height(16.dp))
            } else if (walletState.balances.isEmpty()) {
                // Show placeholder balances when offline
                val placeholders = listOf(
                    Triple("NOUS", "0.000", null),
                    Triple("ETH", "0.000", "$0.00"),
                    Triple("USDC", "0.000", "$0.00"),
                )
                Row(
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                    modifier = Modifier.padding(bottom = 24.dp),
                ) {
                    placeholders.forEach { (token, amount, usd) ->
                        BalanceCard(
                            token = token,
                            amount = amount,
                            usdValue = usd,
                            modifier = Modifier.weight(1f),
                        )
                    }
                }
            } else {
                Row(
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                    modifier = Modifier.padding(bottom = 24.dp),
                ) {
                    walletState.balances.forEach { balance ->
                        BalanceCard(
                            token = balance.token,
                            amount = balance.amount,
                            usdValue = null,
                            modifier = Modifier.weight(1f),
                        )
                    }
                }
            }
        }

        // Action buttons
        item {
            Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                Button(
                    onClick = { showSendDialog = true },
                    colors = ButtonDefaults.buttonColors(
                        containerColor = Gold,
                        contentColor = Color.Black,
                    ),
                    shape = RoundedCornerShape(0.dp),
                ) {
                    Text("Send", fontWeight = FontWeight.Normal)
                }
                OutlinedButton(
                    onClick = {},
                    colors = ButtonDefaults.outlinedButtonColors(
                        contentColor = TextPrimary,
                    ),
                    shape = RoundedCornerShape(0.dp),
                    border = ButtonDefaults.outlinedButtonBorder(true).copy(
                        brush = null,
                    ),
                ) {
                    Text("Receive")
                }
                OutlinedButton(
                    onClick = {},
                    colors = ButtonDefaults.outlinedButtonColors(
                        contentColor = TextPrimary,
                    ),
                    shape = RoundedCornerShape(0.dp),
                ) {
                    Text("Swap")
                }
            }

            Spacer(modifier = Modifier.height(32.dp))
        }

        // Transaction history header
        item {
            Text(
                text = "TRANSACTIONS",
                style = MaterialTheme.typography.labelSmall,
                color = TextSecondary,
                modifier = Modifier.padding(bottom = 12.dp),
            )
        }

        if (txState.loading) {
            item {
                LinearProgressIndicator(
                    modifier = Modifier.fillMaxWidth(),
                    color = Gold,
                    trackColor = BorderColor,
                )
            }
        } else if (txState.transactions.isEmpty()) {
            item {
                Surface(
                    modifier = Modifier
                        .fillMaxWidth()
                        .border(1.dp, BorderColor, RoundedCornerShape(0.dp)),
                    color = SurfaceColor,
                    shape = RoundedCornerShape(0.dp),
                ) {
                    Text(
                        text = "No transactions yet.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = TextSecondary,
                        modifier = Modifier.padding(24.dp),
                    )
                }
            }
        } else {
            items(txState.transactions) { tx ->
                TransactionRow(
                    fromDid = tx.from_did,
                    toDid = tx.to_did,
                    token = tx.token,
                    amount = tx.amount,
                    memo = tx.memo,
                    status = tx.status,
                    timestamp = tx.created_at,
                )
            }
        }

        item { Spacer(modifier = Modifier.height(24.dp)) }
    }
}

@Composable
private fun BalanceCard(
    token: String,
    amount: String,
    usdValue: String?,
    modifier: Modifier = Modifier,
) {
    Surface(
        modifier = modifier
            .border(1.dp, BorderColor, RoundedCornerShape(0.dp)),
        color = SurfaceColor,
        shape = RoundedCornerShape(0.dp),
    ) {
        Column(modifier = Modifier.padding(20.dp)) {
            Text(
                text = token,
                style = MaterialTheme.typography.labelSmall,
                color = if (token == "NOUS") Gold else TextSecondary,
            )
            Text(
                text = amount,
                fontFamily = FontFamily.Monospace,
                fontSize = 20.sp,
                fontWeight = FontWeight.ExtraLight,
                letterSpacing = (-0.02).sp,
                color = TextPrimary,
                modifier = Modifier.padding(top = 8.dp),
            )
            usdValue?.let { usd ->
                Text(
                    text = usd,
                    style = MaterialTheme.typography.bodyMedium,
                    color = TextSecondary,
                    modifier = Modifier.padding(top = 4.dp),
                )
            }
        }
    }
}

@Composable
private fun TransactionRow(
    fromDid: String,
    toDid: String,
    token: String,
    amount: String,
    memo: String?,
    status: String,
    timestamp: String,
) {
    Surface(
        modifier = Modifier
            .fillMaxWidth()
            .border(1.dp, BorderColor, RoundedCornerShape(0.dp)),
        color = SurfaceColor,
        shape = RoundedCornerShape(0.dp),
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = "$amount $token",
                        fontFamily = FontFamily.Monospace,
                        fontSize = 14.sp,
                        fontWeight = FontWeight.Light,
                        color = TextPrimary,
                    )
                    val toDisplay = if (toDid.length > 20) "${toDid.take(16)}..." else toDid
                    Text(
                        text = "to $toDisplay",
                        style = MaterialTheme.typography.bodyMedium,
                        color = TextSecondary,
                        modifier = Modifier.padding(top = 2.dp),
                    )
                }
                Column(horizontalAlignment = Alignment.End) {
                    Text(
                        text = status.uppercase(),
                        fontSize = 10.sp,
                        letterSpacing = 0.06.sp,
                        color = when (status.lowercase()) {
                            "confirmed" -> Color(0xFF22C55E)
                            "pending" -> Gold
                            else -> TextSecondary
                        },
                    )
                    Text(
                        text = timestamp.take(10),
                        style = MaterialTheme.typography.labelSmall,
                        color = TextSecondary,
                        modifier = Modifier.padding(top = 2.dp),
                    )
                }
            }
            memo?.let {
                if (it.isNotBlank()) {
                    Text(
                        text = it,
                        style = MaterialTheme.typography.bodyMedium,
                        color = TextSecondary,
                        modifier = Modifier.padding(top = 8.dp),
                    )
                }
            }
        }
    }

    Spacer(modifier = Modifier.height(8.dp))
}

@Composable
private fun SendDialog(
    onDismiss: () -> Unit,
    onSend: (toDid: String, token: String, amount: String, memo: String?) -> Unit,
) {
    var toDid by remember { mutableStateOf("") }
    var token by remember { mutableStateOf("NOUS") }
    var amount by remember { mutableStateOf("") }
    var memo by remember { mutableStateOf("") }

    val fieldColors = OutlinedTextFieldDefaults.colors(
        focusedBorderColor = Gold,
        unfocusedBorderColor = BorderColor,
        focusedTextColor = TextPrimary,
        unfocusedTextColor = TextPrimary,
        cursorColor = Gold,
    )

    AlertDialog(
        onDismissRequest = onDismiss,
        containerColor = SurfaceColor,
        shape = RoundedCornerShape(0.dp),
        title = {
            Text(
                "Send Tokens",
                style = MaterialTheme.typography.titleLarge,
                color = TextPrimary,
            )
        },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                OutlinedTextField(
                    value = toDid,
                    onValueChange = { toDid = it },
                    label = { Text("Recipient DID", color = TextSecondary) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                    colors = fieldColors,
                    shape = RoundedCornerShape(0.dp),
                )
                Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                    OutlinedTextField(
                        value = amount,
                        onValueChange = { amount = it },
                        label = { Text("Amount", color = TextSecondary) },
                        singleLine = true,
                        modifier = Modifier.weight(1f),
                        colors = fieldColors,
                        shape = RoundedCornerShape(0.dp),
                    )
                    OutlinedTextField(
                        value = token,
                        onValueChange = { token = it },
                        label = { Text("Token", color = TextSecondary) },
                        singleLine = true,
                        modifier = Modifier.width(100.dp),
                        colors = fieldColors,
                        shape = RoundedCornerShape(0.dp),
                    )
                }
                OutlinedTextField(
                    value = memo,
                    onValueChange = { memo = it },
                    label = { Text("Memo (optional)", color = TextSecondary) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                    colors = fieldColors,
                    shape = RoundedCornerShape(0.dp),
                )
            }
        },
        confirmButton = {
            Button(
                onClick = {
                    if (toDid.isNotBlank() && amount.isNotBlank()) {
                        onSend(toDid, token, amount, memo.ifBlank { null })
                    }
                },
                colors = ButtonDefaults.buttonColors(
                    containerColor = Gold,
                    contentColor = Color.Black,
                ),
                shape = RoundedCornerShape(0.dp),
            ) {
                Text("Send")
            }
        },
        dismissButton = {
            TextButton(
                onClick = onDismiss,
                colors = ButtonDefaults.textButtonColors(contentColor = TextSecondary),
            ) {
                Text("Cancel")
            }
        },
    )
}
