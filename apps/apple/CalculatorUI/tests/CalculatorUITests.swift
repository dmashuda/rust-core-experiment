import AppKit
import CalculatorUI
import SwiftUI
import XCTest

@MainActor
final class CalculatorUITests: XCTestCase {
    func testModelDrivesRustCore() {
        let model = CalculatorModel()
        "2*(3+4)^2".forEach { model.append(String($0)) }
        model.evaluate()
        XCTAssertEqual(model.display, "98")
        XCTAssertFalse(model.isError)

        model.append("ans/7")
        model.evaluate()
        XCTAssertEqual(model.display, "14")
        XCTAssertEqual(model.history.map(\.result), ["14", "98"])

        model.append("1/0")
        model.evaluate()
        XCTAssertTrue(model.isError)
        XCTAssertEqual(model.display, "division by zero")
    }

    /// Renders the real view offscreen and saves a PNG into Bazel's undeclared
    /// test outputs (bazel-testlogs/.../test.outputs/), so the UI can be
    /// eyeballed without a display or screen-recording permission.
    func testRenderSnapshot() throws {
        let renderer = ImageRenderer(content: CalculatorView().frame(width: 340, height: 520))
        renderer.scale = 2
        let image = try XCTUnwrap(renderer.nsImage)
        let tiff = try XCTUnwrap(image.tiffRepresentation)
        let png = try XCTUnwrap(NSBitmapImageRep(data: tiff)?.representation(using: .png, properties: [:]))
        XCTAssertGreaterThan(png.count, 1000)

        if let dir = ProcessInfo.processInfo.environment["TEST_UNDECLARED_OUTPUTS_DIR"] {
            try png.write(to: URL(fileURLWithPath: dir).appendingPathComponent("CalculatorView.png"))
        }
    }
}
