import CalcFFI
import Observation

/// Holds the Rust `CalculatorUi` and its latest `ViewModel` for SwiftUI.
/// No calculator logic here: input goes to Rust, the returned state is drawn.
@MainActor
@Observable
public final class CalculatorModel {
    public private(set) var view: ViewModel

    private let ui = CalculatorUi()

    public init() {
        view = ui.view()
    }

    public func press(_ id: String) {
        view = ui.press(id: id)
    }

    /// Returns false if Rust ignored the key, so it can propagate.
    public func keyInput(_ key: String) -> Bool {
        guard let next = ui.keyInput(key: key) else { return false }
        view = next
        return true
    }
}
