import SwiftUI

struct SocialView: View {
    @State private var postContent = ""

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: NousTheme.spacingXL) {
                VStack(alignment: .leading, spacing: NousTheme.spacingXS) {
                    Text("Social")
                        .font(NousTheme.headlineLarge)
                        .foregroundColor(NousTheme.text)
                    Text("Decentralized feed")
                        .font(NousTheme.bodySmall)
                        .foregroundColor(NousTheme.textMuted)
                }

                VStack(alignment: .leading, spacing: 12) {
                    TextEditor(text: $postContent)
                        .frame(minHeight: 80)
                        .font(NousTheme.body)
                        .foregroundColor(NousTheme.text)
                        .scrollContentBackground(.hidden)
                        .background(NousTheme.surface)
                        .overlay(
                            RoundedRectangle(cornerRadius: NousTheme.radiusMD)
                                .stroke(NousTheme.border, lineWidth: 1)
                        )
                        .clipShape(RoundedRectangle(cornerRadius: NousTheme.radiusMD))

                    Button(action: { postContent = "" }) {
                        Text("Post")
                            .font(NousTheme.bodySmall)
                            .fontWeight(.regular)
                            .foregroundColor(.black)
                            .padding(.horizontal, 20)
                            .padding(.vertical, 8)
                            .background(NousTheme.accent)
                            .clipShape(RoundedRectangle(cornerRadius: NousTheme.radiusSM))
                    }
                }

                Text("No posts yet. Be the first to post on the sovereign web.")
                    .font(NousTheme.bodySmall)
                    .foregroundColor(NousTheme.textMuted)
                    .padding(.top, NousTheme.spacingSM)
            }
            .padding(NousTheme.spacingLG)
        }
        .background(NousTheme.background)
    }
}
