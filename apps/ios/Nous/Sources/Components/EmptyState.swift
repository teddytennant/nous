import SwiftUI

/// Editorial empty state: display-serif headline, body subhead, optional
/// oxblood text link (no button chrome). Reserved for "nothing here yet"
/// surfaces; never used as a loading state.
struct EmptyState: View {
    let title: String
    var subtitle: String? = nil
    var actionTitle: String? = nil
    var action: (() -> Void)? = nil

    var body: some View {
        VStack(alignment: .leading, spacing: NousTheme.s4) {
            Text(title)
                .font(NousTheme.headlineLarge)
                .foregroundColor(NousTheme.ivory)
                .tracking(-0.4)
            if let subtitle, !subtitle.isEmpty {
                Text(subtitle)
                    .font(NousTheme.body)
                    .foregroundColor(NousTheme.ivoryDim)
                    .fixedSize(horizontal: false, vertical: true)
            }
            if let actionTitle, let action {
                Button(action: action) {
                    HStack(spacing: NousTheme.s2) {
                        Text(actionTitle)
                            .font(NousTheme.body)
                            .foregroundColor(NousTheme.oxblood)
                        OxbloodMark(style: .arrow)
                    }
                }
                .buttonStyle(.plain)
                .padding(.top, NousTheme.s2)
            }
        }
        .padding(.vertical, NousTheme.s8)
        .frame(maxWidth: .infinity, alignment: .leading)
        .accessibilityElement(children: .combine)
    }
}

#Preview {
    EmptyState(
        title: "No conversations yet",
        subtitle: "Channels appear when you or a peer creates one. Start one from the network or the API.",
        actionTitle: "Read the docs",
        action: {}
    )
    .padding(32)
    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    .background(NousTheme.background)
}
