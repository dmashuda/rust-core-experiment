import AppKit
import CalcFFI
import CalculatorUI
import SwiftUI
import XCTest

// The renderer itself: the real CalculatorView in a real (offscreen) window,
// read back and driven through the accessibility tree, the way VoiceOver or
// an XCUITest sees it. The shared flows cover what Rust does; this covers
// the mapping from the view model and keypad to widgets, which they do not.
// (Hardware keys are KeyboardTests'.)

/// The real `CalculatorUi`, with a note of every key the view pressed.
private final class Recorder: CalculatorUiProtocol, @unchecked Sendable {
    let real = CalculatorUi()
    var presses: [String] = []

    func press(id: String) -> ViewModel {
        presses.append(id)
        return real.press(id: id)
    }

    func keyInput(key: String) -> ViewModel? { real.keyInput(key: key) }
    func view() -> ViewModel { real.view() }
}

/// One element of the accessibility tree. SwiftUI's elements are plain
/// `NSObject`s answering the informal NSAccessibility methods, so they are
/// read by key rather than through the protocol.
private struct Element {
    let object: NSObject

    private func attribute(_ key: String) -> Any? {
        let getter = "is" + key.prefix(1).uppercased() + key.dropFirst()
        guard object.responds(to: Selector(key)) || object.responds(to: Selector(getter)) else { return nil }
        return object.value(forKey: key)
    }

    var id: String { attribute("accessibilityIdentifier") as? String ?? "" }
    var role: String { attribute("accessibilityRole") as? String ?? "" }
    var enabled: Bool { attribute("accessibilityEnabled") as? Bool ?? false }
    var children: [Element] { (attribute("accessibilityChildren") as? [NSObject] ?? []).map(Element.init) }

    /// What it says: its label, title and value.
    var strings: [String] {
        ["accessibilityLabel", "accessibilityTitle", "accessibilityValue"].compactMap { attribute($0) as? String }
            .filter { !$0.isEmpty }
    }

    /// It and everything inside it, depth first.
    var all: [Element] { [self] + children.flatMap(\.all) }

    func says(_ text: String) -> Bool {
        all.contains { $0.strings.contains { $0.contains(text) } }
    }

    /// The first element inside it (or it) with this role.
    func first(_ role: String) -> Element? {
        all.first { $0.role == role }
    }

    /// What VoiceOver does to activate it.
    func press() {
        _ = object.perform(NSSelectorFromString("accessibilityPerformPress"))
    }

    func dump(_ depth: Int = 0) -> String {
        let line = String(repeating: "  ", count: depth) + "\(role) id=\(id) \(strings) enabled=\(enabled)"
        return ([line] + children.map { $0.dump(depth + 1) }).joined(separator: "\n")
    }
}

@MainActor
final class RendererTests: XCTestCase {
    override class func setUp() {
        super.setUp()
        _ = NSApplication.shared
        NSApp.setActivationPolicy(.accessory)
        // SwiftUI builds its accessibility tree only once something asks for
        // it, as VoiceOver does by setting this on the app. (The setter is
        // the old, deprecated one; the attribute has no other.)
        _ = NSApp.perform(
            NSSelectorFromString("accessibilitySetValue:forAttribute:"), with: NSNumber(value: true),
            with: "AXEnhancedUserInterface")
        NSApp.finishLaunching()
    }

    private var window: NSWindow!
    private var model: CalculatorModel!

    private func settle() {
        RunLoop.main.run(until: Date().addingTimeInterval(0.03))
    }

    private func element(_ id: String) -> Element? {
        Element(object: window).all.first { $0.id == id }
    }

    /// Every key on the keypad (from Rust, so a new key is included without
    /// editing this) is a button under its id that shows its label and
    /// reports a press with its id; after each press the view shows the
    /// view model Rust returned.
    func testEveryKeyIsDrawnAndWired() throws {
        let recorder = Recorder()
        model = CalculatorModel(ui: recorder)
        window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 340, height: 640), styleMask: [.titled], backing: .buffered,
            defer: false)
        window.contentView = NSHostingView(rootView: CalculatorView(model: model))
        window.orderFront(nil)
        addTeardownBlock { @MainActor in self.window.close() }
        settle()
        checkDisplay("at start")

        let keys = model.keypad.flatMap(\.keys)
        XCTAssertFalse(keys.isEmpty)
        // Every key in keypad order, then a sum, so the history has a line.
        for id in keys.map(\.id) + ["clear", "6", "*", "7", "evaluate"] {
            let key = try XCTUnwrap(keys.first { $0.id == id })
            let place = "key \(id)"
            guard let element = element(id), let button = element.first("AXButton") else {
                XCTFail("\(place): not drawn as a button\n\(Element(object: window).dump())")
                continue
            }
            XCTAssert(element.says(key.label), "\(place): does not show \"\(key.label)\"\n\(element.dump())")
            XCTAssertTrue(button.enabled, "\(place): enabled")
            recorder.presses = []
            button.press()
            settle()
            XCTAssertEqual(recorder.presses, [id], "\(place): pressed")
            checkDisplay("after \(place)")
        }
        XCTAssertFalse(model.view.history.isEmpty, "the sum went into the history, so its drawing was checked")
    }

    /// The parts of the view model the view shows, each under its field's name.
    private func checkDisplay(_ place: String) {
        let vm = model.view
        for (id, text) in [("expression", vm.expression), ("display", vm.display), ("caption", vm.caption)] {
            guard let element = element(id) else {
                XCTFail("\(place): no \(id)")
                continue
            }
            XCTAssert(text.isEmpty || element.says(text), "\(place): \(id) does not show \"\(text)\"\n\(element.dump())")
        }
        if vm.history.isEmpty { return }
        guard let history = element("history") else { return XCTFail("\(place): no history") }
        for line in vm.history {
            XCTAssert(history.says(line.expression) && history.says(line.result),
                      "\(place): history does not show \(line)\n\(history.dump())")
        }
    }
}
