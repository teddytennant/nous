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
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.nous.app.data.ChannelResponse
import com.nous.app.data.MessageResponse
import com.nous.app.data.NousViewModel
import com.nous.app.ui.components.EditorialRow
import com.nous.app.ui.components.EmptyState
import com.nous.app.ui.components.Hairline
import com.nous.app.ui.components.MetaLabel
import com.nous.app.ui.components.OxbloodMark
import com.nous.app.ui.theme.NousMono
import com.nous.app.ui.theme.NousSpacing

@Composable
fun MessagesScreen(viewModel: NousViewModel = viewModel()) {
    val channelState by viewModel.channels.collectAsState()
    val messageState by viewModel.messages.collectAsState()
    val identity by viewModel.identity.collectAsState()
    var selectedChannel by remember { mutableStateOf<ChannelResponse?>(null) }

    if (selectedChannel != null) {
        MessageView(
            channel = selectedChannel!!,
            messages = messageState.messages,
            loading = messageState.loading,
            currentDid = identity?.did ?: "",
            onBack = { selectedChannel = null },
            onSend = { content -> viewModel.sendMessage(selectedChannel!!.id, content) },
        )
        LaunchedEffect(selectedChannel) {
            selectedChannel?.let { viewModel.loadMessagesForChannel(it.id) }
        }
    } else {
        ChannelList(
            channels = channelState.channels,
            loading = channelState.loading,
            onSelectChannel = { selectedChannel = it },
        )
    }
}

@Composable
private fun ChannelList(
    channels: List<ChannelResponse>,
    loading: Boolean,
    onSelectChannel: (ChannelResponse) -> Unit,
) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(horizontal = NousSpacing.gutter),
    ) {
        Spacer(Modifier.height(NousSpacing.xl))
        Text(
            text = "Messages",
            style = MaterialTheme.typography.displayMedium,
            color = MaterialTheme.colorScheme.onSurface,
        )
        Spacer(Modifier.height(8.dp))
        MetaLabel(text = "End-to-end encrypted · Double Ratchet")

        Spacer(Modifier.height(NousSpacing.xxl))

        when {
            loading -> {
                LinearProgressIndicator(
                    modifier = Modifier.fillMaxWidth(),
                    color = MaterialTheme.colorScheme.primary,
                    trackColor = MaterialTheme.colorScheme.outline,
                )
            }
            channels.isEmpty() -> EmptyState(
                headline = "No conversations yet",
                body = "Channels will appear when peers connect.",
            )
            else -> {
                Hairline()
                LazyColumn {
                    items(channels) { channel ->
                        EditorialRow(
                            onClick = { onSelectChannel(channel) },
                            trailing = {
                                MetaLabel(text = channel.channel_type)
                            },
                        ) {
                            Column {
                                Text(
                                    text = channel.name,
                                    style = MaterialTheme.typography.headlineSmall,
                                    color = MaterialTheme.colorScheme.onSurface,
                                )
                                channel.last_message?.let {
                                    Text(
                                        text = it,
                                        style = MaterialTheme.typography.bodyMedium,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        maxLines = 1,
                                        overflow = TextOverflow.Ellipsis,
                                        modifier = Modifier.padding(top = 2.dp),
                                    )
                                }
                            }
                        }
                        Hairline()
                    }
                }
            }
        }
    }
}

@Composable
private fun MessageView(
    channel: ChannelResponse,
    messages: List<MessageResponse>,
    loading: Boolean,
    currentDid: String,
    onBack: () -> Unit,
    onSend: (String) -> Unit,
) {
    var inputText by remember { mutableStateOf("") }
    val listState = rememberLazyListState()

    LaunchedEffect(messages.size) {
        if (messages.isNotEmpty()) listState.animateScrollToItem(messages.size - 1)
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(horizontal = NousSpacing.gutter),
    ) {
        Spacer(Modifier.height(NousSpacing.xl))
        // Header
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(
                text = "Back",
                style = MaterialTheme.typography.labelLarge,
                color = MaterialTheme.colorScheme.primary,
                modifier = Modifier.clickable(onClick = onBack),
            )
            Spacer(Modifier.width(NousSpacing.lg))
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = channel.name,
                    style = MaterialTheme.typography.headlineSmall,
                    color = MaterialTheme.colorScheme.onSurface,
                )
                MetaLabel(text = "${channel.member_count} member${if (channel.member_count != 1) "s" else ""} · ${channel.channel_type}")
            }
        }
        Spacer(Modifier.height(NousSpacing.lg))
        Hairline()

        // Messages
        if (loading) {
            Box(modifier = Modifier.weight(1f).fillMaxWidth(), contentAlignment = Alignment.Center) {
                LinearProgressIndicator(
                    color = MaterialTheme.colorScheme.primary,
                    trackColor = MaterialTheme.colorScheme.outline,
                )
            }
        } else if (messages.isEmpty()) {
            Box(modifier = Modifier.weight(1f).fillMaxWidth(), contentAlignment = Alignment.Center) {
                EmptyState(headline = "No messages yet", body = "Start the conversation.")
            }
        } else {
            LazyColumn(
                state = listState,
                modifier = Modifier.weight(1f).fillMaxWidth(),
            ) {
                items(messages) { message ->
                    val isOwn = message.sender_did == currentDid
                    MessageBlock(message = message, isOwn = isOwn)
                    Hairline()
                }
            }
        }

        Hairline()
        // Input
        Row(
            verticalAlignment = Alignment.CenterVertically,
            modifier = Modifier
                .fillMaxWidth()
                .padding(vertical = NousSpacing.md),
        ) {
            OutlinedTextField(
                value = inputText,
                onValueChange = { inputText = it },
                placeholder = { Text("Message…", color = MaterialTheme.colorScheme.onSurfaceVariant) },
                modifier = Modifier.weight(1f),
                singleLine = true,
                colors = OutlinedTextFieldDefaults.colors(
                    focusedBorderColor = MaterialTheme.colorScheme.primary,
                    unfocusedBorderColor = MaterialTheme.colorScheme.outline,
                ),
                shape = RoundedCornerShape(0.dp),
            )
            Spacer(Modifier.width(NousSpacing.sm))
            Text(
                text = "Send",
                style = MaterialTheme.typography.labelLarge,
                color = MaterialTheme.colorScheme.primary,
                modifier = Modifier.clickable {
                    if (inputText.isNotBlank()) {
                        onSend(inputText.trim())
                        inputText = ""
                    }
                },
            )
        }
    }
}

@Composable
private fun MessageBlock(message: MessageResponse, isOwn: Boolean) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = NousSpacing.md),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            val sender = if (isOwn) "You" else {
                if (message.sender_did.length > 18) "${message.sender_did.take(14)}…"
                else message.sender_did
            }
            Text(
                text = sender,
                style = MaterialTheme.typography.headlineSmall,
                color = MaterialTheme.colorScheme.onSurface,
                modifier = Modifier.weight(1f),
            )
            if (isOwn) {
                OxbloodMark(filled = true)
                Spacer(Modifier.width(6.dp))
            }
            Text(
                text = message.created_at.takeLast(8).take(5),
                style = NousMono,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        Spacer(Modifier.height(8.dp))
        Text(
            text = message.content,
            style = MaterialTheme.typography.bodyLarge,
            color = MaterialTheme.colorScheme.onSurface,
        )
    }
}
