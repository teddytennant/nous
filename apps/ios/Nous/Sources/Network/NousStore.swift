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
    var daos: [DaoResponse] = []
    var proposals: [ProposalResponse] = []
    var feedEvents: [FeedEvent] = []

    var loading = true

    private let api = NousAPI.shared

    func refresh() async {
        await loadHealth()
        await loadIdentity()
        await loadWallet()
        await loadGovernance()
        await loadFeed()
        loading = false
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
}
