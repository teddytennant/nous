package com.nous.app.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.nous.app.data.NousViewModel
import com.nous.app.ui.components.EmptyState
import com.nous.app.ui.components.Hairline
import com.nous.app.ui.components.MetaLabel
import com.nous.app.ui.theme.NousMono
import com.nous.app.ui.theme.NousSpacing

data class MarketplaceListing(
    val title: String,
    val price: String,
    val seller: String,
    val category: String,
)

@Composable
fun MarketplaceScreen(viewModel: NousViewModel) {
    val node by viewModel.node.collectAsState()

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(horizontal = NousSpacing.gutter, vertical = NousSpacing.xl),
    ) {
        Text(
            text = "Marketplace",
            style = MaterialTheme.typography.displayMedium,
            color = MaterialTheme.colorScheme.onSurface,
        )
        Spacer(Modifier.height(8.dp))
        MetaLabel(text = "P2P commerce · Trustless escrow")

        Spacer(Modifier.height(NousSpacing.xxl))

        if (!node.connected) {
            Box(modifier = Modifier.fillMaxWidth().weight(1f), contentAlignment = Alignment.Center) {
                EmptyState(
                    headline = "No listings available",
                    body = "Connect to the network to browse.",
                )
            }
        } else {
            val demoListings = listOf(
                MarketplaceListing("Digital Art Pack", "0.05 ETH", "did:key:z6Mk…a3f2", "Digital"),
                MarketplaceListing("API Access Token", "50 NOUS", "did:key:z6Mk…b1c4", "Service"),
                MarketplaceListing("Encrypted Dataset", "0.1 ETH", "did:key:z6Mk…d5e6", "Data"),
            )

            Hairline()
            LazyColumn {
                items(demoListings) { listing ->
                    Column(modifier = Modifier.fillMaxWidth().padding(vertical = NousSpacing.md)) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Text(
                                text = listing.title,
                                style = MaterialTheme.typography.headlineSmall,
                                color = MaterialTheme.colorScheme.onSurface,
                            )
                            Text(
                                text = listing.price,
                                style = NousMono,
                                color = MaterialTheme.colorScheme.primary,
                            )
                        }
                        Spacer(Modifier.height(6.dp))
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                        ) {
                            Text(
                                text = listing.seller,
                                style = NousMono,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                            MetaLabel(text = listing.category)
                        }
                    }
                    Hairline()
                }
            }
        }
    }
}
