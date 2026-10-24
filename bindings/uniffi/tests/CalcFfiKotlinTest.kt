package dev.mashuda.calc

import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.BeforeClass
import org.junit.Test

class CalcFfiKotlinTest {
    companion object {
        @JvmStatic
        @BeforeClass
        fun loadRust() {
            // Point UniFFI's loader at the Bazel-built library (runfiles path).
            val lib = File(System.getenv("CALC_FFI_LIB")).absolutePath
            System.setProperty("uniffi.component.calc_ffi.libraryOverride", lib)
        }
    }

    @Test
    fun pressingKeysEvaluates() {
        CalculatorUi().use { ui ->
            listOf("6", "*", "7").forEach { ui.press(it) }
            val vm = ui.press("evaluate")
            assertEquals("42", vm.display)
            assertEquals(listOf(HistoryLine("6*7", "42")), vm.history)
        }
    }

    @Test
    fun errorsAreViewState() {
        CalculatorUi().use { ui ->
            listOf("1", "/", "0").forEach { ui.press(it) }
            val vm = ui.press("evaluate")
            assertTrue(vm.isError)
            assertEquals("division by zero", vm.display)
        }
    }

    @Test
    fun keypadAndKeyboard() {
        CalculatorUi().use { ui ->
            val keypad = ui.view().keypad
            assertEquals("C", keypad.first().keys.first().label)
            assertEquals(KeyStyle.OPERATOR, keypad.last().keys.last().style)
            assertNull(ui.keyInput("Tab"))
            ui.keyInput("9")
            assertEquals("9", ui.keyInput("Enter")?.display)
        }
    }
}
