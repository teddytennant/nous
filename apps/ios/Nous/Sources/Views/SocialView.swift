import SwiftUI

/// Social — feed entries as editorial articles. Display-serif title or first
/// sentence, body sans content, meta caps author + timestamp, mono identifiers.
/// Engagement metrics as KeyValue trio at the bottom of each entry.
struct SocialView: View {
    @Environment(NousStore.self) private var store
    @State private var postContent = ""
    @State private var posting = false

    private var extractedHashtags: [String] {
        postContent
            .split(separator: " ")
            .filter { $0.hasPrefix("#") && $0.count > 1 }
            .map { String($0.dropFirst()) }
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: NousTheme.s12) {
                header
                composer

                if store.feedEvents.isEmpty {
                    EmptyState(
                        title: "An empty broadsheet",
                        subtitle: "Be the first to publish on the sovereign web. Posts here are signed by your DID."
                    )
                } else {
                    feedBlock
                }
            }
            .padding(.horizontal, NousTheme.s6)
            .padding(.vertical, NousTheme.s8)
        }
        .background(NousTheme.background)
        .task { await store.refreshFeed() }
    }

    // MARK: - Header

    private var header: some View {
        VStack(alignment: .leading, spacing: NousTheme.s2) {
            Text("Social")
                .font(NousTheme.display)
                .foregroundColor(NousTheme.ivory)
                .tracking(-1)
            MetaLabel(text: "Decentralised feed")
        }
    }

    // MARK: - Composer

    private var composer: some View {
        VStack(alignment: .leading, spacing: NousTheme.s4) {
            MetaLabel(text: "Compose")
            ZStack(alignment: .topLeading) {
                if postContent.isEmpty {
                    Text("Write something for the public record…")
                        .font(NousTheme.body)
                        .foregroundColor(NousTheme.stone)
                        .padding(.top, NousTheme.s2)
                        .allowsHitTesting(false)
                }
                TextEditor(text: $postContent)
                    .frame(minHeight: 96)
                    .font(NousTheme.body)
                    .foregroundColor(NousTheme.ivory)
                    .scrollContentBackground(.hidden)
                    .background(Color.clear)
            }
            Hairline()

            HStack(alignment: .center) {
                if !extractedHashtags.isEmpty {
                    HStack(spacing: NousTheme.s3) {
                        ForEach(extractedHashtags, id: \.self) { tag in
                            HStack(spacing: 2) {
                                Text("#")
                                    .font(NousTheme.mono)
                                    .foregroundColor(NousTheme.stone)
                                Text(tag)
                                    .font(NousTheme.mono)
                                    .foregroundColor(NousTheme.oxblood)
                            }
                        }
                    }
                }
                Spacer()
                Button(action: { Task { await publishPost() } }) {
                    HStack(spacing: NousTheme.s2) {
                        Text(posting ? "Publishing…" : "Publish")
                            .font(NousTheme.body)
                            .foregroundColor(canPublish ? NousTheme.oxblood : NousTheme.stone)
                        OxbloodMark(
                            style: .arrow,
                            color: canPublish ? NousTheme.oxblood : NousTheme.stone,
                            size: 10
                        )
                    }
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .disabled(!canPublish || posting)
            }
        }
    }

    private var canPublish: Bool {
        !postContent.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    // MARK: - Feed

    private var feedBlock: some View {
        VStack(alignment: .leading, spacing: 0) {
            MetaLabel(text: "Feed")
                .padding(.bottom, NousTheme.s4)
            Hairline()
            ForEach(store.feedEvents) { event in
                FeedArticle(event: event)
            }
        }
    }

    // MARK: - Action

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

// MARK: - Feed Article

private struct FeedArticle: View {
    let event: FeedEvent

    private var authorDisplay: String {
        let p = event.pubkey
        if p.count > 24 { return String(p.prefix(14)) + "…" + String(p.suffix(6)) }
        return p
    }

    private var timestampDisplay: String {
        let raw = event.createdAt
        if raw.count > 10 { return String(raw.prefix(10)) }
        return raw
    }

    private var idDisplay: String {
        let id = event.id
        if id.count > 18 { return String(id.prefix(10)) + "…" + String(id.suffix(4)) }
        return id
    }

    private var hashtags: [String] {
        event.tags.compactMap { tag in
            if tag.first == "t" && tag.count > 1 { return tag[1] }
            return nil
        }
    }

    /// Pull a "headline" from the first sentence — the editorial pattern.
    private var headline: String? {
        let trimmed = event.content.trimmingCharacters(in: .whitespacesAndNewlines)
        let firstSentence = trimmed.split(whereSeparator: { ".!?\n".contains($0) }).first.map(String.init) ?? trimmed
        // Only treat as headline if the post has more content beyond the first sentence
        if firstSentence.count >= 12, firstSentence.count < trimmed.count - 3 {
            return firstSentence
        }
        return nil
    }

    private var body_: String {
        if headline != nil {
            let trimmed = event.content.trimmingCharacters(in: .whitespacesAndNewlines)
            let head = headline!
            return String(trimmed.dropFirst(head.count)).trimmingCharacters(in: CharacterSet(charactersIn: " .!?\n"))
        }
        return event.content
    }

    var body: some View {
        VStack(alignment: .leading, spacing: NousTheme.s4) {
            // Byline
            HStack(spacing: NousTheme.s3) {
                MetaLabel(text: authorDisplay)
                Text("·")
                    .font(NousTheme.label)
                    .foregroundColor(NousTheme.stone)
                MetaLabel(text: timestampDisplay)
                Spacer()
                Text(idDisplay)
                    .font(NousTheme.mono)
                    .foregroundColor(NousTheme.stone)
            }

            // Headline + body
            if let head = headline {
                Text(head)
                    .font(NousTheme.headlineLarge)
                    .foregroundColor(NousTheme.ivory)
                    .tracking(-0.4)
                    .fixedSize(horizontal: false, vertical: true)
            }
            if !body_.isEmpty {
                Text(body_)
                    .font(NousTheme.body)
                    .foregroundColor(NousTheme.ivory)
                    .fixedSize(horizontal: false, vertical: true)
            }

            // Hashtags
            if !hashtags.isEmpty {
                HStack(spacing: NousTheme.s3) {
                    ForEach(hashtags, id: \.self) { tag in
                        HStack(spacing: 2) {
                            Text("#").font(NousTheme.mono).foregroundColor(NousTheme.stone)
                            Text(tag).font(NousTheme.mono).foregroundColor(NousTheme.oxblood)
                        }
                    }
                }
            }

            // Engagement trio (placeholder — store doesn't expose counts yet)
            HStack(spacing: NousTheme.s8) {
                KeyValue(key: "Replies", value: "—")
                KeyValue(key: "Reposts", value: "—")
                KeyValue(key: "Stamps", value: "—")
            }
            .padding(.top, NousTheme.s2)
        }
        .padding(.vertical, NousTheme.s6)
        .frame(maxWidth: .infinity, alignment: .leading)
        .overlay(alignment: .bottom) { Hairline() }
    }
}
