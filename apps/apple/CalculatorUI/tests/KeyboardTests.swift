import AppKit
import CalcFFI
import CalculatorUI
import SwiftUI
import XCTest

/// Drives the real CalculatorView in a real NSWindow with synthetic key
/// events, through AppKit's normal keyDown path.
@MainActor
final class KeyboardTests: XCTestCase {
    private func key(_ chars: String, code: UInt16, in window: NSWindow) {
        let event = NSEvent.keyEvent(
            with: .keyDown, location: .zero, modifierFlags: [], timestamp: ProcessInfo.processInfo.systemUptime,
            windowNumber: window.windowNumber, context: nil, characters: chars,
            charactersIgnoringModifiers: chars, isARepeat: false, keyCode: code)!
        NSApp.sendEvent(event)
        RunLoop.main.run(until: Date().addingTimeInterval(0.2))
    }

    func testKeyboardEvaluateAndEscape() {
        _ = NSApplication.shared
        NSApp.setActivationPolicy(.accessory)
        let model = CalculatorModel()
        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 340, height: 520),
            styleMask: [.titled], backing: .buffered, defer: false)
        window.contentView = NSHostingView(rootView: CalculatorView(model: model))
        window.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
        RunLoop.main.run(until: Date().addingTimeInterval(0.5))

        key("6", code: 22, in: window)
        key("*", code: 28, in: window)
        key("7", code: 26, in: window)
        XCTAssertEqual(model.view.expression, "6*7", "typing should reach Rust")
        key("\r", code: 36, in: window)
        XCTAssertEqual(model.view.display, "42", "Return should evaluate")

        key("1", code: 18, in: window)
        key("\u{1b}", code: 53, in: window)
        XCTAssertEqual(model.view.expression, "", "Escape should clear the expression")
        XCTAssertEqual(model.view.display, "0", "Escape should reset the display")
    }

    /// The Mac "delete" key (backspace) sends DEL (U+007F, key code 51);
    /// forward delete (fn+delete) sends NSDeleteFunctionKey (U+F728, 117).
    func testDeleteKeysRemoveLastCharacter() {
        _ = NSApplication.shared
        NSApp.setActivationPolicy(.accessory)
        let model = CalculatorModel()
        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 340, height: 520),
            styleMask: [.titled], backing: .buffered, defer: false)
        window.contentView = NSHostingView(rootView: CalculatorView(model: model))
        window.makeKeyAndOrderFront(nil)
        RunLoop.main.run(until: Date().addingTimeInterval(0.5))

        key("1", code: 18, in: window)
        key("2", code: 19, in: window)
        key("3", code: 20, in: window)
        key("\u{7f}", code: 51, in: window)
        XCTAssertEqual(model.view.expression, "12", "delete (backspace) should remove one character")
        key("\u{f728}", code: 117, in: window)
        XCTAssertEqual(model.view.expression, "1", "forward delete should remove one character")
    }
}
