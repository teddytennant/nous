package com.nous.app.data

import io.ktor.client.HttpClient
import io.ktor.client.call.body
import io.ktor.client.engine.okhttp.OkHttp
import io.ktor.client.plugins.contentnegotiation.ContentNegotiation
import io.ktor.client.request.get
import io.ktor.client.request.post
import io.ktor.client.request.setBody
import io.ktor.http.ContentType
import io.ktor.http.contentType
import io.ktor.serialization.kotlinx.json.json
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json

@Serializable
data class HealthResponse(
    val status: String,
    val version: String,
    val uptime_ms: Long,
)

@Serializable
data class IdentityResponse(
    val did: String,
    val display_name: String? = null,
    val signing_key_type: String,
    val exchange_key_type: String,
)

@Serializable
data class BalanceEntry(
    val token: String,
    val amount: String,
)

@Serializable
data class WalletResponse(
    val did: String,
    val balances: List<BalanceEntry>,
    val nonce: Int,
    val created_at: String,
)

@Serializable
data class FeedEvent(
    val id: String,
    val pubkey: String,
    val created_at: String,
    val kind: Int,
    val content: String,
    val tags: List<List<String>>,
)

@Serializable
data class FeedResponse(
    val events: List<FeedEvent>,
    val count: Int,
)

@Serializable
data class DaoResponse(
    val id: String,
    val name: String,
    val description: String,
    val founder_did: String,
    val member_count: Int,
    val created_at: String,
)

@Serializable
data class DaoListResponse(
    val daos: List<DaoResponse>,
    val count: Int,
)

@Serializable
data class ProposalResponse(
    val id: String,
    val dao_id: String,
    val title: String,
    val description: String,
    val proposer_did: String,
    val status: String,
    val created_at: String,
    val voting_starts: String,
    val voting_ends: String,
    val quorum: Double,
    val threshold: Double,
)

@Serializable
data class ProposalListResponse(
    val proposals: List<ProposalResponse>,
    val count: Int,
)

@Serializable
data class CreateIdentityRequest(
    val display_name: String? = null,
)

class NousApi(private val baseUrl: String = "http://10.0.2.2:8080/api/v1") {

    private val client = HttpClient(OkHttp) {
        install(ContentNegotiation) {
            json(Json {
                ignoreUnknownKeys = true
                isLenient = true
            })
        }
    }

    suspend fun health(): HealthResponse =
        client.get("$baseUrl/health").body()

    suspend fun createIdentity(displayName: String? = null): IdentityResponse =
        client.post("$baseUrl/identities") {
            contentType(ContentType.Application.Json)
            setBody(CreateIdentityRequest(displayName))
        }.body()

    suspend fun getIdentity(did: String): IdentityResponse =
        client.get("$baseUrl/identities/$did").body()

    suspend fun getWallet(did: String): WalletResponse =
        client.get("$baseUrl/wallets/$did").body()

    suspend fun getFeed(limit: Int = 20): FeedResponse =
        client.get("$baseUrl/feed?limit=$limit").body()

    suspend fun listDaos(): DaoListResponse =
        client.get("$baseUrl/daos").body()

    suspend fun listProposals(): ProposalListResponse =
        client.get("$baseUrl/proposals").body()
}
