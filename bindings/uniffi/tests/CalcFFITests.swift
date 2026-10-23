import CalcFFI
import XCTest

/// Exercises the generated Swift bindings against the real Rust library.
final class CalcFFITests: XCTestCase {
    func testFreeFunction() throws {
        XCTAssertEqual(try evaluate(expression: "1 + 2 * 3"), "7")
        XCTAssertEqual(try evaluate(expression: "0.1 + 0.2"), "0.3")
    }

    func testErrorsBecomeSwiftErrors() {
        XCTAssertThrowsError(try evaluate(expression: "1 / 0")) { error in
            guard case CalcError.Invalid(let reason) = error else {
                return XCTFail("unexpected error \(error)")
            }
            XCTAssertEqual(reason, "division by zero")
        }
    }

    func testCalculatorObject() throws {
        let calc = Calculator()
        XCTAssertEqual(try calc.evaluate(expression: "6 * 7"), "42")
        XCTAssertEqual(try calc.evaluate(expression: "ans / 2"), "21")
        XCTAssertEqual(
            calc.history(),
            [
                HistoryEntry(expression: "6 * 7", result: "42"),
                HistoryEntry(expression: "ans / 2", result: "21"),
            ])
        calc.clear()
        XCTAssertTrue(calc.history().isEmpty)
    }
}
