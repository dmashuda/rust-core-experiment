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

    func testKeyboardInput() {
        let ui = CalculatorUi()
        XCTAssertNotNil(ui.keyInput(key: "9"))
        XCTAssertNil(ui.keyInput(key: "Tab"))
        XCTAssertEqual(ui.keyInput(key: "Enter")?.display, "9")
    }
}
