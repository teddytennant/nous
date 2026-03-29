import SwiftUI

struct SocialView: View {
    @Environment(NousStore.self) private var store
    @State private var postContent = ""
    @State private var posting = false

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: NousTheme.spacingXL) {
                // Header
                VStack(alignment: .leading, spacing: NousTheme.spacingXS) {
                    Text("Social")
                        .font(NousTheme.headlineLarge)
                        .foregroundColor(NousTheme.text)
                    Text("Decentralized feed")
                        .font(NousTheme.bodySmall)
                        .foregroundColor(NousTheme.textMuted)
                }

                // Compose
                VStack(alignment: .leading, spacing: 12) {
                    Text("COMPOSE")
                        .font(.system(size: 10, weight: .regular, design: .monospaced))
                        .foregroundColor(NousTheme.textMuted)
                        .tracking(0.8)

                    TextEditor(text: $postContent)
                        .frame(minHeight: 80)
                        .font(NousTheme.body)
                        .foregroundColor(NousTheme.text)
                        .scrollContentBackground(.hidden)
                        .padding(12)
                        .background(NousTheme.surface)
                        .overlay(
                            Rectangle()
                                .stroke(NousTheme.border, lineWidth: 1)
                        )

                    HStack {
                        if !extractedHashtags.isEmpty {
                            HStack(spacing: 6) {
                                ForEach(extractedHashtags, id: \.self) { tag in
                                    Text("#\(tag)")
                                        .font(.system(size: 10, weight: .regular, design: .monospaced))
                                        .foregroundColor(NousTheme.accent)
                                        .tracking(0.4)
                                        .padding(.horizontal, 8)
                                        .padding(.vertical, 4)
                                        .background(NousTheme.accentDim)
                                }
                            }
                        }

                        Spacer()

                        Button(action: {
                            Task { await publishPost() }
                        }) {
                            HStack(spacing: 6) {
                                if posting {
                                    ProgressView()
                                        .tint(.black)
                                        .scaleEffect(0.7)
                                }
                                Text(posting ? "Posting" : "Post")
                                    .font(NousTheme.bodySmall)
                                    .foregroundColor(.black)
                            }
                            .padding(.horizontal, 20)
                            .padding(.vertical, 8)
                            .background(
                                postContent.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                                    ? NousTheme.accent.opacity(0.3)
                                    : NousTheme.accent
                            )
                        }
                        .cornerRadius(0)
                        .disabled(
                            postContent.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                                || posting
                        )
                    }
                }

                // Feed
                VStack(alignment: .leading, spacing: 12) {
                    Text("FEED")
                        .font(.system(size: 10, weight: .regular, design: .monospaced))
                        .foregroundColor(NousTheme.textMuted)
                        .tracking(0.8)

                    if store.feedEvents.isEmpty {
                        Text("No posts yet. Be the first to post on the sovereign web.")
                            .font(NousTheme.bodySmall)
                            .foregroundColor(NousTheme.textMuted)
                            .padding(.top, NousTheme.spacingSM)
                    } else {
                        ForEach(store.feedEvents) { event in
                            FeedEventCard(event: event)
                        }
                    }
                }
            }
            .padding(NousTheme.spacingLG)
        }
        .background(NousTheme.background)
        .task {
            await store.refreshFeed()
        }
    }

    // Extract hashtags from post content
    private var extractedHashtags: [String] {
        let words = postContent.split(separator: " ")
        return words
            .filter { $0.hasPrefix("#") && $0.count > 1 }
            .map { String($0.dropFirst()) }
    }

    private func publishPost() async {
        let content = postContent.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !content.isEmpty else { return }
        posting = true
        let tags = extractedHashtags.isEmpty ? nil : extractedHashtags
        await store.publishPost(content: content, hashtags: tags)
        postContent = ""
        posting = false
    }
}

// MARK: - Feed Event Card

struct FeedEventCard: View {
    let event: FeedEvent

    private var authorDisplay: String {
        let p = event.pubkey
        if p.count > 20 { return String(p.prefix(16)) + "..." }
        return p
    }

    private var timestampDisplay: String {
        let raw = event.createdAt
        if raw.count > 10 { return String(raw.prefix(10)) }
        return raw
    }

    private var hashtags: [String] {
        event.tags.compactMap { tag in
            if tag.first == "t" && tag.count > 1 { return tag[1] }
            return nil
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            // Author + timestamp
            HStack {
                Text(authorDisplay)
                    .font(.system(size: 10, weight: .regular, design: .monospaced))
                    .foregroundColor(NousTheme.accent)
                    .tracking(0.4)
                Spacer()
                Text(timestampDisplay)
                    .font(NousTheme.label)
                    .foregroundColor(NousTheme.textMuted)
            }

            // Content
            Text(event.content)
                .font(NousTheme.body)
                .foregroundColor(NousTheme.text)
                .fixedSize(horizontal: false, vertical: true)

            // Hashtags
            if !hashtags.isEmpty {
                HStack(spacing: 6) {
                    ForEach(hashtags, id: \.self) { tag in
                        Text("#\(tag)")
                            .font(.system(size: 10, weight: .regular, design: .monospaced))
                            .foregroundColor(NousTheme.accent)
                            .tracking(0.4)
                            .padding(.horizontal, 8)
                            .padding(.vertical, 4)
                            .background(NousTheme.accentDim)
                    }
                }
            }
        }
        .padding(16)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(NousTheme.surface)
        .overlay(
            Rectangle()
                .stroke(NousTheme.border, lineWidth: 1)
        )
    }
}
