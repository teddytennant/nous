import SwiftUI

/// Flexible horizontal row with leading/trailing slots and an optional press
/// state — 12% oxblood overlay on tap, never a tap ripple. The row is bounded
/// by a single hairline below by default.
struct Row<Leading: View, Trailing: View>: View {
    var action: (() -> Void)? = nil
    var bottomRule: Bool = true
    var verticalPadding: CGFloat = NousTheme.s3
    @ViewBuilder var leading: () -> Leading
    @ViewBuilder var trailing: () -> Trailing

    @State private var pressed = false

    var body: some View {
        Group {
            if let action {
                Button(action: action) { content }
                    .buttonStyle(.plain)
                    .simultaneousGesture(
                        DragGesture(minimumDistance: 0)
                            .onChanged { _ in
                                if !pressed {
                                    withAnimation(NousTheme.easing.speed(2)) { pressed = true }
                                }
                            }
                            .onEnded { _ in
                                withAnimation(NousTheme.easing) { pressed = false }
                            }
                    )
            } else {
                content
            }
        }
    }

    private var content: some View {
        VStack(spacing: 0) {
            HStack(alignment: .firstTextBaseline, spacing: NousTheme.s3) {
                leading()
                Spacer(minLength: NousTheme.s2)
                trailing()
            }
            .padding(.vertical, verticalPadding)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(
                NousTheme.oxblood.opacity(pressed ? 0.12 : 0)
            )
            if bottomRule { Hairline() }
        }
    }
}

#Preview {
    ScrollView {
        VStack(spacing: 0) {
            Row(action: {}) {
                Text("did:key:z6MkpTHR…J9q")
                    .font(NousTheme.body)
                    .foregroundColor(NousTheme.ivory)
            } trailing: {
                MetaLabel(text: "Direct")
            }
            Row(action: {}) {
                Text("Storage Daemon")
                    .font(NousTheme.body)
                    .foregroundColor(NousTheme.ivory)
            } trailing: {
                Text("0.04231 NOUS")
                    .font(NousTheme.mono)
                    .foregroundColor(NousTheme.ivory)
            }
            Row(bottomRule: false) {
                Text("Idle")
                    .font(NousTheme.body)
                    .foregroundColor(NousTheme.ivoryDim)
            } trailing: {
                MetaLabel(text: "Standby")
            }
        }
        .padding(.horizontal, 32)
    }
    .background(NousTheme.background)
}
