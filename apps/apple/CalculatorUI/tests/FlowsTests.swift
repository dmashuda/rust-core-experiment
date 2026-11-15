import CalcFFI
import CalculatorUI
import XCTest

// The shared flows (//core/calc_flows:flows.json), played through
// `CalculatorModel`: the same calls the view makes, over a real SQLite file.
// Nothing here knows about arithmetic; a new flow needs no change.

private struct Flow: Decodable {
    let name: String
    let steps: [Step]
}

private struct Step: Decodable, CustomStringConvertible {
    let op: String
    let id: String?
    let key: String?
    let field: String?
    let value: String?

    var description: String {
        [op, id, key, field, value].compactMap { $0 }.joined(separator: " ")
    }
}

/// `field` of `vm` the way a flow spells it.
private func read(_ vm: ViewModel, _ field: String) -> String {
    switch field {
    case "expression": vm.expression
    case "display": vm.display
    case "error": String(vm.isError)
    case "history": vm.history.map { "\($0.expression) = \($0.result)" }.joined(separator: "\n")
    default: "unknown field \(field)"
    }
}

@MainActor
final class FlowsTests: XCTestCase {
    func testSharedFlows() throws {
        let url = try XCTUnwrap(Bundle(for: Self.self).url(forResource: "flows", withExtension: "json"))
        let flows = try JSONDecoder().decode([Flow].self, from: Data(contentsOf: url))
        XCTAssertFalse(flows.isEmpty)
        for flow in flows {
            play(flow)
        }
    }

    private func play(_ flow: Flow) {
        let path = NSTemporaryDirectory() + "calc-flow-\(UUID().uuidString).db"
        defer { try? FileManager.default.removeItem(atPath: path) }
        let model = CalculatorModel(databasePath: path)
        let keys = model.keypad.flatMap(\.keys).map(\.id)
        for (index, step) in flow.steps.enumerated() {
            let place = "\"\(flow.name)\", step \(index + 1) (\(step))"
            switch step.op {
            case "press":
                guard let id = step.id, keys.contains(id) else { return XCTFail("\(place): no such key") }
                model.press(id)
            case "key":
                guard model.keyInput(step.key ?? "") else { return XCTFail("\(place): the model ignored it") }
            case "ignored":
                guard !model.keyInput(step.key ?? "") else { return XCTFail("\(place): the model took it") }
            case "expect":
                let actual = read(model.view, step.field ?? "")
                guard actual == step.value else { return XCTFail("\(place): it is \"\(actual)\"") }
            default:
                return XCTFail("\(place): unknown step")
            }
        }
    }
}
