import CalcFFI
import SwiftUI

/// Renders the Rust `ViewModel`; shared by the macOS and iOS apps.
public struct CalculatorView: View {
    @State private var model = CalculatorModel()
    @FocusState private var focused: Bool

    public init() {}

    public var body: some View {
        let vm = model.view
        VStack(spacing: 12) {
            VStack(alignment: .trailing, spacing: 4) {
                Text(vm.expression.isEmpty ? " " : vm.expression)
                    .font(.title3.monospacedDigit())
                    .foregroundStyle(.secondary)
                Text(vm.display)
                    .font(vm.isError ? .title3 : .system(size: 44, weight: .light).monospacedDigit())
                    .foregroundStyle(vm.isError ? .red : .primary)
                    .lineLimit(1)
                    .minimumScaleFactor(0.4)
            }
            .frame(maxWidth: .infinity, alignment: .trailing)
            .textSelection(.enabled)

            Grid(horizontalSpacing: 8, verticalSpacing: 8) {
                ForEach(vm.keypad.indices, id: \.self) { r in
                    GridRow {
                        ForEach(vm.keypad[r].keys, id: \.id) { key in
                            Button { model.press(key.id) } label: {
                                Text(key.label)
                                    .font(.title2)
                                    .frame(maxWidth: .infinity, minHeight: 48)
                            }
                            .buttonStyle(KeyButtonStyle(style: key.style))
                            .gridCellColumns(Int(key.span))
                        }
                    }
                }
            }

            if !vm.history.isEmpty {
                ScrollView {
                    VStack(spacing: 2) {
                        ForEach(Array(vm.history.enumerated()), id: \.offset) { _, line in
                            HStack {
                                Text(line.expression).foregroundStyle(.secondary)
                                Spacer()
                                Text("= \(line.result)")
                            }
                            .font(.callout.monospacedDigit())
                        }
                    }
                }
                .frame(maxHeight: 120)
            }

            Text("\(vm.caption) (UniFFI)")
                .font(.caption2)
                .foregroundStyle(.tertiary)
        }
        .padding(20)
        .focusable()
        .focusEffectDisabled()
        .focused($focused)
        .onAppear { focused = true }
        .onKeyPress(phases: .down) { press in
            let key: String
            switch press.key {
            case .return: key = "Enter"
            case .delete: key = "Backspace"
            case .escape: key = "Escape"
            default: key = press.characters
            }
            return model.keyInput(key) ? .handled : .ignored
        }
    }
}

private struct KeyButtonStyle: ButtonStyle {
    let style: KeyStyle

    func makeBody(configuration: Configuration) -> some View {
        let isOperator = style == .operator
        configuration.label
            .foregroundStyle(isOperator ? Color.white : Color.primary)
            .background(
                RoundedRectangle(cornerRadius: 12, style: .continuous)
                    .fill(isOperator ? Color.orange : Color.secondary.opacity(style == .input ? 0.18 : 0.28))
            )
            .opacity(configuration.isPressed ? 0.7 : 1)
    }
}
