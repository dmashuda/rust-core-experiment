import CalcFFI
import Foundation
import Observation

/// Holds the Rust `CalculatorUi` and its latest `ViewModel` for SwiftUI.
/// No calculator logic here: input goes to Rust, the returned state is drawn.
@MainActor
@Observable
public final class CalculatorModel {
    public private(set) var view: ViewModel

    /// The keypad is static, so it is fetched from Rust once per screen rather
    /// than arriving in every `ViewModel`.
    public let keypad: [KeyRow] = CalcFFI.keypad()

    private let ui: CalculatorUi

    /// `databasePath` nil keeps history in memory, which is what tests and
    /// previews want. The apps use ``persistent()``.
    public init(databasePath: String? = nil) {
        ui = databasePath.map { CalculatorUi.open(dbPath: $0) } ?? CalculatorUi()
        view = ui.view()
    }

    /// A model whose history is a SQLite file in the app's own Application
    /// Support directory, so it is still there next launch.
    ///
    /// Nothing here handles failure: if the directory cannot be made, Rust
    /// keeps history in memory and `view.caption` says so.
    public static func persistent() -> CalculatorModel {
        CalculatorModel(databasePath: historyDatabasePath())
    }

    /// `<Application Support>/<bundle id>/history.db`, directory created.
    private static func historyDatabasePath() -> String? {
        let manager = FileManager.default
        guard let base = manager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
        else { return nil }
        let directory = base.appendingPathComponent(Bundle.main.bundleIdentifier ?? "RustCalculator")
        try? manager.createDirectory(at: directory, withIntermediateDirectories: true)
        return directory.appendingPathComponent("history.db").path
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
