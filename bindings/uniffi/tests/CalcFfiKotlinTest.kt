package dev.mashuda.calc

import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
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
    fun freeFunction() {
        assertEquals("7", evaluate("1 + 2 * 3"))
        assertEquals("0.3", evaluate("0.1 + 0.2"))
    }

    @Test
    fun errorsBecomeExceptions() {
        val e = assertThrows(CalcException.Invalid::class.java) { evaluate("1 / 0") }
        assertEquals("division by zero", e.reason)
    }

    @Test
    fun calculatorObject() {
        Calculator().use { calc ->
            assertEquals("42", calc.evaluate("6 * 7"))
            assertEquals("21", calc.evaluate("ans / 2"))
            assertEquals(
                listOf(HistoryEntry("6 * 7", "42"), HistoryEntry("ans / 2", "21")),
                calc.history(),
            )
        }
    }
}
