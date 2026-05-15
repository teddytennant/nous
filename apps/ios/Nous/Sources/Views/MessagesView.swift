import SwiftUI

/// Messages — editorial thread list and message detail. No bubbles.
struct MessagesView: View {
    @Environment(NousStore.self) private var store
    @State private var selectedChannel: ChannelResponse?

    var body: some View {
        if let channel = selectedChannel {
            ChannelMessageView(channel: channel, onBack: { selectedChannel = nil })
                .environment(store)
        } else {
            ChannelListView(onSelect: { selectedChannel = $0 })
                .environment(store)
        }
    }
}

// MARK: - Channel List

struct ChannelListView: View {
    @Environment(NousStore.self) private var store
    let onSelect: (ChannelResponse) -> Void

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: NousTheme.s8) {
                header

                if store.channels.isEmpty {
                    EmptyState(
                        title: "No conversations yet",
                        subtitle: "Channels appear when you or a peer creates one. The Nous protocol carries them end-to-end with the Double Ratchet."
                    )
                } else {
                    VStack(spacing: 0) {
                        Hairline()
                        ForEach(store.channels) { channel in
                            ChannelRow(channel: channel, onTap: { onSelect(channel) })
                        }
                    }
                }
            }
            .padding(.horizontal, NousTheme.s6)
            .padding(.vertical, NousTheme.s8)
        }
        .background(NousTheme.background)
        .task { await store.refreshChannels() }
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: NousTheme.s2) {
            Text("Messages")
                .font(NousTheme.display)
                .foregroundColor(NousTheme.ivory)
                .tracking(-1)
            MetaLabel(text: "End-to-end · Double Ratchet")
        }
    }
}

// MARK: - Channel Row (editorial)

private struct ChannelRow: View {
    let channel: ChannelResponse
    let onTap: () -> Void
    @State private var pressed = false

    private var displayName: String {
        if let name = channel.name, !name.isEmpty { return name }
        if channel.kind == "direct", let peer = channel.members.first {
            if peer.count > 24 {
                return String(peer.prefix(14)) + "…" + String(peer.suffix(6))
            }
            return peer
        }
        return "Channel"
    }

    private var lastExcerpt: String {
        switch channel.kind {
        case "direct": return "Direct message · encrypted"
        case "group": return "Group · \(channel.members.count) member\(channel.members.count == 1 ? "" : "s")"
        case "public": return "Public channel"
        default: return "Channel"
        }
    }

    private var timestampDisplay: String {
        let raw = channel.createdAt
        if raw.count > 10 { return String(raw.prefix(10)) }
        return raw
    }

    var body: some View {
        Button(action: onTap) {
            VStack(spacing: 0) {
                HStack(alignment: .firstTextBaseline, spacing: NousTheme.s4) {
                    VStack(alignment: .leading, spacing: NousTheme.s2) {
                        Text(displayName)
                            .font(NousTheme.headlineMedium)
                            .foregroundColor(NousTheme.ivory)
                            .lineLimit(1)
                        Text(lastExcerpt)
                            .font(NousTheme.body)
                            .foregroundColor(NousTheme.ivoryDim)
                            .lineLimit(1)
                    }
                    Spacer(minLength: NousTheme.s2)
                    Text(timestampDisplay)
                        .font(NousTheme.mono)
                        .foregroundColor(NousTheme.stone)
                }
                .padding(.vertical, NousTheme.s4)
                .background(NousTheme.oxblood.opacity(pressed ? 0.12 : 0))
                Hairline()
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .simultaneousGesture(
            DragGesture(minimumDistance: 0)
                .onChanged { _ in if !pressed { pressed = true } }
                .onEnded { _ in pressed = false }
        )
    }
}

// MARK: - Channel Detail

struct ChannelMessageView: View {
    @Environment(NousStore.self) private var store
    let channel: ChannelResponse
    let onBack: () -> Void

    @State private var messages: [MessageResponse] = []
    @State private var inputText = ""
    @State private var sending = false

    private var channelName: String {
        if let name = channel.name, !name.isEmpty { return name }
        return channel.kind.capitalized + " channel"
    }

    var body: some View {
        VStack(spacing: 0) {
            detailHeader
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 0) {
                    if messages.isEmpty {
                        EmptyState(
                            title: "Quiet line.",
                            subtitle: "No messages have come through yet. Start the conversation."
                        )
                        .padding(.horizontal, NousTheme.s6)
                    } else {
                        Hairline()
                        ForEach(messages) { msg in
                            MessageBlock(message: msg, isMe: msg.sender == store.did)
                        }
                    }
                }
                .padding(.bottom, NousTheme.s8)
            }
            inputBar
        }
        .background(NousTheme.background)
        .task {
            messages = await store.getMessages(channelId: channel.id)
        }
    }

