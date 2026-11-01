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

    /** A database path of this test's own, inside Bazel's test sandbox. */
    private fun dbPath(name: String): String {
        val dir = System.getenv("TEST_TMPDIR") ?: System.getProperty("java.io.tmpdir")
        val file = File(dir, "$name.db")
        // A test runner that reuses TEST_TMPDIR would otherwise hand this
        // test the history the last run left behind.
        file.delete()
        File(dir, "$name.db-journal").delete()
        return file.absolutePath
    }

    @Test
    fun historyInSqliteSurvivesANewCalculator() {
        val path = dbPath("kotlin-history")
        CalculatorUi.open(path).use { ui ->
            assertTrue(ui.view().caption, ui.view().caption.startsWith("Evaluated by Rust, history in SQLite 3."))
            listOf("6", "*", "7", "evaluate").forEach { ui.press(it) }
        }

        // A second calculator over the same file: the Activity being recreated
        // after the process died.
        CalculatorUi.open(path).use { ui ->
            val vm = ui.view()
            assertEquals(listOf(HistoryLine(1uL, "6*7", "42")), vm.history)
            assertEquals("42", vm.display)
            ui.press("+")
            ui.press("1")
            assertEquals("43", ui.press("evaluate").display)
        }
    }

    @Test
    fun anUnusablePathStillGivesAWorkingCalculator() {
        CalculatorUi.open(dbPath("no/such/dir/history")).use { ui ->
            listOf("6", "*", "7", "evaluate").forEach { ui.press(it) }
            val vm = ui.view()
            assertEquals("42", vm.display)
            assertTrue(vm.caption, vm.caption.contains("memory ("))
        }
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
