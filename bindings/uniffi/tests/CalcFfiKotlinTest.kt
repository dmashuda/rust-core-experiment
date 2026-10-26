package dev.mashuda.calc

import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
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
            assertEquals(listOf(HistoryLine(1uL, "6*7", "42")), vm.history)
        }
    }

    @Test
    fun errorsAreViewStateAndEditingRestoresTheResult() {
        CalculatorUi().use { ui ->
            listOf("6", "*", "7", "evaluate", "1", "/", "0").forEach { ui.press(it) }
            val error = ui.press("evaluate")
            assertTrue(error.isError)
            assertEquals("division by zero", error.display)

            val edited = ui.press("backspace")
            assertFalse(edited.isError)
            assertEquals("42", edited.display)
        }
    }

    @Test
    fun operatorsContinueFromTheLastResult() {
        CalculatorUi().use { ui ->
            listOf("6", "*", "7", "evaluate").forEach { ui.press(it) }
            assertEquals("ans+", ui.press("+").expression)
            ui.press("1")
            assertEquals("43", ui.press("evaluate").display)
        }
    }

    @Test
    fun keypadIsFetchedOnceAndNotPartOfTheViewModel() {
        val rows = keypad()
        assertEquals("C", rows.first().keys.first().label)
        assertEquals(KeyStyle.OPERATOR, rows.last().keys.last().style)
        assertEquals(2u, rows.last().keys.last().span)
        assertEquals(rows, keypad())
    }

    @Test
    fun keyboardInput() {
        CalculatorUi().use { ui ->
            assertNull(ui.keyInput("Tab"))
            ui.keyInput("9")
            assertEquals("9", ui.keyInput("Enter")?.display)
        }
    }
}
