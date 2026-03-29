package com.nous.app.data

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch

data class NodeState(
    val connected: Boolean = false,
    val version: String = "offline",
    val uptimeMs: Long = 0,
    val did: String = "not initialized",
)

data class WalletState(
    val balances: List<BalanceEntry> = emptyList(),
    val loading: Boolean = true,
)

data class GovernanceState(
    val daos: List<DaoResponse> = emptyList(),
    val proposals: List<ProposalResponse> = emptyList(),
    val loading: Boolean = true,
)

data class SocialState(
    val events: List<FeedEvent> = emptyList(),
    val loading: Boolean = true,
)

class NousViewModel : ViewModel() {
    private val api = NousApi()

    private val _node = MutableStateFlow(NodeState())
    val node: StateFlow<NodeState> = _node

    private val _identity = MutableStateFlow<IdentityResponse?>(null)
    val identity: StateFlow<IdentityResponse?> = _identity

    private val _wallet = MutableStateFlow(WalletState())
    val wallet: StateFlow<WalletState> = _wallet

    private val _governance = MutableStateFlow(GovernanceState())
    val governance: StateFlow<GovernanceState> = _governance

    private val _social = MutableStateFlow(SocialState())
    val social: StateFlow<SocialState> = _social

    init {
        refresh()
    }

    fun refresh() {
        viewModelScope.launch {
            loadHealth()
            loadIdentity()
            loadWallet()
            loadGovernance()
            loadSocial()
        }
    }

    private suspend fun loadHealth() {
        try {
            val health = api.health()
            _node.value = NodeState(
                connected = true,
                version = health.version,
                uptimeMs = health.uptime_ms,
                did = _identity.value?.did ?: "loading...",
            )
        } catch (_: Exception) {
            _node.value = NodeState(connected = false)
        }
    }

    private suspend fun loadIdentity() {
        try {
            val id = api.createIdentity("Nous Android")
            _identity.value = id
            _node.value = _node.value.copy(did = id.did)
        } catch (_: Exception) {
            // Offline
        }
    }

    private suspend fun loadWallet() {
        val did = _identity.value?.did ?: return
        try {
            val wallet = api.getWallet(did)
            _wallet.value = WalletState(balances = wallet.balances, loading = false)
        } catch (_: Exception) {
            _wallet.value = WalletState(loading = false)
        }
    }

    private suspend fun loadGovernance() {
        try {
            val daos = api.listDaos()
            val proposals = api.listProposals()
            _governance.value = GovernanceState(
                daos = daos.daos,
                proposals = proposals.proposals,
                loading = false,
            )
        } catch (_: Exception) {
            _governance.value = GovernanceState(loading = false)
        }
    }

    private suspend fun loadSocial() {
        try {
            val feed = api.getFeed()
            _social.value = SocialState(events = feed.events, loading = false)
        } catch (_: Exception) {
            _social.value = SocialState(loading = false)
        }
    }
}
