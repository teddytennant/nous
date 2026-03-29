package com.nous.app.ui.screens

import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
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
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
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
import com.nous.app.data.FeedEvent
import com.nous.app.data.NousViewModel

private val Gold = Color(0xFFD4AF37)
private val TextPrimary = Color(0xFFFAFAFA)
private val TextSecondary = Color(0xFF737373)
private val SurfaceColor = Color(0xFF0A0A0A)
private val BorderColor = Color(0xFF1A1A1A)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SocialScreen(viewModel: NousViewModel = viewModel()) {
    val socialState by viewModel.social.collectAsState()
    var postContent by remember { mutableStateOf("") }
    var isRefreshing by remember { mutableStateOf(false) }

    PullToRefreshBox(
        isRefreshing = isRefreshing,
        onRefresh = {
            isRefreshing = true
            viewModel.refreshSocial()
            isRefreshing = false
        },
        modifier = Modifier.fillMaxSize(),
    ) {
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(horizontal = 24.dp),
        ) {
            item {
                Spacer(modifier = Modifier.height(24.dp))

                Text(
                    text = "Social",
                    style = MaterialTheme.typography.headlineLarge,
                    color = TextPrimary,
                    modifier = Modifier.padding(bottom = 4.dp),
                )
                Text(
                    text = "Decentralized feed",
                    style = MaterialTheme.typography.bodyMedium,
                    color = TextSecondary,
                    modifier = Modifier.padding(bottom = 24.dp),
                )
            }

            // Compose area
            item {
                ComposeArea(
                    value = postContent,
                    onValueChange = { postContent = it },
                    onPost = {
                        if (postContent.isNotBlank()) {
                            val tags = extractHashtags(postContent)
                            viewModel.createPost(postContent.trim(), tags)
                            postContent = ""
                        }
                    },
                )
                Spacer(modifier = Modifier.height(32.dp))
            }

            // Feed header
            item {
                Text(
                    text = "FEED",
                    style = MaterialTheme.typography.labelSmall,
                    color = TextSecondary,
                    modifier = Modifier.padding(bottom = 12.dp),
                )
            }

            if (socialState.loading) {
                item {
                    LinearProgressIndicator(
                        modifier = Modifier.fillMaxWidth(),
                        color = Gold,
                        trackColor = BorderColor,
                    )
                }
            } else if (socialState.events.isEmpty()) {
                item {
                    Surface(
                        modifier = Modifier
                            .fillMaxWidth()
                            .border(1.dp, BorderColor, RoundedCornerShape(0.dp)),
                        color = SurfaceColor,
                        shape = RoundedCornerShape(0.dp),
                    ) {
                        Column(
                            modifier = Modifier.padding(24.dp),
                            horizontalAlignment = Alignment.CenterHorizontally,
                        ) {
                            Text(
                                text = "No posts yet.",
                                style = MaterialTheme.typography.bodyMedium,
                                color = TextSecondary,
                            )
                            Spacer(modifier = Modifier.height(4.dp))
                            Text(
                                text = "Be the first to post on the sovereign web.",
                                style = MaterialTheme.typography.labelSmall,
                                color = TextSecondary,
                            )
                        }
                    }
                }
            } else {
                items(socialState.events) { event ->
                    PostCard(event = event)
                    Spacer(modifier = Modifier.height(8.dp))
                }
            }

            item { Spacer(modifier = Modifier.height(24.dp)) }
        }
    }
}

