import CalcFFI
import Observation

/// View model over the Rust `Calculator` (exposed through UniFFI).
@MainActor
@Observable
public final class CalculatorModel {
    public private(set) var expression = ""
    public private(set) var display = "0"
    public private(set) var isError = false
    public private(set) var history: [HistoryEntry] = []

    private let calculator = Calculator()

    public init() {}

    public func append(_ token: String) {
        expression += token
        isError = false
    }

    public func backspace() {
        if !expression.isEmpty { expression.removeLast() }
        isError = false
    }

    public func clear() {
        expression = ""
        isError = false
    }

    public func evaluate() {
        do {
            display = try calculator.evaluate(expression: expression)
            history = calculator.history().reversed()
            expression = ""
            isError = false
        } catch CalcError.Invalid(let message) {
            display = message
            isError = true
        } catch {
            display = "\(error)"
            isError = true
        }
    }
}
