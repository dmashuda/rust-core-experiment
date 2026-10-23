import CalculatorUI
import SwiftUI

@main
struct CalculatorApp: App {
    var body: some Scene {
        WindowGroup("Rust Calculator") {
            CalculatorView()
                .frame(minWidth: 320, idealWidth: 340, minHeight: 520)
        }
        .defaultSize(width: 340, height: 560)
        .windowResizability(.contentSize)
    }
}
