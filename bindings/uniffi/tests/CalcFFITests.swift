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
        XCTAssertEqual(vm.history, [HistoryLine(expression: "6*7", result: "42")])
    }

    func testErrorsAreViewState() {
        let ui = CalculatorUi()
        for id in ["1", "/", "0"] { _ = ui.press(id: id) }
        let vm = ui.press(id: "evaluate")
        XCTAssertTrue(vm.isError)
        XCTAssertEqual(vm.display, "division by zero")
    }

    func testKeypadAndKeyboard() {
        let ui = CalculatorUi()
        let keypad = ui.view().keypad
        XCTAssertEqual(keypad.first?.keys.first?.label, "C")
        XCTAssertEqual(keypad.last?.keys.last?.style, .operator)
        XCTAssertNotNil(ui.keyInput(key: "9"))
        XCTAssertNil(ui.keyInput(key: "Tab"))
        XCTAssertEqual(ui.keyInput(key: "Enter")?.display, "9")
    }
}
