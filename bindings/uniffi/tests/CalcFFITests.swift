import CalcFFI
import XCTest

/// Exercises the generated Swift bindings against the real Rust library.
final class CalcFFITests: XCTestCase {
    func testPressingKeysEvaluates() {
        let ui = CalculatorUi()
        var vm = ui.view()
        XCTAssertEqual(vm.display, "0")
        for id in ["6", "*", "7"] { vm = ui.press(id: id) }
        XCTAssertEqual(vm.expression, "6*7")
        vm = ui.press(id: "evaluate")
        XCTAssertEqual(vm.display, "42")
        XCTAssertEqual(vm.history, [HistoryLine(id: 1, expression: "6*7", result: "42")])
    }

    func testErrorsAreViewStateAndEditingRestoresTheResult() {
        let ui = CalculatorUi()
        for id in ["6", "*", "7", "evaluate", "1", "/", "0"] { _ = ui.press(id: id) }
        let error = ui.press(id: "evaluate")
        XCTAssertTrue(error.isError)
        XCTAssertEqual(error.display, "division by zero")

        let edited = ui.press(id: "backspace")
        XCTAssertFalse(edited.isError)
        XCTAssertEqual(edited.display, "42", "the error text must not linger as a result")
    }

    func testOperatorsContinueFromTheLastResult() {
        let ui = CalculatorUi()
        for id in ["6", "*", "7", "evaluate"] { _ = ui.press(id: id) }
        XCTAssertEqual(ui.press(id: "+").expression, "ans+")
        _ = ui.press(id: "1")
        XCTAssertEqual(ui.press(id: "evaluate").display, "43")
    }

    func testKeypadIsFetchedOnceAndNotPartOfTheViewModel() {
        let rows = keypad()
        XCTAssertEqual(rows.first?.keys.first?.label, "C")
        XCTAssertEqual(rows.last?.keys.last?.style, .operator)
        XCTAssertEqual(rows.last?.keys.last?.span, 2)
        XCTAssertEqual(rows, keypad())
    }

    /// A database path of this test's own, inside the sandbox Bazel gives it.
    ///
    /// Deleted first: `xcrun xctest` reuses TEST_TMPDIR between runs, so a
    /// leftover database from the last run would already hold history.
    private func dbPath(_ name: String) -> String {
        let dir = ProcessInfo.processInfo.environment["TEST_TMPDIR"] ?? NSTemporaryDirectory()
        let path = "\(dir)/\(name).db"
        for file in [path, path + "-journal"] {
            try? FileManager.default.removeItem(atPath: file)
        }
        return path
    }

    func testHistoryInSqliteSurvivesANewCalculator() {
        let path = dbPath("swift-history")
        let first = CalculatorUi.open(dbPath: path)
        XCTAssertTrue(
            first.view().caption.hasPrefix("Evaluated by Rust, history in SQLite 3."),
            first.view().caption
        )
        for id in ["6", "*", "7", "evaluate"] { _ = first.press(id: id) }

        // A second calculator over the same file: the app being reopened.
        let second = CalculatorUi.open(dbPath: path)
        let vm = second.view()
        XCTAssertEqual(vm.history, [HistoryLine(id: 1, expression: "6*7", result: "42")])
        XCTAssertEqual(vm.display, "42", "the previous answer is the display, not 0")
        _ = second.press(id: "+")
        _ = second.press(id: "1")
        XCTAssertEqual(second.press(id: "evaluate").display, "43", "`ans` survived too")
    }

    func testAnUnusablePathStillGivesAWorkingCalculator() {
        let ui = CalculatorUi.open(dbPath: dbPath("no/such/dir/history"))
        for id in ["6", "*", "7", "evaluate"] { _ = ui.press(id: id) }
        let vm = ui.view()
        XCTAssertEqual(vm.display, "42")
        XCTAssertTrue(vm.caption.contains("memory ("), vm.caption)
    }

    func testKeyboardInput() {
        let ui = CalculatorUi()
        XCTAssertNotNil(ui.keyInput(key: "9"))
        XCTAssertNil(ui.keyInput(key: "Tab"))
        XCTAssertEqual(ui.keyInput(key: "Enter")?.display, "9")
    }
}
