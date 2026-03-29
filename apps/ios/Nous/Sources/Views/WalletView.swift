import SwiftUI

struct WalletView: View {
    @Environment(NousStore.self) private var store
    @State private var showingSendSheet = false

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: NousTheme.spacingXL) {
                // Header
                VStack(alignment: .leading, spacing: NousTheme.spacingXS) {
                    Text("Wallet")
                        .font(NousTheme.headlineLarge)
                        .foregroundColor(NousTheme.text)
                    Text("Multi-chain, escrow-backed")
                        .font(NousTheme.bodySmall)
                        .foregroundColor(NousTheme.textMuted)
                }

                // Balances
                if store.balances.isEmpty {
                    VStack(alignment: .leading, spacing: NousTheme.spacingSM) {
                        Text("NO BALANCES")
                            .font(NousTheme.label)
                            .foregroundColor(NousTheme.textMuted)
                            .tracking(0.8)
                        Text("0.000")
                            .font(NousTheme.monoLarge)
                            .foregroundColor(NousTheme.text)
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(20)
                    .background(NousTheme.surface)
                    .overlay(
                        Rectangle()
                            .stroke(NousTheme.border, lineWidth: 1)
                    )
                } else {
                    ForEach(store.balances) { balance in
                        VStack(alignment: .leading, spacing: NousTheme.spacingSM) {
                            Text(balance.token.uppercased())
                                .font(NousTheme.label)
                                .foregroundColor(NousTheme.textMuted)
                                .tracking(0.8)
                            Text(balance.amount)
                                .font(NousTheme.monoLarge)
                                .foregroundColor(NousTheme.text)
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(20)
                        .background(NousTheme.surface)
                        .overlay(
                            Rectangle()
                                .stroke(NousTheme.border, lineWidth: 1)
                        )
                    }
                }

                // Actions
                HStack(spacing: 12) {
                    Button(action: { showingSendSheet = true }) {
                        Text("Send")
                            .font(NousTheme.bodySmall)
                            .foregroundColor(.black)
                            .padding(.horizontal, 20)
                            .padding(.vertical, 10)
                            .frame(maxWidth: .infinity)
                            .background(NousTheme.accent)
                    }
                    .cornerRadius(0)

                    Button(action: {}) {
                        Text("Receive")
                            .font(NousTheme.bodySmall)
                            .foregroundColor(NousTheme.textSecondary)
                            .padding(.horizontal, 20)
                            .padding(.vertical, 10)
                            .frame(maxWidth: .infinity)
                            .overlay(
                                Rectangle()
                                    .stroke(NousTheme.border, lineWidth: 1)
                            )
                    }
                }

                // Transaction History
                VStack(alignment: .leading, spacing: 12) {
                    Text("TRANSACTIONS")
                        .font(NousTheme.label)
                        .foregroundColor(NousTheme.textMuted)
                        .tracking(0.8)

                    if store.transactions.isEmpty {
                        Text("No transactions yet.")
                            .font(NousTheme.bodySmall)
                            .foregroundColor(NousTheme.textMuted)
                            .padding(.top, NousTheme.spacingSM)
                    } else {
                        ForEach(store.transactions) { tx in
                            TransactionRow(tx: tx, myDid: store.did)
                        }
                    }
                }
            }
            .padding(NousTheme.spacingLG)
        }
        .background(NousTheme.background)
        .sheet(isPresented: $showingSendSheet) {
            SendSheet()
                .environment(store)
        }
        .task {
            await store.refreshWallet()
        }
    }
}

// MARK: - Transaction Row

struct TransactionRow: View {
    let tx: TransactionResponse
    let myDid: String

    private var isSent: Bool { tx.fromDid == myDid }

    private var peerDid: String {
        let peer = isSent ? tx.toDid : tx.fromDid
        if peer.count > 20 { return String(peer.prefix(16)) + "..." }
        return peer
    }

    private var timestampDisplay: String {
        let raw = tx.timestamp
        if raw.count > 10 { return String(raw.prefix(10)) }
        return raw
    }

    var body: some View {
        HStack(alignment: .top) {
            VStack(alignment: .leading, spacing: 4) {
                Text(isSent ? "SENT" : "RECEIVED")
                    .font(.system(size: 10, weight: .regular, design: .monospaced))
                    .foregroundColor(isSent ? NousTheme.error : NousTheme.success)
                    .tracking(0.6)
                Text(peerDid)
                    .font(NousTheme.mono)
                    .foregroundColor(NousTheme.textSecondary)
                    .lineLimit(1)
                if let memo = tx.memo, !memo.isEmpty {
                    Text(memo)
                        .font(NousTheme.bodySmall)
                        .foregroundColor(NousTheme.textMuted)
                        .lineLimit(1)
                }
            }
            Spacer()
            VStack(alignment: .trailing, spacing: 4) {
                Text("\(isSent ? "-" : "+")\(tx.amount)")
                    .font(NousTheme.mono)
                    .foregroundColor(isSent ? NousTheme.error : NousTheme.success)
                Text(tx.token.uppercased())
                    .font(.system(size: 10, weight: .regular, design: .monospaced))
                    .foregroundColor(NousTheme.textMuted)
                    .tracking(0.6)
                Text(timestampDisplay)
                    .font(NousTheme.label)
                    .foregroundColor(NousTheme.textMuted)
            }
        }
        .padding(16)
        .background(NousTheme.surface)
        .overlay(
            Rectangle()
                .stroke(NousTheme.border, lineWidth: 1)
        )
    }
}

