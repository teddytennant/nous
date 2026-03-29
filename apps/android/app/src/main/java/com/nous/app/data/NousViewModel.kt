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

data class TransactionState(
    val transactions: List<TransactionResponse> = emptyList(),
    val loading: Boolean = true,
)

data class ChannelState(
    val channels: List<ChannelResponse> = emptyList(),
    val loading: Boolean = true,
)

data class MessageState(
    val messages: List<MessageResponse> = emptyList(),
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

    private val _transactions = MutableStateFlow(TransactionState())
    val transactions: StateFlow<TransactionState> = _transactions

    private val _channels = MutableStateFlow(ChannelState())
    val channels: StateFlow<ChannelState> = _channels

    private val _messages = MutableStateFlow(MessageState())
    val messages: StateFlow<MessageState> = _messages

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
            loadTransactions()
            loadGovernance()
            loadSocial()
            loadChannels()
        }
    }

    fun refreshWallet() {
        viewModelScope.launch {
            _wallet.value = _wallet.value.copy(loading = true)
            loadWallet()
            loadTransactions()
        }
    }

    fun refreshSocial() {
        viewModelScope.launch {
            _social.value = _social.value.copy(loading = true)
            loadSocial()
        }
    }

    fun refreshChannels() {
        viewModelScope.launch {
            _channels.value = _channels.value.copy(loading = true)
            loadChannels()
        }
    }

    fun loadMessagesForChannel(channelId: String) {
        viewModelScope.launch {
            _messages.value = MessageState(loading = true)
            try {
                val result = api.getMessages(channelId)
                _messages.value = MessageState(messages = result.messages, loading = false)
            } catch (_: Exception) {
                _messages.value = MessageState(loading = false)
            }
        }
    }

    fun sendMessage(channelId: String, content: String) {
        viewModelScope.launch {
            try {
                api.sendMessage(channelId, SendMessageRequest(content))
                loadMessagesForChannel(channelId)
            } catch (_: Exception) {
                // Failed to send
            }
        }
    }

    fun sendTransaction(toDid: String, token: String, amount: String, memo: String?) {
        viewModelScope.launch {
            val fromDid = _identity.value?.did ?: return@launch
            try {
                api.sendTransaction(fromDid, SendTransactionRequest(toDid, token, amount, memo))
                refreshWallet()
            } catch (_: Exception) {
                // Failed to send
            }
        }
    }

    fun createPost(content: String, tags: List<String> = emptyList()) {
        viewModelScope.launch {
            try {
                api.createPost(CreatePostRequest(content, tags))
                refreshSocial()
            } catch (_: Exception) {
                // Failed to post
            }
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

    private suspend fun loadTransactions() {
        val did = _identity.value?.did ?: return
        try {
            val result = api.getTransactions(did)
            _transactions.value = TransactionState(transactions = result.transactions, loading = false)
        } catch (_: Exception) {
            _transactions.value = TransactionState(loading = false)
        }
    }

    private suspend fun loadChannels() {
        try {
            val result = api.listChannels()
            _channels.value = ChannelState(channels = result.channels, loading = false)
        } catch (_: Exception) {
            _channels.value = ChannelState(loading = false)
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
