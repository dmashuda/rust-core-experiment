package dev.mashuda.calc.app

import com.google.gson.JsonObject
import com.google.gson.JsonParser
import dev.mashuda.calc.CalculatorUi
import dev.mashuda.calc.ViewModel
import dev.mashuda.calc.keypad
import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.BeforeClass
import org.junit.Test

/**
 * Plays the shared flows (//core/calc_flows:flows.json) through
 * [CalculatorUi] on the host JVM: the same calls the Android renderer
 * makes (Kotlin -> JNA -> Rust), over a real SQLite file.
 *
 * Nothing here knows about arithmetic; a new flow needs no change.
 */
class CalcFlowsTest {
    companion object {
        @JvmStatic
        @BeforeClass
        fun loadRust() {
            // Point UniFFI's loader at the Bazel-built library (runfiles path).
            val lib = File(System.getenv("CALC_FFI_LIB")!!).absolutePath
            System.setProperty("uniffi.component.calc_ffi.libraryOverride", lib)
        }
    }

    /** `field` of `vm` the way a flow spells it. */
    private fun read(vm: ViewModel, field: String): String = when (field) {
        "expression" -> vm.expression
        "display" -> vm.display
        "error" -> vm.isError.toString()
        "history" -> vm.history.joinToString("\n") { "${it.expression} = ${it.result}" }
        else -> "unknown field $field"
    }

    @Test
    fun sharedFlows() {
        val flows = JsonParser.parseString(File(System.getenv("CALC_FLOWS")!!).readText()).asJsonArray
        assertTrue(flows.size() > 0)
        val keys = keypad().flatMap { it.keys }.map { it.id }
        val failures = mutableListOf<String>()
        flows.forEachIndexed { n, element ->
            val flow = element.asJsonObject
            val name = flow["name"].asString
            val db = File(System.getenv("TEST_TMPDIR") ?: System.getProperty("java.io.tmpdir"), "flow-$n.db")
            db.delete()
            CalculatorUi.open(db.absolutePath).use { ui ->
                var vm = ui.view()
                for ((index, s) in flow["steps"].asJsonArray.withIndex()) {
                    val step = s as JsonObject
                    val place = "\"$name\", step ${index + 1} ($step)"
                    fun str(key: String) = step[key]?.asString ?: ""
                    val why: String? = when (str("op")) {
                        "press" -> if (str("id") !in keys) {
                            "no such key"
                        } else {
                            vm = ui.press(str("id"))
                            null
                        }
                        "key" -> when (val next = ui.keyInput(str("key"))) {
                            null -> "the calculator ignored it"
                            else -> {
                                vm = next
                                null
                            }
                        }
                        "ignored" -> if (ui.keyInput(str("key")) != null) "the calculator took it" else null
                        "expect" -> read(vm, str("field")).takeIf { it != str("value") }?.let { "it is \"$it\"" }
                        else -> "unknown step"
                    }
                    if (why != null) {
                        failures += "$place: $why"
                        break
                    }
                }
            }
        }
        assertEquals("", failures.joinToString("\n"))
    }
}