// MARK: - Send Sheet

struct SendSheet: View {
    @Environment(NousStore.self) private var store
    @Environment(\.dismiss) private var dismiss

    @State private var recipientDid = ""
    @State private var amount = ""
    @State private var selectedToken = "NOUS"
    @State private var memo = ""
    @State private var sending = false

    private let tokens = ["NOUS", "ETH", "USDC"]

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: NousTheme.spacingXL) {
                    VStack(alignment: .leading, spacing: NousTheme.spacingXS) {
                        Text("Send")
                            .font(NousTheme.headlineLarge)
                            .foregroundColor(NousTheme.text)
                        Text("Transfer tokens to a DID")
                            .font(NousTheme.bodySmall)
                            .foregroundColor(NousTheme.textMuted)
                    }

                    // Recipient
                    VStack(alignment: .leading, spacing: NousTheme.spacingSM) {
                        Text("RECIPIENT DID")
                            .font(.system(size: 10, weight: .regular, design: .monospaced))
                            .foregroundColor(NousTheme.textMuted)
                            .tracking(0.8)
                        TextField("did:key:z6Mk...", text: $recipientDid)
                            .font(NousTheme.mono)
                            .foregroundColor(NousTheme.text)
                            .padding(14)
                            .background(NousTheme.surface)
                            .overlay(
                                Rectangle()
                                    .stroke(NousTheme.border, lineWidth: 1)
                            )
                            .autocorrectionDisabled()
                            .textInputAutocapitalization(.never)
                    }

                    // Token selector
                    VStack(alignment: .leading, spacing: NousTheme.spacingSM) {
                        Text("TOKEN")
                            .font(.system(size: 10, weight: .regular, design: .monospaced))
                            .foregroundColor(NousTheme.textMuted)
                            .tracking(0.8)
                        HStack(spacing: 0) {
                            ForEach(tokens, id: \.self) { token in
                                Button(action: { selectedToken = token }) {
                                    Text(token)
                                        .font(.system(size: 10, weight: .regular, design: .monospaced))
                                        .tracking(0.6)
                                        .foregroundColor(
                                            selectedToken == token ? .black : NousTheme.textSecondary
                                        )
                                        .padding(.horizontal, 16)
                                        .padding(.vertical, 10)
                                        .frame(maxWidth: .infinity)
                                        .background(
                                            selectedToken == token
                                                ? NousTheme.accent
                                                : NousTheme.surface
                                        )
                                }
                            }
                        }
                        .overlay(
                            Rectangle()
                                .stroke(NousTheme.border, lineWidth: 1)
                        )
                    }

                    // Amount
                    VStack(alignment: .leading, spacing: NousTheme.spacingSM) {
                        Text("AMOUNT")
                            .font(.system(size: 10, weight: .regular, design: .monospaced))
                            .foregroundColor(NousTheme.textMuted)
                            .tracking(0.8)
                        TextField("0", text: $amount)
                            .font(NousTheme.monoLarge)
                            .foregroundColor(NousTheme.text)
                            .keyboardType(.numberPad)
                            .padding(14)
                            .background(NousTheme.surface)
                            .overlay(
                                Rectangle()
                                    .stroke(NousTheme.border, lineWidth: 1)
                            )
                    }

                    // Memo
                    VStack(alignment: .leading, spacing: NousTheme.spacingSM) {
                        Text("MEMO")
                            .font(.system(size: 10, weight: .regular, design: .monospaced))
                            .foregroundColor(NousTheme.textMuted)
                            .tracking(0.8)
                        TextField("Optional note", text: $memo)
                            .font(NousTheme.body)
                            .foregroundColor(NousTheme.text)
                            .padding(14)
                            .background(NousTheme.surface)
                            .overlay(
                                Rectangle()
                                    .stroke(NousTheme.border, lineWidth: 1)
                            )
                    }

                    // Send button
                    Button(action: {
                        Task { await send() }
                    }) {
                        HStack {
                            if sending {
                                ProgressView()
                                    .tint(.black)
                            }
                            Text(sending ? "Sending..." : "Confirm Transfer")
                                .font(NousTheme.bodySmall)
                                .foregroundColor(.black)
                        }
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 14)
                        .background(canSend ? NousTheme.accent : NousTheme.accent.opacity(0.3))
                    }
                    .cornerRadius(0)
                    .disabled(!canSend || sending)
                }
                .padding(NousTheme.spacingLG)
            }
            .background(NousTheme.background)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                        .foregroundColor(NousTheme.textSecondary)
                }
            }
        }
        .preferredColorScheme(.dark)
    }

    private var canSend: Bool {
        !recipientDid.isEmpty && !amount.isEmpty && Int(amount) != nil
    }

    private func send() async {
        guard let amountInt = Int(amount) else { return }
        sending = true
        await store.sendTransfer(
            toDid: recipientDid,
            token: selectedToken,
            amount: amountInt,
            memo: memo.isEmpty ? nil : memo
        )
        sending = false
        dismiss()
    }
}
