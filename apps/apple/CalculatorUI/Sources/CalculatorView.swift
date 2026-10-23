import SwiftUI

/// The calculator UI, shared by the macOS and iOS apps.
public struct CalculatorView: View {
    @State private var model = CalculatorModel()
    @FocusState private var focused: Bool

    public init() {}

    private struct Key: Identifiable {
        let label: String
        let action: Action
        var isOperator = false
        var span = 1
        var id: String { label }
    }

    private enum Action {
        case insert(String), clear, backspace, evaluate
    }

    private let rows: [[Key]] = [
        [Key(label: "C", action: .clear), Key(label: "(", action: .insert("(")),
         Key(label: ")", action: .insert(")")), Key(label: "÷", action: .insert("/"), isOperator: true)],
        [Key(label: "7", action: .insert("7")), Key(label: "8", action: .insert("8")),
         Key(label: "9", action: .insert("9")), Key(label: "×", action: .insert("*"), isOperator: true)],
        [Key(label: "4", action: .insert("4")), Key(label: "5", action: .insert("5")),
         Key(label: "6", action: .insert("6")), Key(label: "−", action: .insert("-"), isOperator: true)],
        [Key(label: "1", action: .insert("1")), Key(label: "2", action: .insert("2")),
         Key(label: "3", action: .insert("3")), Key(label: "+", action: .insert("+"), isOperator: true)],
        [Key(label: "0", action: .insert("0")), Key(label: ".", action: .insert(".")),
         Key(label: "^", action: .insert("^")), Key(label: "⌫", action: .backspace)],
        [Key(label: "ans", action: .insert("ans")), Key(label: "%", action: .insert("%")),
         Key(label: "=", action: .evaluate, isOperator: true, span: 2)],
    ]

    public var body: some View {
        VStack(spacing: 12) {
            VStack(alignment: .trailing, spacing: 4) {
                Text(model.expression.isEmpty ? " " : model.expression)
                    .font(.title3.monospacedDigit())
                    .foregroundStyle(.secondary)
                Text(model.display)
                    .font(model.isError ? .title3 : .system(size: 44, weight: .light).monospacedDigit())
                    .foregroundStyle(model.isError ? .red : .primary)
                    .lineLimit(1)
                    .minimumScaleFactor(0.4)
            }
            .frame(maxWidth: .infinity, alignment: .trailing)
            .textSelection(.enabled)

            Grid(horizontalSpacing: 8, verticalSpacing: 8) {
                ForEach(rows.indices, id: \.self) { r in
                    GridRow {
                        ForEach(rows[r]) { key in
                            Button { perform(key.action) } label: {
                                Text(key.label)
                                    .font(.title2)
                                    .frame(maxWidth: .infinity, minHeight: 48)
                            }
                            .buttonStyle(KeyStyle(isOperator: key.isOperator))
                            .gridCellColumns(key.span)
                        }
                    }
                }
            }

            if !model.history.isEmpty {
                ScrollView {
                    VStack(spacing: 2) {
                        ForEach(Array(model.history.enumerated()), id: \.offset) { _, entry in
                            HStack {
                                Text(entry.expression).foregroundStyle(.secondary)
                                Spacer()
                                Text("= \(entry.result)")
                            }
                            .font(.callout.monospacedDigit())
                        }
                    }
                }
                .frame(maxHeight: 120)
            }

            Text("Evaluated by Rust via UniFFI")
                .font(.caption2)
                .foregroundStyle(.tertiary)
        }
        .padding(20)
        .focusable()
        .focusEffectDisabled()
        .focused($focused)
        .onAppear { focused = true }
        .onKeyPress(phases: .down) { press in
            switch press.key {
            case .return: model.evaluate()
            case .delete: model.backspace()
            case .escape: model.clear()
            default:
                let allowed = Set("0123456789abcdefghijklmnopqrstuvwxyz.+-*/%^() ")
                guard let ch = press.characters.first, press.characters.count == 1, allowed.contains(ch) else {
                    return .ignored
                }
                if ch == "=" { model.evaluate() } else { model.append(String(ch)) }
            }
            return .handled
        }
    }

    private func perform(_ action: Action) {
        switch action {
        case .insert(let token): model.append(token)
        case .clear: model.clear()
        case .backspace: model.backspace()
        case .evaluate: model.evaluate()
        }
    }
}

private struct KeyStyle: ButtonStyle {
    let isOperator: Bool

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .foregroundStyle(isOperator ? Color.white : Color.primary)
            .background(
                RoundedRectangle(cornerRadius: 12, style: .continuous)
                    .fill(isOperator ? Color.orange : Color.secondary.opacity(0.18))
            )
            .opacity(configuration.isPressed ? 0.7 : 1)
    }
}
