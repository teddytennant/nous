package com.nous.app.ui.screens

import androidx.compose.animation.AnimatedContent
import androidx.compose.foundation.background
import androidx.compose.foundation.border
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
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
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
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.nous.app.data.ChannelResponse
import com.nous.app.data.MessageResponse
import com.nous.app.data.NousViewModel

private val Gold = Color(0xFFD4AF37)
private val TextPrimary = Color(0xFFFAFAFA)
private val TextSecondary = Color(0xFF737373)
private val SurfaceColor = Color(0xFF0A0A0A)
private val BorderColor = Color(0xFF1A1A1A)

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
            onSend = { content ->
                viewModel.sendMessage(selectedChannel!!.id, content)
            },
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
            .padding(horizontal = 24.dp),
    ) {
        Spacer(modifier = Modifier.height(24.dp))

        Text(
            text = "Messages",
            style = MaterialTheme.typography.headlineLarge,
            color = TextPrimary,
            modifier = Modifier.padding(bottom = 4.dp),
        )
        Text(
            text = "End-to-end encrypted via Double Ratchet",
            style = MaterialTheme.typography.bodyMedium,
            color = TextSecondary,
            modifier = Modifier.padding(bottom = 24.dp),
        )

        if (loading) {
            LinearProgressIndicator(
                modifier = Modifier.fillMaxWidth(),
                color = Gold,
                trackColor = BorderColor,
            )
        } else if (channels.isEmpty()) {
            Box(
                modifier = Modifier.fillMaxSize(),
                contentAlignment = Alignment.Center,
            ) {
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Text(
                        text = "No conversations yet.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = TextSecondary,
                    )
                    Spacer(modifier = Modifier.height(8.dp))
                    Text(
                        text = "Channels will appear when peers connect.",
                        style = MaterialTheme.typography.labelSmall,
                        color = TextSecondary,
                    )
                }
            }
        } else {
            LazyColumn(
                verticalArrangement = Arrangement.spacedBy(0.dp),
            ) {
                items(channels) { channel ->
                    ChannelRow(
                        channel = channel,
                        onClick = { onSelectChannel(channel) },
                    )
                    HorizontalDivider(color = BorderColor, thickness = 1.dp)
                }
            }
        }
    }
}

