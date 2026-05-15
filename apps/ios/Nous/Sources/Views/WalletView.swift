import SwiftUI

/// Wallet — display-serif amount, ticker in meta caps, hairline, ledger
/// entries as editorial list rows. No card backgrounds.
struct WalletView: View {
    @Environment(NousStore.self) private var store
    @State private var showingSendSheet = false
    @State private var primaryToken = "NOUS"

    private var primaryBalance: BalanceEntry? {
        if let match = store.balances.first(where: { $0.token.uppercased() == primaryToken }) {
            return match
        }
        return store.balances.first
    }

    private var secondaryBalances: [BalanceEntry] {
        guard let primary = primaryBalance else { return [] }
        return store.balances.filter { $0.token != primary.token }
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: NousTheme.s12) {
                header
                primaryBalanceBlock
                actions
                if !secondaryBalances.isEmpty {
                    secondaryBalanceBlock
                }
                ledgerBlock
            }
            .padding(.horizontal, NousTheme.s6)
            .padding(.vertical, NousTheme.s8)
        }
        .background(NousTheme.background)
        .sheet(isPresented: $showingSendSheet) {
            SendSheet().environment(store)
        }
        .task { await store.refreshWallet() }
    }

    // MARK: - Header

    private var header: some View {
        VStack(alignment: .leading, spacing: NousTheme.s2) {
            Text("Wallet")
                .font(NousTheme.display)
                .foregroundColor(NousTheme.ivory)
                .tracking(-1)
            MetaLabel(text: "Multi-chain · escrow-backed")
        }
    }

    // MARK: - Primary balance

    private var primaryBalanceBlock: some View {
        VStack(alignment: .leading, spacing: NousTheme.s3) {
            Text(primaryBalance?.amount ?? "0.000")
                .font(.custom("SourceSerif4", size: 80).weight(.regular))
                .foregroundColor(NousTheme.ivory)
                .tracking(-2)
                .lineLimit(1)
                .minimumScaleFactor(0.4)
            MetaLabel(
                text: primaryBalance?.token.uppercased() ?? "NOUS",
                active: true
            )
            Hairline().padding(.top, NousTheme.s4)
        }
    }

    // MARK: - Actions

    private var actions: some View {
        HStack(spacing: NousTheme.s8) {
            Button(action: { showingSendSheet = true }) {
                HStack(spacing: NousTheme.s2) {
                    Text("Send")
                        .font(NousTheme.body)
                        .foregroundColor(NousTheme.oxblood)
                    OxbloodMark(style: .arrow, size: 10)
                }
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)

            Button(action: {}) {
                HStack(spacing: NousTheme.s2) {
                    Text("Receive")
                        .font(NousTheme.body)
                        .foregroundColor(NousTheme.ivoryDim)
                }
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)

            Spacer()
        }
    }

    // MARK: - Secondary balances

    private var secondaryBalanceBlock: some View {
        VStack(alignment: .leading, spacing: NousTheme.s4) {
            MetaLabel(text: "Other Balances")
            VStack(spacing: 0) {
                Hairline()
                ForEach(secondaryBalances) { balance in
                    HStack(alignment: .firstTextBaseline) {
                        Text(balance.token.uppercased())
                            .font(NousTheme.body)
                            .foregroundColor(NousTheme.ivory)
                        Spacer()
                        Text(balance.amount)
                            .font(NousTheme.mono)
                            .foregroundColor(NousTheme.ivory)
                    }
                    .padding(.vertical, NousTheme.s3)
                    Hairline()
                }
            }
        }
    }

    // MARK: - Ledger

    private var ledgerBlock: some View {
        VStack(alignment: .leading, spacing: NousTheme.s4) {
            MetaLabel(text: "Ledger")
            if store.transactions.isEmpty {
                Text("No transactions yet.")
                    .font(NousTheme.body)
                    .foregroundColor(NousTheme.ivoryDim)
                    .padding(.vertical, NousTheme.s4)
            } else {
                VStack(spacing: 0) {
                    Hairline()
                    ForEach(store.transactions) { tx in
                        LedgerRow(tx: tx, myDid: store.did)
                    }
                }
            }
        }
    }
}

// MARK: - Ledger Row (editorial)

private struct LedgerRow: View {
    let tx: TransactionResponse
    let myDid: String

    private var isSent: Bool { tx.fromDid == myDid }
    private var counterparty: String {
        let peer = isSent ? tx.toDid : tx.fromDid
        if peer.count > 24 {
            return String(peer.prefix(14)) + "…" + String(peer.suffix(6))
        }
        return peer
    }

