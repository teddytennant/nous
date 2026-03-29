import Foundation

// MARK: - Response Types

struct HealthResponse: Codable {
    let status: String
    let version: String
    let uptimeMs: Int

    enum CodingKeys: String, CodingKey {
        case status, version
        case uptimeMs = "uptime_ms"
    }
}

struct IdentityResponse: Codable, Identifiable {
    var id: String { did }
    let did: String
    let displayName: String?
    let signingKeyType: String
    let exchangeKeyType: String

    enum CodingKeys: String, CodingKey {
        case did
        case displayName = "display_name"
        case signingKeyType = "signing_key_type"
        case exchangeKeyType = "exchange_key_type"
    }
}

struct BalanceEntry: Codable, Identifiable {
    var id: String { token }
    let token: String
    let amount: String
}

struct WalletResponse: Codable {
    let did: String
    let balances: [BalanceEntry]
    let nonce: Int
    let createdAt: String

    enum CodingKeys: String, CodingKey {
        case did, balances, nonce
        case createdAt = "created_at"
    }
}

struct TransactionResponse: Codable, Identifiable {
    let id: String
    let fromDid: String
    let toDid: String
    let token: String
    let amount: String
    let fee: String
    let memo: String?
    let status: String
    let timestamp: String

    enum CodingKeys: String, CodingKey {
        case id, token, amount, fee, memo, status, timestamp
        case fromDid = "from_did"
        case toDid = "to_did"
    }
}

struct TransferRequest: Codable {
    let fromDid: String
    let toDid: String
    let token: String
    let amount: Int
    let memo: String?

    enum CodingKeys: String, CodingKey {
        case token, amount, memo
        case fromDid = "from_did"
        case toDid = "to_did"
    }
}

struct ChannelResponse: Codable, Identifiable {
    let id: String
    let kind: String
    let name: String?
    let members: [String]
    let createdAt: String

    enum CodingKeys: String, CodingKey {
        case id, kind, name, members
        case createdAt = "created_at"
    }
}

struct MessageResponse: Codable, Identifiable {
    let id: String
    let channelId: String
    let sender: String
    let content: String
    let replyTo: String?
    let timestamp: String

    enum CodingKeys: String, CodingKey {
        case id, sender, content, timestamp
        case channelId = "channel_id"
        case replyTo = "reply_to"
    }
}

struct SendMessageRequest: Codable {
    let channelId: String
    let senderDid: String
    let content: String
    let replyTo: String?

    enum CodingKeys: String, CodingKey {
        case content
        case channelId = "channel_id"
        case senderDid = "sender_did"
        case replyTo = "reply_to"
    }
}

struct CreatePostRequest: Codable {
    let authorDid: String
    let content: String
    let replyTo: String?
    let hashtags: [String]?

    enum CodingKeys: String, CodingKey {
        case content, hashtags
        case authorDid = "author_did"
        case replyTo = "reply_to"
    }
}

struct FeedEvent: Codable, Identifiable {
    let id: String
    let pubkey: String
    let createdAt: String
    let kind: Int
    let content: String
    let tags: [[String]]

    enum CodingKeys: String, CodingKey {
        case id, pubkey, kind, content, tags
        case createdAt = "created_at"
    }
}

struct FeedResponse: Codable {
    let events: [FeedEvent]
    let count: Int
}

struct DaoResponse: Codable, Identifiable {
    let id: String
    let name: String
    let description: String
    let founderDid: String
    let memberCount: Int
    let createdAt: String

    enum CodingKeys: String, CodingKey {
        case id, name, description
        case founderDid = "founder_did"
        case memberCount = "member_count"
        case createdAt = "created_at"
    }
}

struct DaoListResponse: Codable {
    let daos: [DaoResponse]
    let count: Int
}

struct ProposalResponse: Codable, Identifiable {
    let id: String
    let daoId: String
    let title: String
    let description: String
    let proposerDid: String
    let status: String
    let createdAt: String
    let votingStarts: String
    let votingEnds: String
    let quorum: Double
    let threshold: Double

    enum CodingKeys: String, CodingKey {
        case id, title, description, status, quorum, threshold
        case daoId = "dao_id"
        case proposerDid = "proposer_did"
        case createdAt = "created_at"
        case votingStarts = "voting_starts"
        case votingEnds = "voting_ends"
    }
}

struct ProposalListResponse: Codable {
    let proposals: [ProposalResponse]
    let count: Int
}

// MARK: - API Client

final class NousAPI: @unchecked Sendable {
    static let shared = NousAPI()

    private let baseURL: String
    private let session: URLSession
    private let decoder: JSONDecoder

    init(baseURL: String = "http://localhost:8080/api/v1") {
        self.baseURL = baseURL
        self.session = URLSession.shared
        self.decoder = JSONDecoder()
    }

    private func get<T: Decodable>(_ path: String) async throws -> T {
        guard let url = URL(string: "\(baseURL)\(path)") else {
            throw URLError(.badURL)
        }
        let (data, _) = try await session.data(from: url)
        return try decoder.decode(T.self, from: data)
    }

    private func post<T: Decodable>(_ path: String, body: some Encodable) async throws -> T {
        guard let url = URL(string: "\(baseURL)\(path)") else {
            throw URLError(.badURL)
        }
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONEncoder().encode(body)
        let (data, _) = try await session.data(for: request)
        return try decoder.decode(T.self, from: data)
    }

    // MARK: - Health

    func health() async throws -> HealthResponse {
        try await get("/health")
    }

    // MARK: - Identity

    func createIdentity(displayName: String? = nil) async throws -> IdentityResponse {
        struct Req: Encodable {
            let display_name: String?
        }
        return try await post("/identities", body: Req(display_name: displayName))
    }

    func getIdentity(did: String) async throws -> IdentityResponse {
        try await get("/identities/\(did)")
    }

    // MARK: - Wallet & Payments

    func getWallet(did: String) async throws -> WalletResponse {
        try await get("/wallets/\(did)")
    }

    func getTransactions(did: String, limit: Int = 50) async throws -> [TransactionResponse] {
        try await get("/wallets/\(did)/transactions?limit=\(limit)")
    }

    func createTransfer(request: TransferRequest) async throws -> TransactionResponse {
        try await post("/transfers", body: request)
    }

    // MARK: - Social Feed

    func getFeed(limit: Int = 20) async throws -> FeedResponse {
        try await get("/feed?limit=\(limit)")
    }

    func createPost(request: CreatePostRequest) async throws -> FeedEvent {
        try await post("/events", body: request)
    }

    // MARK: - Messaging

    func listChannels(did: String) async throws -> [ChannelResponse] {
        try await get("/channels?did=\(did)")
    }

    func getChannelMessages(channelId: String, limit: Int = 50) async throws -> [MessageResponse] {
        try await get("/channels/\(channelId)/messages?limit=\(limit)")
    }

    func sendMessage(request: SendMessageRequest) async throws -> MessageResponse {
        try await post("/messages", body: request)
    }

    // MARK: - Governance

    func listDaos() async throws -> DaoListResponse {
        try await get("/daos")
    }

    func listProposals() async throws -> ProposalListResponse {
        try await get("/proposals")
    }
}