    private var detailHeader: some View {
        VStack(spacing: 0) {
            HStack(spacing: NousTheme.s3) {
                Button(action: onBack) {
                    HStack(spacing: NousTheme.s1) {
                        Text("‹")
                            .font(.system(size: 22, weight: .regular))
                            .foregroundColor(NousTheme.oxblood)
                        Text("Back")
                            .font(NousTheme.label)
                            .tracking(NousTheme.metaTracking)
                            .foregroundColor(NousTheme.oxblood)
                    }
                }
                .buttonStyle(.plain)
                Spacer()
                VStack(alignment: .trailing, spacing: NousTheme.s1) {
                    Text(channelName)
                        .font(NousTheme.titleLarge)
                        .foregroundColor(NousTheme.ivory)
                        .lineLimit(1)
                    MetaLabel(text: channel.kind)
                }
            }
            .padding(.horizontal, NousTheme.s6)
            .padding(.vertical, NousTheme.s4)
            Hairline()
        }
        .background(NousTheme.background)
    }

    private var inputBar: some View {
        VStack(spacing: 0) {
            Hairline()
            HStack(alignment: .bottom, spacing: NousTheme.s3) {
                TextField("", text: $inputText, prompt: Text("Compose").foregroundColor(NousTheme.stone), axis: .vertical)
                    .font(NousTheme.body)
                    .foregroundColor(NousTheme.ivory)
                    .lineLimit(1...5)
                    .textInputAutocapitalization(.sentences)

                Button(action: { Task { await send() } }) {
                    HStack(spacing: NousTheme.s1) {
                        Text(sending ? "Sending" : "Send")
                            .font(NousTheme.label)
                            .tracking(NousTheme.metaTracking)
                            .foregroundColor(inputText.isEmpty ? NousTheme.stone : NousTheme.oxblood)
                        OxbloodMark(
                            style: .arrow,
                            color: inputText.isEmpty ? NousTheme.stone : NousTheme.oxblood,
                            size: 10
                        )
                    }
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .disabled(inputText.isEmpty || sending)
            }
            .padding(.horizontal, NousTheme.s6)
            .padding(.vertical, NousTheme.s3)
        }
        .background(NousTheme.background)
    }

    private func send() async {
        let text = inputText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty else { return }
        sending = true
        inputText = ""
        if let msg = await store.sendMessage(channelId: channel.id, content: text) {
            messages.append(msg)
        }
        sending = false
    }
}

// MARK: - Message Block (editorial — no bubble)

private struct MessageBlock: View {
    let message: MessageResponse
    let isMe: Bool

    private var senderDisplay: String {
        if isMe { return "You" }
        let s = message.sender
        if s.count > 24 { return String(s.prefix(14)) + "…" + String(s.suffix(6)) }
        return s
    }

    private var timeDisplay: String {
        let t = message.timestamp
        if t.count > 16 {
            let start = t.index(t.startIndex, offsetBy: 11)
            let end = t.index(t.startIndex, offsetBy: 16)
            return String(t[start..<end])
        }
        return t
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack(alignment: .firstTextBaseline, spacing: NousTheme.s4) {
                VStack(alignment: .leading, spacing: NousTheme.s2) {
                    HStack(spacing: NousTheme.s2) {
                        MetaLabel(text: senderDisplay, active: isMe)
                        if isMe {
                            OxbloodMark(style: .disc, size: 5)
                        }
                    }
                    Text(message.content)
                        .font(NousTheme.body)
                        .foregroundColor(NousTheme.ivory)
                        .fixedSize(horizontal: false, vertical: true)
                }
                Spacer(minLength: NousTheme.s4)
                Text(timeDisplay)
                    .font(NousTheme.mono)
                    .foregroundColor(NousTheme.stone)
            }
            .padding(.horizontal, NousTheme.s6)
            .padding(.vertical, NousTheme.s4)
            Hairline()
        }
    }
}