    private var timestampDisplay: String {
        let raw = tx.timestamp
        if raw.count > 10 { return String(raw.prefix(10)) }
        return raw
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack(alignment: .firstTextBaseline, spacing: NousTheme.s4) {
                MetaLabel(text: timestampDisplay)
                    .frame(width: 88, alignment: .leading)
                VStack(alignment: .leading, spacing: NousTheme.s1) {
                    Text(isSent ? "Sent to" : "Received from")
                        .font(NousTheme.body)
                        .foregroundColor(NousTheme.ivory)
                    Text(counterparty)
                        .font(NousTheme.mono)
                        .foregroundColor(NousTheme.stone)
                        .lineLimit(1)
                        .truncationMode(.middle)
                    if let memo = tx.memo, !memo.isEmpty {
                        Text(memo)
                            .font(NousTheme.bodySmall)
                            .foregroundColor(NousTheme.ivoryDim)
                            .lineLimit(1)
                    }
                }
                Spacer(minLength: NousTheme.s4)
                VStack(alignment: .trailing, spacing: NousTheme.s1) {
                    Text("\(isSent ? "−" : "+")\(tx.amount)")
                        .font(NousTheme.mono)
                        .foregroundColor(NousTheme.ivory)
                    MetaLabel(text: tx.token)
                }
            }
            .padding(.vertical, NousTheme.s3)
            Hairline()
        }
    }
}

// MARK: - Send Sheet (editorial)

struct SendSheet: View {
    @Environment(NousStore.self) private var store
    @Environment(\.dismiss) private var dismiss

    @State private var recipientDid = ""
    @State private var amount = ""
    @State private var selectedToken = "NOUS"
    @State private var memo = ""
    @State private var sending = false

    private let tokens = ["NOUS", "ETH", "USDC"]

    private var canSend: Bool {
        !recipientDid.isEmpty && !amount.isEmpty && Int(amount) != nil
    }

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: NousTheme.s8) {
                    VStack(alignment: .leading, spacing: NousTheme.s2) {
                        Text("Send")
                            .font(NousTheme.display)
                            .foregroundColor(NousTheme.ivory)
                            .tracking(-1)
                        MetaLabel(text: "Transfer to a DID")
                    }

                    field(
                        label: "Recipient DID",
                        text: $recipientDid,
                        placeholder: "did:key:z6Mk…",
                        font: NousTheme.mono,
                        autocorrect: false,
                        capitalization: .never
                    )

                    VStack(alignment: .leading, spacing: NousTheme.s2) {
                        MetaLabel(text: "Token")
                        HStack(spacing: NousTheme.s4) {
                            ForEach(tokens, id: \.self) { token in
                                Button(action: { selectedToken = token }) {
                                    HStack(spacing: NousTheme.s1) {
                                        if selectedToken == token {
                                            OxbloodMark(style: .dot, size: 6)
                                        }
                                        Text(token)
                                            .font(NousTheme.mono)
                                            .foregroundColor(
                                                selectedToken == token ? NousTheme.oxblood : NousTheme.ivoryDim
                                            )
                                    }
                                    .padding(.vertical, NousTheme.s2)
                                    .contentShape(Rectangle())
                                }
                                .buttonStyle(.plain)
                            }
                            Spacer()
                        }
                        Hairline()
                    }

                    field(
                        label: "Amount",
                        text: $amount,
                        placeholder: "0",
                        font: NousTheme.monoLarge,
                        keyboard: .numberPad
                    )

                    field(
                        label: "Memo",
                        text: $memo,
                        placeholder: "Optional note",
                        font: NousTheme.body
                    )

                    Button(action: { Task { await send() } }) {
                        HStack(spacing: NousTheme.s2) {
                            Text(sending ? "Sending…" : "Confirm Transfer")
                                .font(NousTheme.body)
                                .foregroundColor(canSend ? NousTheme.oxblood : NousTheme.stone)
                            OxbloodMark(
                                style: .arrow,
                                color: canSend ? NousTheme.oxblood : NousTheme.stone,
                                size: 10
                            )
                        }
                        .padding(.vertical, NousTheme.s4)
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                    .disabled(!canSend || sending)
                }
                .padding(.horizontal, NousTheme.s6)
                .padding(.vertical, NousTheme.s8)
            }
            .background(NousTheme.background)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                        .tint(NousTheme.oxblood)
                }
            }
        }
        .preferredColorScheme(.dark)
    }

    @ViewBuilder
    private func field(
        label: String,
        text: Binding<String>,
        placeholder: String,
        font: Font,
        keyboard: UIKeyboardType = .default,
        autocorrect: Bool = true,
        capitalization: TextInputAutocapitalization = .sentences
    ) -> some View {
        VStack(alignment: .leading, spacing: NousTheme.s2) {
            MetaLabel(text: label)
            TextField("", text: text, prompt: Text(placeholder).foregroundColor(NousTheme.stone))
                .font(font)
                .foregroundColor(NousTheme.ivory)
                .keyboardType(keyboard)
                .autocorrectionDisabled(!autocorrect)
                .textInputAutocapitalization(capitalization)
                .padding(.vertical, NousTheme.s2)
            Hairline()
        }
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
