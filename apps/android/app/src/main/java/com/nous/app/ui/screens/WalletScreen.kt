package com.nous.app.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
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
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.nous.app.data.NousViewModel
import com.nous.app.ui.components.EditorialRow
import com.nous.app.ui.components.EmptyState
import com.nous.app.ui.components.Hairline
import com.nous.app.ui.components.MetaLabel
import com.nous.app.ui.theme.NousMono
import com.nous.app.ui.theme.NousMonoLarge
import com.nous.app.ui.theme.NousSpacing

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
            .padding(horizontal = NousSpacing.gutter),
    ) {
        item {
            Spacer(Modifier.height(NousSpacing.xl))
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column {
                    Text(
                        text = "Wallet",
                        style = MaterialTheme.typography.displayMedium,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                    Spacer(Modifier.height(8.dp))
                    MetaLabel(text = "Multi-chain · Escrow-backed")
                }
                Text(
                    text = "Send",
                    style = MaterialTheme.typography.labelLarge,
                    color = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.clickable { showSendDialog = true },
                )
            }
            Spacer(Modifier.height(NousSpacing.xxl))
        }

        // Featured balance
        item {
            val featured = walletState.balances.firstOrNull()
            val token = featured?.token ?: "NOUS"
            val amount = featured?.amount ?: "0.000"
            Text(
                text = amount,
                style = NousMonoLarge.copy(fontSize = 56.sp),
                color = MaterialTheme.colorScheme.onSurface,
            )
            Spacer(Modifier.height(4.dp))
            MetaLabel(text = token, active = true)
            Spacer(Modifier.height(NousSpacing.xl))
            Hairline()

            // Other balances
            walletState.balances.drop(1).forEach { balance ->
                EditorialRow(
                    trailing = {
                        Text(
                            text = balance.amount,
                            style = NousMono,
                            color = MaterialTheme.colorScheme.onSurface,
                        )
                    },
                ) {
                    MetaLabel(text = balance.token)
                }
                Hairline()
            }
            Spacer(Modifier.height(NousSpacing.xxl))
        }

        item {
            MetaLabel(text = "Ledger")
            Spacer(Modifier.height(NousSpacing.md))
            Hairline()
        }

        if (txState.loading) {
            item {
                LinearProgressIndicator(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(top = 12.dp),
                    color = MaterialTheme.colorScheme.primary,
                    trackColor = MaterialTheme.colorScheme.outline,
                )
            }
        } else if (txState.transactions.isEmpty()) {
            item { EmptyState(headline = "No transactions yet", body = "Sent and received tokens will appear here.") }
        } else {
            items(txState.transactions) { tx ->
                EditorialRow(
                    trailing = {
                        Text(
                            text = "${tx.amount} ${tx.token}",
                            style = NousMono,
                            color = MaterialTheme.colorScheme.onSurface,
                        )
                    },
                ) {
                    Column {
                        MetaLabel(text = tx.created_at.take(10))
                        Spacer(Modifier.height(4.dp))
                        val counter = if (tx.to_did.length > 24) "${tx.to_did.take(20)}…" else tx.to_did
                        Text(
                            text = counter,
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurface,
                        )
                    }
                }
                Hairline()
            }
        }

        item { Spacer(Modifier.height(NousSpacing.xl)) }
    }
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
        focusedBorderColor = MaterialTheme.colorScheme.primary,
        unfocusedBorderColor = MaterialTheme.colorScheme.outline,
        focusedTextColor = MaterialTheme.colorScheme.onSurface,
        unfocusedTextColor = MaterialTheme.colorScheme.onSurface,
        cursorColor = MaterialTheme.colorScheme.primary,
    )

    AlertDialog(
        onDismissRequest = onDismiss,
        containerColor = MaterialTheme.colorScheme.surface,
        shape = RoundedCornerShape(0.dp),
        title = {
            Text(
                "Send Tokens",
                style = MaterialTheme.typography.headlineSmall,
                color = MaterialTheme.colorScheme.onSurface,
            )
        },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                OutlinedTextField(
                    value = toDid, onValueChange = { toDid = it },
                    label = { Text("Recipient DID") }, singleLine = true,
                    modifier = Modifier.fillMaxWidth(), colors = fieldColors,
                    shape = RoundedCornerShape(0.dp),
                )
                Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                    OutlinedTextField(
                        value = amount, onValueChange = { amount = it },
                        label = { Text("Amount") }, singleLine = true,
                        modifier = Modifier.weight(1f), colors = fieldColors,
                        shape = RoundedCornerShape(0.dp),
                    )
                    OutlinedTextField(
                        value = token, onValueChange = { token = it },
                        label = { Text("Token") }, singleLine = true,
                        modifier = Modifier.width(100.dp), colors = fieldColors,
                        shape = RoundedCornerShape(0.dp),
                    )
                }
                OutlinedTextField(
                    value = memo, onValueChange = { memo = it },
                    label = { Text("Memo (optional)") }, singleLine = true,
                    modifier = Modifier.fillMaxWidth(), colors = fieldColors,
                    shape = RoundedCornerShape(0.dp),
                )
            }
        },
        confirmButton = {
            TextButton(onClick = {
                if (toDid.isNotBlank() && amount.isNotBlank()) {
                    onSend(toDid, token, amount, memo.ifBlank { null })
                }
            }) { Text("Send", color = MaterialTheme.colorScheme.primary) }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text("Cancel", color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        },
    )
}
