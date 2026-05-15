package com.nous.app.ui.screens

import androidx.compose.foundation.clickable
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
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.nous.app.data.NousViewModel
import com.nous.app.ui.components.EmptyState
import com.nous.app.ui.components.Hairline
import com.nous.app.ui.components.MetaLabel
import com.nous.app.ui.components.OxbloodMark
import com.nous.app.ui.theme.NousSpacing

data class ChatMessage(val role: String, val content: String)

@Composable
fun AIScreen(viewModel: NousViewModel) {
    val node by viewModel.node.collectAsState()
    var input by remember { mutableStateOf("") }
    var messages by remember { mutableStateOf(listOf<ChatMessage>()) }
    var loading by remember { mutableStateOf(false) }
    val listState = rememberLazyListState()

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(horizontal = NousSpacing.gutter, vertical = NousSpacing.xl),
    ) {
        Text(
            text = "AI",
            style = MaterialTheme.typography.displayMedium,
            color = MaterialTheme.colorScheme.onSurface,
        )
        Spacer(Modifier.height(8.dp))
        MetaLabel(text = "Local inference · Private by default")
        Spacer(Modifier.height(NousSpacing.xl))

        if (!node.connected) {
            Box(modifier = Modifier.fillMaxWidth().weight(1f), contentAlignment = Alignment.Center) {
                EmptyState(
                    headline = "API offline",
                    body = "Start the API server to use AI features.",
                )
            }
        } else {
            Hairline()
            LazyColumn(
                state = listState,
                modifier = Modifier.weight(1f).fillMaxWidth(),
            ) {
                if (messages.isEmpty()) {
                    item {
                        EmptyState(
                            headline = "Start a conversation",
                            body = "Ask anything — runs locally on your node.",
                        )
                    }
                }

                items(messages) { msg ->
                    val isUser = msg.role == "user"
                    Column(modifier = Modifier.fillMaxWidth().padding(vertical = NousSpacing.md)) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            MetaLabel(
                                text = if (isUser) "You" else "Assistant",
                                active = !isUser,
                            )
                            if (isUser) {
                                Spacer(Modifier.size(6.dp))
                                OxbloodMark(filled = true)
                            }
                        }
                        Spacer(Modifier.height(6.dp))
                        Text(
                            text = msg.content,
                            style = MaterialTheme.typography.bodyLarge,
                            color = MaterialTheme.colorScheme.onSurface,
                        )
                    }
                    Hairline()
                }

                if (loading) {
                    item {
                        Row(modifier = Modifier.padding(vertical = NousSpacing.md)) {
                            CircularProgressIndicator(
                                modifier = Modifier.size(14.dp),
                                strokeWidth = 1.dp,
                                color = MaterialTheme.colorScheme.primary,
                            )
                            Spacer(Modifier.size(8.dp))
                            MetaLabel(text = "Thinking…")
                        }
                    }
                }
            }

            Hairline()
            Spacer(Modifier.height(NousSpacing.md))

            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.Bottom,
                horizontalArrangement = Arrangement.spacedBy(NousSpacing.sm),
            ) {
                OutlinedTextField(
                    value = input,
                    onValueChange = { input = it },
                    modifier = Modifier.weight(1f),
                    placeholder = { Text("Ask something…", color = MaterialTheme.colorScheme.onSurfaceVariant) },
                    colors = OutlinedTextFieldDefaults.colors(
                        focusedBorderColor = MaterialTheme.colorScheme.primary,
                        unfocusedBorderColor = MaterialTheme.colorScheme.outline,
                    ),
                    shape = RoundedCornerShape(0.dp),
                    maxLines = 4,
                )
                Text(
                    text = "Send",
                    style = MaterialTheme.typography.labelLarge,
                    color = if (input.isBlank() || loading)
                        MaterialTheme.colorScheme.onSurfaceVariant
                    else
                        MaterialTheme.colorScheme.primary,
                    modifier = Modifier
                        .padding(bottom = 16.dp)
                        .clickable(enabled = input.isNotBlank() && !loading) {
                            val userMsg = ChatMessage("user", input.trim())
                            messages = messages + userMsg
                            input = ""
                            loading = true
                            messages = messages + ChatMessage(
                                "assistant",
                                "AI inference is running locally. Connect to the API for full responses.",
                            )
                            loading = false
                        },
                )
            }
        }
    }
}
