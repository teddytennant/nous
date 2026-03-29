import Foundation
import Observation

@Observable
@MainActor
final class NousStore {
    var connected = false
    var version = "offline"
    var uptimeMs = 0
    var did = "not initialized"

    var identity: IdentityResponse?
    var balances: [BalanceEntry] = []
    var transactions: [TransactionResponse] = []
    var daos: [DaoResponse] = []
    var proposals: [ProposalResponse] = []
    var feedEvents: [FeedEvent] = []
    var channels: [ChannelResponse] = []

    var loading = true

    private let api = NousAPI.shared

    func refresh() async {
        await loadHealth()
        await loadIdentity()
        await loadWallet()
        await loadTransactions()
        await loadGovernance()
        await loadFeed()
        await loadChannels()
        loading = false
    }

    func refreshWallet() async {
        await loadWallet()
        await loadTransactions()
    }

    func refreshFeed() async {
        await loadFeed()
    }

    func refreshChannels() async {
        await loadChannels()
    }

    private func loadHealth() async {
        do {
            let health = try await api.health()
            connected = true
            version = health.version
            uptimeMs = health.uptimeMs
        } catch {
            connected = false
        }
    }

    private func loadIdentity() async {
        do {
            let id = try await api.createIdentity(displayName: "Nous iOS")
            identity = id
            did = id.did
        } catch {
            // Offline
        }
    }

    private func loadWallet() async {
        guard let did = identity?.did else { return }
        do {
            let wallet = try await api.getWallet(did: did)
            balances = wallet.balances
        } catch {
            // Offline
        }
    }

    private func loadTransactions() async {
        guard let did = identity?.did else { return }
        do {
            transactions = try await api.getTransactions(did: did)
        } catch {
            // Offline
        }
    }

    private func loadGovernance() async {
        do {
            let daoList = try await api.listDaos()
            daos = daoList.daos
            let propList = try await api.listProposals()
            proposals = propList.proposals
        } catch {
            // Offline
        }
    }

    private func loadFeed() async {
        do {
            let feed = try await api.getFeed()
            feedEvents = feed.events
        } catch {
            // Offline
        }
    }

    private func loadChannels() async {
        guard let did = identity?.did else { return }
        do {
            channels = try await api.listChannels(did: did)
        } catch {
            // Offline
        }
    }

    // MARK: - Actions

    func sendTransfer(toDid: String, token: String, amount: Int, memo: String?) async {
        guard let fromDid = identity?.did else { return }
        let request = TransferRequest(
            fromDid: fromDid,
            toDid: toDid,
            token: token,
            amount: amount,
            memo: memo
        )
        do {
            let tx = try await api.createTransfer(request: request)
            transactions.insert(tx, at: 0)
            await loadWallet()
        } catch {
            // Handle error
        }
    }

    func publishPost(content: String, hashtags: [String]?) async {
        guard let authorDid = identity?.did else { return }
        let request = CreatePostRequest(
            authorDid: authorDid,
            content: content,
            replyTo: nil,
            hashtags: hashtags
        )
        do {
            let event = try await api.createPost(request: request)
            feedEvents.insert(event, at: 0)
        } catch {
            // Handle error
        }
    }

    func sendMessage(channelId: String, content: String) async -> MessageResponse? {
        guard let senderDid = identity?.did else { return nil }
        let request = SendMessageRequest(
            channelId: channelId,
            senderDid: senderDid,
            content: content,
            replyTo: nil
        )
        do {
            return try await api.sendMessage(request: request)
        } catch {
            return nil
        }
    }

    func getMessages(channelId: String) async -> [MessageResponse] {
        do {
            return try await api.getChannelMessages(channelId: channelId)
        } catch {
            return []
        }
    }
}