@Composable
private fun ComposeArea(
    value: String,
    onValueChange: (String) -> Unit,
    onPost: () -> Unit,
) {
    Surface(
        modifier = Modifier
            .fillMaxWidth()
            .border(1.dp, BorderColor, RoundedCornerShape(0.dp)),
        color = SurfaceColor,
        shape = RoundedCornerShape(0.dp),
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            OutlinedTextField(
                value = value,
                onValueChange = onValueChange,
                placeholder = {
                    Text(
                        "What's on your mind?",
                        color = TextSecondary,
                        style = MaterialTheme.typography.bodyMedium,
                    )
                },
                modifier = Modifier.fillMaxWidth(),
                minLines = 3,
                maxLines = 6,
                colors = OutlinedTextFieldDefaults.colors(
                    focusedBorderColor = Gold,
                    unfocusedBorderColor = BorderColor,
                    focusedTextColor = TextPrimary,
                    unfocusedTextColor = TextPrimary,
                    cursorColor = Gold,
                ),
                shape = RoundedCornerShape(0.dp),
            )

            Spacer(modifier = Modifier.height(12.dp))

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                // Show detected hashtags
                val tags = extractHashtags(value)
                if (tags.isNotEmpty()) {
                    Row(
                        horizontalArrangement = Arrangement.spacedBy(6.dp),
                        modifier = Modifier.weight(1f),
                    ) {
                        tags.take(3).forEach { tag ->
                            Text(
                                text = "#$tag",
                                fontSize = 11.sp,
                                color = Gold,
                                fontFamily = FontFamily.Monospace,
                            )
                        }
                        if (tags.size > 3) {
                            Text(
                                text = "+${tags.size - 3}",
                                fontSize = 11.sp,
                                color = TextSecondary,
                            )
                        }
                    }
                } else {
                    Spacer(modifier = Modifier.weight(1f))
                }

                Button(
                    onClick = onPost,
                    enabled = value.isNotBlank(),
                    colors = ButtonDefaults.buttonColors(
                        containerColor = Gold,
                        contentColor = Color.Black,
                        disabledContainerColor = BorderColor,
                        disabledContentColor = TextSecondary,
                    ),
                    shape = RoundedCornerShape(0.dp),
                ) {
                    Text("Post", fontWeight = FontWeight.Normal)
                }
            }
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun PostCard(event: FeedEvent) {
    val tags = event.tags
        .filter { it.isNotEmpty() && it[0] == "t" }
        .mapNotNull { it.getOrNull(1) }

    Surface(
        modifier = Modifier
            .fillMaxWidth()
            .border(1.dp, BorderColor, RoundedCornerShape(0.dp)),
        color = SurfaceColor,
        shape = RoundedCornerShape(0.dp),
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            // Author and timestamp
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                val authorDisplay = if (event.pubkey.length > 16) {
                    "${event.pubkey.take(12)}..."
                } else {
                    event.pubkey
                }
                Text(
                    text = authorDisplay,
                    fontFamily = FontFamily.Monospace,
                    fontSize = 11.sp,
                    color = Gold,
                )
                Text(
                    text = formatTimestamp(event.created_at),
                    style = MaterialTheme.typography.labelSmall,
                    color = TextSecondary,
                )
            }

            Spacer(modifier = Modifier.height(10.dp))

            // Content
            Text(
                text = event.content,
                style = MaterialTheme.typography.bodyMedium,
                color = TextPrimary,
                lineHeight = 20.sp,
            )

            // Hashtags
            if (tags.isNotEmpty()) {
                Spacer(modifier = Modifier.height(12.dp))
                HorizontalDivider(color = BorderColor, thickness = 1.dp)
                Spacer(modifier = Modifier.height(8.dp))

                FlowRow(
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                    verticalArrangement = Arrangement.spacedBy(4.dp),
                ) {
                    tags.forEach { tag ->
                        Surface(
                            modifier = Modifier
                                .border(1.dp, Gold.copy(alpha = 0.3f), RoundedCornerShape(0.dp)),
                            color = Gold.copy(alpha = 0.08f),
                            shape = RoundedCornerShape(0.dp),
                        ) {
                            Text(
                                text = "#$tag",
                                fontSize = 11.sp,
                                fontFamily = FontFamily.Monospace,
                                color = Gold,
                                modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
                            )
                        }
                    }
                }
            }

            // Kind indicator
            Spacer(modifier = Modifier.height(8.dp))
            Text(
                text = "kind:${event.kind}",
                fontSize = 10.sp,
                fontFamily = FontFamily.Monospace,
                color = TextSecondary,
            )
        }
    }
}

private fun extractHashtags(text: String): List<String> {
    val regex = Regex("#(\\w+)")
    return regex.findAll(text).map { it.groupValues[1] }.toList().distinct()
}

private fun formatTimestamp(timestamp: String): String {
    // Show date portion only for compact display
    return if (timestamp.length >= 10) timestamp.take(10) else timestamp
}
