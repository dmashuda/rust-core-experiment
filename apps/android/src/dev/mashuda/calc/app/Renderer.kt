package dev.mashuda.calc.app

import android.content.Context
import android.content.res.Configuration
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.util.TypedValue
import android.view.Gravity
import android.view.KeyEvent
import android.view.View
import android.view.ViewGroup.LayoutParams.MATCH_PARENT
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.widget.Button
import android.widget.GridLayout
import android.widget.LinearLayout
import android.widget.TextView
import dev.mashuda.calc.CalculatorUiInterface
import dev.mashuda.calc.KeyStyle
import dev.mashuda.calc.ViewModel
import dev.mashuda.calc.keypad
import dev.mashuda.calc.keypadColumns

/**
 * Renderer only: draws the Rust ViewModel with plain Android Views and
 * forwards input. Keypad layout, editing, evaluation and history all live in
 * //core/calc_ui.
 *
 * Each key's button carries the key's id as its tag, and each part of the
 * display the name of the ViewModel field it shows. Apart from the Activity
 * (its lifecycle), so a test can build it with any Context.
 */
class Renderer(private val context: Context, private val ui: CalculatorUiInterface) {
    private val dark get() =
        (context.resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK) == Configuration.UI_MODE_NIGHT_YES
    private val fg get() = if (dark) Color.rgb(250, 250, 250) else Color.rgb(24, 24, 27)
    private val muted get() = if (dark) Color.rgb(161, 161, 170) else Color.rgb(113, 113, 122)

    private val exprView = TextView(context).apply {
        tag = "expression"
        gravity = Gravity.END
        setTextColor(muted)
        setTextSize(TypedValue.COMPLEX_UNIT_SP, 20f)
        minHeight = dp(28)
    }
    private val resultView = TextView(context).apply {
        tag = "display"
        gravity = Gravity.END
        typeface = Typeface.create("sans-serif-light", Typeface.NORMAL)
        maxLines = 1
    }
    private val historyView = TextView(context).apply {
        tag = "history"
        setTextColor(muted)
        setTextSize(TypedValue.COMPLEX_UNIT_SP, 15f)
        typeface = Typeface.MONOSPACE
    }
    private val captionView = TextView(context).apply {
        tag = "caption"
        gravity = Gravity.CENTER
        setTextColor(muted)
        setTextSize(TypedValue.COMPLEX_UNIT_SP, 12f)
    }

    /** The whole layout, for the Activity's content view. Filled by [render]. */
    val root: View = LinearLayout(context).apply {
        orientation = LinearLayout.VERTICAL
        setBackgroundColor(if (dark) Color.rgb(9, 9, 11) else Color.rgb(244, 244, 245))
        setPadding(dp(20), dp(48), dp(20), dp(20))
        addView(exprView, LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT))
        addView(resultView, LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT))
        addView(buildKeypad(), LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT).apply { topMargin = dp(12) })
        addView(historyView, LinearLayout.LayoutParams(MATCH_PARENT, 0, 1f).apply { topMargin = dp(16) })
        addView(captionView)
    }

    /**
     * A hardware key, from the Activity's onKeyDown. False if Rust does not
     * use it, so the Activity can let it propagate.
     */
    fun onKeyDown(keyCode: Int, event: KeyEvent): Boolean {
        val key = when (keyCode) {
            KeyEvent.KEYCODE_ENTER, KeyEvent.KEYCODE_NUMPAD_ENTER -> "Enter"
            KeyEvent.KEYCODE_DEL -> "Backspace"
            KeyEvent.KEYCODE_ESCAPE -> "Escape"
            else -> event.unicodeChar.takeIf { it != 0 }?.toChar()?.toString()
        }
        val vm = key?.let { ui.keyInput(it) } ?: return false
        render(vm)
        return true
    }

    /** The keypad never changes, so its buttons are created once. */
    private fun buildKeypad(): GridLayout {
        val grid = GridLayout(context)
        val rows = keypad()
        grid.columnCount = keypadColumns().toInt()
        for (row in rows) {
            for (key in row.keys) {
                val isOperator = key.style == KeyStyle.OPERATOR
                val button = Button(context).apply {
                    tag = key.id
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

    fun render(vm: ViewModel) {
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

    private fun dp(v: Int): Int = (v * context.resources.displayMetrics.density).toInt()
}
