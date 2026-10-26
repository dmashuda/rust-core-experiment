package dev.mashuda.calc.app

import android.app.Activity
import android.content.res.Configuration
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.os.Bundle
import android.util.TypedValue
import android.view.Gravity
import android.view.KeyEvent
import android.view.ViewGroup.LayoutParams.MATCH_PARENT
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.view.WindowInsetsController
import android.widget.Button
import android.widget.GridLayout
import android.widget.LinearLayout
import android.widget.TextView
import dev.mashuda.calc.CalculatorUi
import dev.mashuda.calc.KeyStyle
import dev.mashuda.calc.ViewModel
import dev.mashuda.calc.keypad

/**
 * Renderer only: draws the Rust ViewModel with plain Android Views and
 * forwards input. Keypad layout, editing, evaluation and history all live in
 * //core/calc_ui.
 */
class MainActivity : Activity() {
    private lateinit var ui: CalculatorUi
    private lateinit var exprView: TextView
    private lateinit var resultView: TextView
    private lateinit var keypad: GridLayout
    private lateinit var historyView: TextView
    private lateinit var captionView: TextView

    private val dark get() =
        (resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK) == Configuration.UI_MODE_NIGHT_YES
    private val fg get() = if (dark) Color.rgb(250, 250, 250) else Color.rgb(24, 24, 27)
    private val muted get() = if (dark) Color.rgb(161, 161, 170) else Color.rgb(113, 113, 122)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // A rotation or a light/dark switch recreates the Activity. Take the
        // Rust calculator from the outgoing instance so the expression,
        // display and history survive instead of silently resetting.
        ui = lastNonConfigurationInstance as? CalculatorUi ?: CalculatorUi()
        setContentView(buildLayout())
        // Dark status-bar icons on the light background (light mode only).
        if (!dark) {
            window.insetsController?.setSystemBarsAppearance(
                WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS,
                WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS,
            )
        }
        render(ui.view())
    }

    /** Hands the live Rust object to the Activity that replaces this one. */
    override fun onRetainNonConfigurationInstance(): Any = ui

    override fun onDestroy() {
        // On a configuration change the next Activity takes it over instead.
        if (!isChangingConfigurations) ui.close()
        super.onDestroy()
    }

    override fun onKeyDown(keyCode: Int, event: KeyEvent): Boolean {
        val key = when (keyCode) {
            KeyEvent.KEYCODE_ENTER, KeyEvent.KEYCODE_NUMPAD_ENTER -> "Enter"
            KeyEvent.KEYCODE_DEL -> "Backspace"
            KeyEvent.KEYCODE_ESCAPE -> "Escape"
            else -> event.unicodeChar.takeIf { it != 0 }?.toChar()?.toString()
        }
        val vm = key?.let { ui.keyInput(it) } ?: return super.onKeyDown(keyCode, event)
        render(vm)
        return true
    }

    /** Static scaffolding; everything with content is filled by [render]. */
    private fun buildLayout(): LinearLayout {
        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(if (dark) Color.rgb(9, 9, 11) else Color.rgb(244, 244, 245))
            setPadding(dp(20), dp(48), dp(20), dp(20))
        }
        exprView = TextView(this).apply {
            gravity = Gravity.END
            setTextColor(muted)
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 20f)
            minHeight = dp(28)
        }
        resultView = TextView(this).apply {
            gravity = Gravity.END
            typeface = Typeface.create("sans-serif-light", Typeface.NORMAL)
            maxLines = 1
        }
        keypad = buildKeypad()
        historyView = TextView(this).apply {
            setTextColor(muted)
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 15f)
            typeface = Typeface.MONOSPACE
        }
        captionView = TextView(this).apply {
            gravity = Gravity.CENTER
            setTextColor(muted)
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 12f)
        }
        root.addView(exprView, LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT))
        root.addView(resultView, LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT))
        root.addView(keypad, LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT).apply { topMargin = dp(12) })
        root.addView(historyView, LinearLayout.LayoutParams(MATCH_PARENT, 0, 1f).apply { topMargin = dp(16) })
        root.addView(captionView)
        return root
    }

    /** The keypad never changes, so its buttons are created once. */
    private fun buildKeypad(): GridLayout {
        val grid = GridLayout(this)
        val rows = keypad()
        grid.columnCount = rows.maxOf { row -> row.keys.sumOf { it.span.toInt() } }
        for (row in rows) {
            for (key in row.keys) {
                val isOperator = key.style == KeyStyle.OPERATOR
                val button = Button(this).apply {
                    text = key.label
                    isAllCaps = false
                    setTextSize(TypedValue.COMPLEX_UNIT_SP, 22f)
                    setTextColor(if (isOperator) Color.WHITE else fg)
                    background = GradientDrawable().apply {
                        cornerRadius = dp(14).toFloat()
                        setColor(keyColor(key.style))
                    }
                    stateListAnimator = null
                    setOnClickListener { render(ui.press(key.id)) }
                }
                val params = GridLayout.LayoutParams(
                    GridLayout.spec(GridLayout.UNDEFINED),
                    GridLayout.spec(GridLayout.UNDEFINED, key.span.toInt(), 1f),
                ).apply {
                    width = 0
                    height = dp(64)
                    setMargins(dp(4), dp(4), dp(4), dp(4))
                }
                grid.addView(button, params)
            }
        }
        return grid
    }

    private fun render(vm: ViewModel) {
        exprView.text = vm.expression
        resultView.text = vm.display
        resultView.setTextSize(TypedValue.COMPLEX_UNIT_SP, if (vm.isError) 22f else 48f)
        resultView.setTextColor(if (vm.isError) Color.rgb(220, 38, 38) else fg)
        historyView.text = vm.history.joinToString("\n") { "${it.expression} = ${it.result}" }
        captionView.text = "${vm.caption} (UniFFI + JNA)"
    }

    private fun keyColor(style: KeyStyle): Int = when (style) {
        KeyStyle.OPERATOR -> Color.rgb(251, 146, 60)
        KeyStyle.FUNCTION -> if (dark) Color.rgb(63, 63, 70) else Color.rgb(212, 212, 216)
        KeyStyle.INPUT -> if (dark) Color.rgb(39, 39, 42) else Color.rgb(228, 228, 231)
    }

    private fun dp(v: Int): Int = (v * resources.displayMetrics.density).toInt()
}