@Composable
private fun ChannelRow(
    channel: ChannelResponse,
    onClick: () -> Unit,
) {
    Surface(
        modifier = Modifier
            .fillMaxWidth()
            .clickable(onClick = onClick),
        color = Color.Transparent,
    ) {
        Row(
            modifier = Modifier
                .padding(vertical = 16.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            // Channel type indicator
            Box(
                modifier = Modifier
                    .size(36.dp)
                    .border(1.dp, BorderColor, RoundedCornerShape(0.dp))
                    .background(SurfaceColor),
                contentAlignment = Alignment.Center,
            ) {
                Text(
                    text = when (channel.channel_type.lowercase()) {
                        "dm" -> "DM"
                        "group" -> "GR"
                        "public" -> "PB"
                        else -> "CH"
                    },
                    fontSize = 10.sp,
                    fontFamily = FontFamily.Monospace,
                    fontWeight = FontWeight.Normal,
                    color = when (channel.channel_type.lowercase()) {
                        "dm" -> Gold
                        "group" -> Color(0xFF22C55E)
                        "public" -> Color(0xFF3B82F6)
                        else -> TextSecondary
                    },
                )
            }

            Spacer(modifier = Modifier.width(12.dp))

            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = channel.name,
                    style = MaterialTheme.typography.titleMedium,
                    color = TextPrimary,
                )
                channel.last_message?.let { msg ->
                    Text(
                        text = msg,
                        style = MaterialTheme.typography.bodyMedium,
                        color = TextSecondary,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                        modifier = Modifier.padding(top = 2.dp),
                    )
                }
            }

            Column(horizontalAlignment = Alignment.End) {
                Text(
                    text = channel.channel_type.uppercase(),
                    fontSize = 10.sp,
                    letterSpacing = 0.06.sp,
                    color = TextSecondary,
                )
                Text(
                    text = "${channel.member_count} member${if (channel.member_count != 1) "s" else ""}",
                    style = MaterialTheme.typography.labelSmall,
                    color = TextSecondary,
                    modifier = Modifier.padding(top = 2.dp),
                )
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
        if (messages.isNotEmpty()) {
            listState.animateScrollToItem(messages.size - 1)
        }
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(horizontal = 24.dp),
    ) {
        Spacer(modifier = Modifier.height(24.dp))

        // Header with back button
        Row(
            verticalAlignment = Alignment.CenterVertically,
            modifier = Modifier.padding(bottom = 16.dp),
        ) {
            TextButton(
                onClick = onBack,
                colors = ButtonDefaults.textButtonColors(contentColor = Gold),
            ) {
                Text("<", fontFamily = FontFamily.Monospace, fontSize = 16.sp)
            }
            Spacer(modifier = Modifier.width(8.dp))
            Column {
                Text(
                    text = channel.name,
                    style = MaterialTheme.typography.titleLarge,
                    color = TextPrimary,
                )
                Text(
                    text = "${channel.member_count} member${if (channel.member_count != 1) "s" else ""} / ${channel.channel_type}",
                    style = MaterialTheme.typography.labelSmall,
                    color = TextSecondary,
                )
            }
        }

        HorizontalDivider(color = BorderColor, thickness = 1.dp)

        // Messages
        if (loading) {
            Box(
                modifier = Modifier
                    .weight(1f)
                    .fillMaxWidth(),
                contentAlignment = Alignment.Center,
            ) {
                LinearProgressIndicator(color = Gold, trackColor = BorderColor)
            }
        } else if (messages.isEmpty()) {
            Box(
                modifier = Modifier
                    .weight(1f)
                    .fillMaxWidth(),
                contentAlignment = Alignment.Center,
            ) {
                Text(
                    text = "No messages yet. Start the conversation.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = TextSecondary,
                )
            }
        } else {
            LazyColumn(
                state = listState,
                modifier = Modifier
                    .weight(1f)
                    .fillMaxWidth()
                    .padding(vertical = 12.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                items(messages) { message ->
                    val isOwn = message.sender_did == currentDid
                    MessageBubble(
                        message = message,
                        isOwn = isOwn,
                    )
                }
            }
        }

        // Input bar
        HorizontalDivider(color = BorderColor, thickness = 1.dp)
        Spacer(modifier = Modifier.height(12.dp))

        Row(
            verticalAlignment = Alignment.CenterVertically,
            modifier = Modifier.padding(bottom = 24.dp),
        ) {
            OutlinedTextField(
                value = inputText,
                onValueChange = { inputText = it },
                placeholder = {
                    Text(
                        "Message...",
                        color = TextSecondary,
                        style = MaterialTheme.typography.bodyMedium,
                    )
                },
                modifier = Modifier.weight(1f),
                singleLine = true,
                colors = OutlinedTextFieldDefaults.colors(
                    focusedBorderColor = Gold,
                    unfocusedBorderColor = BorderColor,
                    focusedTextColor = TextPrimary,
                    unfocusedTextColor = TextPrimary,
                    cursorColor = Gold,
                ),
                shape = RoundedCornerShape(0.dp),
            )
            Spacer(modifier = Modifier.width(8.dp))
            Button(
                onClick = {
                    if (inputText.isNotBlank()) {
                        onSend(inputText.trim())
                        inputText = ""
                    }
                },
                colors = ButtonDefaults.buttonColors(
                    containerColor = Gold,
                    contentColor = Color.Black,
                ),
                shape = RoundedCornerShape(0.dp),
            ) {
                Text("Send", fontWeight = FontWeight.Normal)
            }
        }
    }
}

@Composable
private fun MessageBubble(
    message: MessageResponse,
    isOwn: Boolean,
) {
    val alignment = if (isOwn) Alignment.CenterEnd else Alignment.CenterStart
    val bgColor = if (isOwn) Gold.copy(alpha = 0.12f) else SurfaceColor
    val borderColor = if (isOwn) Gold.copy(alpha = 0.3f) else BorderColor
    val textColor = TextPrimary

    Box(
        modifier = Modifier.fillMaxWidth(),
        contentAlignment = alignment,
    ) {
        Surface(
            modifier = Modifier
                .widthIn(max = 280.dp)
                .border(1.dp, borderColor, RoundedCornerShape(0.dp)),
            color = bgColor,
            shape = RoundedCornerShape(0.dp),
        ) {
            Column(modifier = Modifier.padding(12.dp)) {
                if (!isOwn) {
                    val senderDisplay = if (message.sender_did.length > 20) {
                        "${message.sender_did.take(16)}..."
                    } else {
                        message.sender_did
                    }
                    Text(
                        text = senderDisplay,
                        fontSize = 10.sp,
                        fontFamily = FontFamily.Monospace,
                        color = Gold,
                        modifier = Modifier.padding(bottom = 4.dp),
                    )
                }
                Text(
                    text = message.content,
                    style = MaterialTheme.typography.bodyMedium,
                    color = textColor,
                )
                Text(
                    text = message.created_at.takeLast(8).take(5),
                    fontSize = 10.sp,
                    color = TextSecondary,
                    modifier = Modifier
                        .align(Alignment.End)
                        .padding(top = 4.dp),
                )
            }
        }
    }
}
