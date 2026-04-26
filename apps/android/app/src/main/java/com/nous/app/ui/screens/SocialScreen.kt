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
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
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
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.nous.app.data.FeedEvent
import com.nous.app.data.NousViewModel
import com.nous.app.ui.components.EmptyState
import com.nous.app.ui.components.Hairline
import com.nous.app.ui.components.KeyValue
import com.nous.app.ui.components.MetaLabel
import com.nous.app.ui.theme.NousMono
import com.nous.app.ui.theme.NousSpacing

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
                .padding(horizontal = NousSpacing.gutter),
        ) {
            item {
                Spacer(Modifier.height(NousSpacing.xl))
                Text(
                    text = "Social",
                    style = MaterialTheme.typography.displayMedium,
                    color = MaterialTheme.colorScheme.onSurface,
                )
                Spacer(Modifier.height(8.dp))
                MetaLabel(text = "Decentralized feed · Nostr")
                Spacer(Modifier.height(NousSpacing.xxl))
            }

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
                Spacer(Modifier.height(NousSpacing.xxl))
                MetaLabel(text = "Feed")
                Spacer(Modifier.height(NousSpacing.md))
                Hairline()
            }

            when {
                socialState.loading -> item {
                    LinearProgressIndicator(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(top = 12.dp),
                        color = MaterialTheme.colorScheme.primary,
                        trackColor = MaterialTheme.colorScheme.outline,
                    )
                }
                socialState.events.isEmpty() -> item {
                    EmptyState(
                        headline = "No posts yet",
                        body = "Be the first voice on the sovereign web.",
                    )
                }
                else -> items(socialState.events) { event ->
                    PostArticle(event = event)
                    Hairline()
                }
            }

            item { Spacer(Modifier.height(NousSpacing.xl)) }
        }
    }
}

@Composable
private fun ComposeArea(
    value: String,
    onValueChange: (String) -> Unit,
    onPost: () -> Unit,
) {
    Column {
        OutlinedTextField(
            value = value,
            onValueChange = onValueChange,
            placeholder = {
                Text("What's on your mind?", color = MaterialTheme.colorScheme.onSurfaceVariant)
            },
            modifier = Modifier.fillMaxWidth(),
            minLines = 3,
            maxLines = 6,
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = MaterialTheme.colorScheme.primary,
                unfocusedBorderColor = MaterialTheme.colorScheme.outline,
            ),
            shape = RoundedCornerShape(0.dp),
        )
        Spacer(Modifier.height(NousSpacing.md))
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.End,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            val tags = extractHashtags(value)
            if (tags.isNotEmpty()) {
                Text(
                    text = tags.take(4).joinToString("  ") { "#$it" },
                    style = NousMono,
                    color = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.weight(1f),
                )
            }
            Text(
                text = "Publish",
                style = MaterialTheme.typography.labelLarge,
                color = if (value.isBlank())
                    MaterialTheme.colorScheme.onSurfaceVariant
                else
                    MaterialTheme.colorScheme.primary,
                modifier = Modifier.clickable(enabled = value.isNotBlank()) { onPost() },
            )
        }
    }
}

@Composable
private fun PostArticle(event: FeedEvent) {
    val tags = event.tags
        .filter { it.isNotEmpty() && it[0] == "t" }
        .mapNotNull { it.getOrNull(1) }

    val firstSentence = event.content.split('.', '!', '?', '\n')
        .firstOrNull { it.isNotBlank() }
        ?.trim()
        ?: event.content.take(60)
    val rest = event.content.removePrefix(firstSentence).trimStart('.', '!', '?', ' ', '\n')

    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = NousSpacing.lg),
    ) {
        Text(
            text = firstSentence,
            style = MaterialTheme.typography.headlineSmall,
            color = MaterialTheme.colorScheme.onSurface,
        )
        if (rest.isNotBlank()) {
            Spacer(Modifier.height(8.dp))
            Text(
                text = rest,
                style = MaterialTheme.typography.bodyLarge,
                color = MaterialTheme.colorScheme.onSurface,
            )
        }

        Spacer(Modifier.height(NousSpacing.md))

        // Author + timestamp meta
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
        ) {
            val author = if (event.pubkey.length > 18) "${event.pubkey.take(14)}…" else event.pubkey
            MetaLabel(text = author)
            MetaLabel(text = formatTimestamp(event.created_at))
        }

        if (tags.isNotEmpty()) {
            Spacer(Modifier.height(8.dp))
            Text(
                text = tags.joinToString("  ") { "#$it" },
                style = NousMono,
                color = MaterialTheme.colorScheme.onSurface,
            )
        }

        Spacer(Modifier.height(NousSpacing.md))

        // Engagement metrics
        Row(modifier = Modifier.fillMaxWidth()) {
            EngagementCell(label = "Kind", value = "${event.kind}", modifier = Modifier.weight(1f))
            EngagementCell(label = "Replies", value = "0", modifier = Modifier.weight(1f))
            EngagementCell(label = "Boosts", value = "0", modifier = Modifier.weight(1f))
        }
    }
}

@Composable
private fun EngagementCell(label: String, value: String, modifier: Modifier = Modifier) {
    Column(modifier = modifier) {
        MetaLabel(text = label)
        Spacer(Modifier.height(2.dp))
        Text(
            text = value,
            style = NousMono,
            color = MaterialTheme.colorScheme.onSurface,
        )
    }
}

private fun extractHashtags(text: String): List<String> {
    val regex = Regex("#(\\w+)")
    return regex.findAll(text).map { it.groupValues[1] }.toList().distinct()
}

private fun formatTimestamp(timestamp: String): String {
    return if (timestamp.length >= 10) timestamp.take(10) else timestamp
}
