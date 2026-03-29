import SwiftUI

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
            VStack(alignment: .leading, spacing: NousTheme.spacingXL) {
                VStack(alignment: .leading, spacing: NousTheme.spacingXS) {
                    Text("Messages")
                        .font(NousTheme.headlineLarge)
                        .foregroundColor(NousTheme.text)
                    Text("End-to-end encrypted via Double Ratchet")
                        .font(NousTheme.bodySmall)
                        .foregroundColor(NousTheme.textMuted)
                }

                if store.channels.isEmpty {
                    VStack(spacing: NousTheme.spacingMD) {
                        Spacer().frame(height: 60)
                        Text("No conversations yet.")
                            .font(NousTheme.bodySmall)
                            .foregroundColor(NousTheme.textMuted)
                        Text("Channels appear when you or a peer creates one via the API.")
                            .font(NousTheme.label)
                            .foregroundColor(NousTheme.textMuted)
                            .multilineTextAlignment(.center)
                        Spacer().frame(height: 60)
                    }
                    .frame(maxWidth: .infinity)
                } else {
                    VStack(alignment: .leading, spacing: 0) {
                        ForEach(store.channels) { channel in
                            Button(action: { onSelect(channel) }) {
                                ChannelRow(channel: channel)
                            }
                            .buttonStyle(.plain)

                            if channel.id != store.channels.last?.id {
                                Rectangle()
                                    .fill(NousTheme.border)
                                    .frame(height: 1)
                            }
                        }
                    }
                    .overlay(
                        Rectangle()
                            .stroke(NousTheme.border, lineWidth: 1)
                    )
                }
            }
            .padding(NousTheme.spacingLG)
        }
        .background(NousTheme.background)
        .task {
            await store.refreshChannels()
        }
    }
}

// MARK: - Channel Row

struct ChannelRow: View {
    let channel: ChannelResponse

    private var kindIcon: String {
        switch channel.kind {
        case "direct": return "person.2"
        case "group": return "person.3"
        case "public": return "megaphone"
        default: return "bubble.left"
        }
    }

    private var displayName: String {
        if let name = channel.name, !name.isEmpty { return name }
        if channel.kind == "direct" && channel.members.count >= 2 {
            let peer = channel.members.first ?? "Unknown"
            if peer.count > 20 { return String(peer.prefix(16)) + "..." }
            return peer
        }
        return "Channel"
    }

    var body: some View {
        HStack(spacing: 14) {
            Image(systemName: kindIcon)
                .font(.system(size: 14))
                .foregroundColor(NousTheme.accent)
                .frame(width: 32, height: 32)

            VStack(alignment: .leading, spacing: 3) {
                Text(displayName)
                    .font(NousTheme.bodySmall)
                    .foregroundColor(NousTheme.text)
                    .lineLimit(1)
                HStack(spacing: 6) {
                    Text(channel.kind.uppercased())
                        .font(.system(size: 10, weight: .regular, design: .monospaced))
                        .foregroundColor(NousTheme.accent)
                        .tracking(0.6)
                    Text("\(channel.members.count) member\(channel.members.count == 1 ? "" : "s")")
                        .font(NousTheme.label)
                        .foregroundColor(NousTheme.textMuted)
                }
            }

            Spacer()

            Image(systemName: "chevron.right")
                .font(.system(size: 12))
                .foregroundColor(NousTheme.textMuted)
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 14)
        .background(NousTheme.surface)
    }
}

// MARK: - Channel Message View

struct ChannelMessageView: View {
    @Environment(NousStore.self) private var store
    let channel: ChannelResponse
    let onBack: () -> Void

    @State private var messages: [MessageResponse] = []
    @State private var inputText = ""
    @State private var sending = false

    private var channelName: String {
        channel.name ?? "Direct Message"
    }

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack(spacing: 12) {
                Button(action: onBack) {
                    Image(systemName: "chevron.left")
                        .font(.system(size: 14))
                        .foregroundColor(NousTheme.accent)
                }
                VStack(alignment: .leading, spacing: 2) {
                    Text(channelName)
                        .font(NousTheme.bodySmall)
                        .foregroundColor(NousTheme.text)
                        .lineLimit(1)
                    Text(channel.kind.uppercased())
                        .font(.system(size: 10, weight: .regular, design: .monospaced))
                        .foregroundColor(NousTheme.textMuted)
                        .tracking(0.6)
                }
                Spacer()
            }
            .padding(.horizontal, NousTheme.spacingLG)
            .padding(.vertical, 14)
            .background(NousTheme.surface)
            .overlay(
                Rectangle()
                    .fill(NousTheme.border)
                    .frame(height: 1),
                alignment: .bottom
            )

            // Messages
            ScrollView {
                LazyVStack(spacing: 8) {
                    if messages.isEmpty {
                        Text("No messages. Start the conversation.")
                            .font(NousTheme.bodySmall)
                            .foregroundColor(NousTheme.textMuted)
                            .padding(.top, 40)
                    } else {
                        ForEach(messages) { msg in
                            MessageBubble(message: msg, isMe: msg.sender == store.did)
                        }
                    }
                }
                .padding(NousTheme.spacingMD)
            }

            // Input
            HStack(spacing: 10) {
                TextField("Message", text: $inputText)
                    .font(NousTheme.body)
                    .foregroundColor(NousTheme.text)
                    .padding(.horizontal, 14)
                    .padding(.vertical, 10)
                    .background(NousTheme.surface)
                    .overlay(
                        Rectangle()
                            .stroke(NousTheme.border, lineWidth: 1)
                    )
                    .textInputAutocapitalization(.sentences)

                Button(action: {
                    Task { await send() }
                }) {
                    Image(systemName: "arrow.up")
                        .font(.system(size: 14, weight: .medium))
                        .foregroundColor(inputText.isEmpty ? NousTheme.textMuted : .black)
                        .frame(width: 36, height: 36)
                        .background(inputText.isEmpty ? NousTheme.surface : NousTheme.accent)
                }
                .disabled(inputText.isEmpty || sending)
            }
            .padding(.horizontal, NousTheme.spacingMD)
            .padding(.vertical, 10)
            .background(NousTheme.background)
            .overlay(
                Rectangle()
                    .fill(NousTheme.border)
                    .frame(height: 1),
                alignment: .top
            )
        }
        .background(NousTheme.background)
        .task {
            messages = await store.getMessages(channelId: channel.id)
        }
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

// MARK: - Message Bubble

struct MessageBubble: View {
    let message: MessageResponse
    let isMe: Bool

    private var senderDisplay: String {
        let s = message.sender
        if s.count > 16 { return String(s.prefix(12)) + "..." }
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
        HStack {
            if isMe { Spacer(minLength: 40) }
            VStack(alignment: isMe ? .trailing : .leading, spacing: 4) {
                if !isMe {
                    Text(senderDisplay)
                        .font(.system(size: 10, weight: .regular, design: .monospaced))
                        .foregroundColor(NousTheme.accent)
                        .tracking(0.4)
                }
                Text(message.content)
                    .font(NousTheme.body)
                    .foregroundColor(isMe ? .black : NousTheme.text)
                    .padding(.horizontal, 14)
                    .padding(.vertical, 10)
                    .background(isMe ? NousTheme.accent : NousTheme.surface)
                    .overlay(
                        Rectangle()
                            .stroke(
                                isMe ? Color.clear : NousTheme.border,
                                lineWidth: 1
                            )
                    )
                Text(timeDisplay)
                    .font(NousTheme.label)
                    .foregroundColor(NousTheme.textMuted)
            }
            if !isMe { Spacer(minLength: 40) }
        }
    }
}
