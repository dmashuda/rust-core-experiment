import AppKit
import CalculatorUI
import SwiftUI
import XCTest

@MainActor
final class CalculatorUITests: XCTestCase {
    /// The Swift side only forwards input; this checks the wiring, not the
    /// calculator logic (that is tested once, in Rust: //core/calc_ui).
    func testModelForwardsToRust() {
        let model = CalculatorModel()
        for id in ["2", "*", "(", "3", "+", "4", ")", "^", "2", "evaluate"] { model.press(id) }
        XCTAssertEqual(model.view.display, "98")
        XCTAssertTrue(model.keyInput("1"))
        XCTAssertFalse(model.keyInput("Tab"))
        XCTAssertEqual(model.view.expression, "1")
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
