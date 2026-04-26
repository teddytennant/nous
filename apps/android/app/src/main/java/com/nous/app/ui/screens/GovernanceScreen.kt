package com.nous.app.ui.screens

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
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.nous.app.data.NousViewModel
import com.nous.app.ui.components.EmptyState
import com.nous.app.ui.components.Hairline
import com.nous.app.ui.components.MetaLabel
import com.nous.app.ui.theme.NousMono
import com.nous.app.ui.theme.NousSpacing

@Composable
fun GovernanceScreen(viewModel: NousViewModel = viewModel()) {
    val gov by viewModel.governance.collectAsState()

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(horizontal = NousSpacing.gutter, vertical = NousSpacing.xl),
    ) {
        Text(
            text = "Governance",
            style = MaterialTheme.typography.displayMedium,
            color = MaterialTheme.colorScheme.onSurface,
        )
        Spacer(Modifier.height(8.dp))
        MetaLabel(text = "Quadratic voting · Proposals")

        Spacer(Modifier.height(NousSpacing.xxl))

        if (gov.daos.isNotEmpty()) {
            MetaLabel(text = "DAOs")
            Spacer(Modifier.height(NousSpacing.md))
            Hairline()
            gov.daos.forEach { dao ->
                Column(modifier = Modifier.fillMaxWidth().padding(vertical = NousSpacing.md)) {
                    Text(
                        text = dao.name,
                        style = MaterialTheme.typography.headlineSmall,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                    Text(
                        text = dao.description,
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(top = 4.dp),
                    )
                    Text(
                        text = "${dao.member_count} member${if (dao.member_count != 1) "s" else ""}",
                        style = NousMono,
                        color = MaterialTheme.colorScheme.primary,
                        modifier = Modifier.padding(top = 6.dp),
                    )
                }
                Hairline()
            }
            Spacer(Modifier.height(NousSpacing.xl))
        }

        MetaLabel(text = "Proposals")
        Spacer(Modifier.height(NousSpacing.md))
        Hairline()

        when {
            gov.loading -> LinearProgressIndicator(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(top = 12.dp),
                color = MaterialTheme.colorScheme.primary,
                trackColor = MaterialTheme.colorScheme.outline,
            )
            gov.proposals.isEmpty() -> EmptyState(headline = "No active proposals")
            else -> LazyColumn {
                items(gov.proposals) { proposal ->
                    Column(modifier = Modifier.fillMaxWidth().padding(vertical = NousSpacing.md)) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                        ) {
                            Text(
                                text = proposal.title,
                                style = MaterialTheme.typography.headlineSmall,
                                color = MaterialTheme.colorScheme.onSurface,
                                modifier = Modifier.weight(1f),
                            )
                            MetaLabel(
                                text = proposal.status,
                                active = proposal.status.lowercase() == "active",
                            )
                        }
                        Text(
                            text = proposal.description,
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            maxLines = 2,
                            modifier = Modifier.padding(top = 4.dp),
                        )
                        Text(
                            text = "Quorum ${(proposal.quorum * 100).toInt()}%   Threshold ${(proposal.threshold * 100).toInt()}%",
                            style = NousMono,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.padding(top = 6.dp),
                        )
                    }
                    Hairline()
                }
            }
        }
    }
}
