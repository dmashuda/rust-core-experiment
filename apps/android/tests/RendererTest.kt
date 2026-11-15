package dev.mashuda.calc.app

import android.app.Activity
import android.os.Looper
import android.view.KeyEvent
import android.view.View
import android.widget.Button
import android.widget.TextView
import dev.mashuda.calc.CalculatorUi
import dev.mashuda.calc.CalculatorUiInterface
import dev.mashuda.calc.ViewModel
import dev.mashuda.calc.keypad
import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config

/**
 * The renderer itself: the real [Renderer] drawing into real Views
 * (Robolectric's Android framework, on the host JVM), found by tag and
 * clicked or typed at. The shared flows cover what Rust does; this covers
 * the mapping from the view model, keypad and hardware keys to widgets,
 * which they do not.
 */
@RunWith(RendererTest.Offline::class)
@Config(sdk = [34])
class RendererTest {
    /**
     * Robolectric with the Android framework jar Bazel fetched, so it never
     * downloads one, and with UniFFI pointed at the Bazel-built library.
     */
    class Offline(test: Class<*>) : RobolectricTestRunner(test.also { configure() })

    companion object {
        fun configure() {
            System.setProperty("robolectric.offline", "true")
            System.setProperty("robolectric.dependency.dir", File(System.getenv("ANDROID_ALL")!!).absoluteFile.parent)
            val lib = File(System.getenv("CALC_FFI_LIB")!!).absolutePath
            System.setProperty("uniffi.component.calc_ffi.libraryOverride", lib)
        }
    }

    /** The real `CalculatorUi`, with a note of every key the widgets reported. */
    private class Recorder : CalculatorUiInterface {
        val real = CalculatorUi()
        val presses = mutableListOf<String>()
        val keys = mutableListOf<String>()
        var last: ViewModel = real.view()
        /** Whether Rust used the last hardware key. */
        var took = false

        override fun press(id: String): ViewModel {
            presses += id
            return real.press(id).also { last = it }
        }

        override fun keyInput(key: String): ViewModel? {
            keys += key
            return real.keyInput(key).also { took = it != null }?.also { last = it }
        }

        override fun view(): ViewModel = real.view()
    }

    private val recorder = Recorder()
    private val failures = mutableListOf<String>()
    private lateinit var renderer: Renderer
    private lateinit var activity: Activity

    private fun settle() = shadowOf(Looper.getMainLooper()).idle()

    private fun <T : View> find(tag: String): T? = activity.window.decorView.findViewWithTag(tag)

    private fun setUp() {
        activity = Robolectric.buildActivity(Activity::class.java).setup().get()
        renderer = Renderer(activity, recorder)
        activity.setContentView(renderer.root)
        renderer.render(recorder.view())
        settle()
    }

    /**
     * Every key on the keypad (from Rust, so a new key is included without
     * editing this) is a button under its id that shows its label and
     * reports a click with its id; after each one the view shows the view
     * model Rust returned.
     */
    @Test
    fun everyKeyIsDrawnAndWired() {
        setUp()
        checkDisplay("at start")
        val keys = keypad().flatMap { it.keys }
        // Every key in keypad order, then a sum, so the history has a line.
        for (id in keys.map { it.id } + listOf("clear", "6", "*", "7", "evaluate")) {
            val key = keys.first { it.id == id }
            val place = "key $id"
            val button = find<View>(id) as? Button
            if (button == null) {
                failures += "$place: not drawn as a button"
                continue
            }
            if (button.text.toString() != key.label) failures += "$place: shows \"${button.text}\", not \"${key.label}\""
            if (!button.isEnabled) failures += "$place: disabled"
            recorder.presses.clear()
            button.performClick()
            settle()
            if (recorder.presses != listOf(id)) failures += "$place: clicked, reported ${recorder.presses}"
            checkDisplay("after $place")
        }
        if (recorder.last.history.isEmpty()) failures += "the sum did not go into the history, so its drawing went unchecked"
        assertEquals("", failures.joinToString("\n"))
    }

    /** Each hardware key reaches Rust as the name `key_input` takes. */
    @Test
    fun hardwareKeysAreNamedForRust() {
        setUp()
        val named = listOf(
            KeyEvent.KEYCODE_7 to "7",
            KeyEvent.KEYCODE_ENTER to "Enter",
            KeyEvent.KEYCODE_NUMPAD_ENTER to "Enter",
            KeyEvent.KEYCODE_DEL to "Backspace",
            KeyEvent.KEYCODE_ESCAPE to "Escape",
            KeyEvent.KEYCODE_PLUS to "+",
        )
        for ((code, name) in named) {
            recorder.keys.clear()
            val handled = renderer.onKeyDown(code, KeyEvent(KeyEvent.ACTION_DOWN, code))
            if (recorder.keys != listOf(name)) failures += "key code $code: reported ${recorder.keys}, not [$name]"
            if (handled != recorder.took) failures += "key code $code: handled is $handled, Rust took it: ${recorder.took}"
            checkDisplay("after key code $code")
        }
        // A key Rust does not use is left to the Activity.
        recorder.keys.clear()
        if (renderer.onKeyDown(KeyEvent.KEYCODE_TAB, KeyEvent(KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_TAB))) {
            failures += "Tab was taken, reported ${recorder.keys}"
        }
        if (renderer.onKeyDown(KeyEvent.KEYCODE_SHIFT_LEFT, KeyEvent(KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_SHIFT_LEFT))) {
            failures += "Shift was taken"
        }
        assertEquals("", failures.joinToString("\n"))
    }

    /** The parts of the view model the view shows, each under its field's name. */
    private fun checkDisplay(place: String) {
        val vm = recorder.last
        val history = vm.history.joinToString("\n") { "${it.expression} = ${it.result}" }
        for ((tag, text) in listOf("expression" to vm.expression, "display" to vm.display, "history" to history)) {
            val shown = find<View>(tag) as? TextView
            if (shown == null) failures += "$place: no $tag" else if (shown.text.toString() != text) {
                failures += "$place: $tag shows \"${shown.text}\", not \"$text\""
            }
        }
        val caption = (find<View>("caption") as? TextView)?.text.toString()
        if (!caption.contains(vm.caption)) failures += "$place: caption shows \"$caption\", not \"${vm.caption}\""
    }
}
