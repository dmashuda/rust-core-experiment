package dev.mashuda.calc.app

import android.app.Activity
import android.content.res.Configuration
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.os.Bundle
import android.util.TypedValue
import android.view.Gravity
import android.view.ViewGroup.LayoutParams.MATCH_PARENT
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.view.WindowInsetsController
import android.widget.Button
import android.widget.GridLayout
import android.widget.LinearLayout
import android.widget.TextView
import dev.mashuda.calc.CalcException
import dev.mashuda.calc.Calculator

/** Calculator UI in plain Android Views; every evaluation runs in Rust. */
class MainActivity : Activity() {
    private lateinit var calculator: Calculator
    private lateinit var exprView: TextView
    private lateinit var resultView: TextView
    private lateinit var historyView: TextView
    private var expression = ""

    private data class Key(val label: String, val insert: String? = null, val op: Boolean = false, val span: Int = 1)

    private val keys = listOf(
        Key("C"), Key("(", "("), Key(")", ")"), Key("÷", "/", op = true),
        Key("7", "7"), Key("8", "8"), Key("9", "9"), Key("×", "*", op = true),
        Key("4", "4"), Key("5", "5"), Key("6", "6"), Key("−", "-", op = true),
        Key("1", "1"), Key("2", "2"), Key("3", "3"), Key("+", "+", op = true),
        Key("0", "0"), Key(".", "."), Key("^", "^"), Key("⌫"),
        Key("ans", "ans"), Key("%", "%"), Key("=", op = true, span = 2),
    )

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        calculator = Calculator()
        setContentView(buildUi())
        // Dark status-bar icons on the light background (light mode only).
        if (!isDarkMode()) {
            window.insetsController?.setSystemBarsAppearance(
                WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS,
                WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS,
            )
        }
    }

    override fun onDestroy() {
        calculator.close()
        super.onDestroy()
    }

    private fun buildUi(): LinearLayout {
        val dark = isDarkMode()
        val bg = if (dark) Color.rgb(9, 9, 11) else Color.rgb(244, 244, 245)
        val fg = if (dark) Color.rgb(250, 250, 250) else Color.rgb(24, 24, 27)
        val muted = if (dark) Color.rgb(161, 161, 170) else Color.rgb(113, 113, 122)
        val keyBg = if (dark) Color.rgb(39, 39, 42) else Color.rgb(228, 228, 231)
        val opBg = Color.rgb(251, 146, 60)

        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(bg)
            setPadding(dp(20), dp(48), dp(20), dp(20))
        }
        exprView = TextView(this).apply {
            gravity = Gravity.END
            setTextColor(muted)
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 20f)
            minHeight = dp(28)
        }
        resultView = TextView(this).apply {
            text = "0"
            gravity = Gravity.END
            setTextColor(fg)
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 48f)
            typeface = Typeface.create("sans-serif-light", Typeface.NORMAL)
            maxLines = 1
        }
        root.addView(exprView, LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT))
        root.addView(resultView, LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT))

        val grid = GridLayout(this).apply { columnCount = 4 }
        for (key in keys) {
            val button = Button(this).apply {
                text = key.label
                isAllCaps = false
                setTextSize(TypedValue.COMPLEX_UNIT_SP, 22f)
                setTextColor(if (key.op) Color.WHITE else fg)
                background = GradientDrawable().apply {
                    cornerRadius = dp(14).toFloat()
                    setColor(if (key.op) opBg else keyBg)
                }
                stateListAnimator = null
                setOnClickListener { press(key) }
            }
            val params = GridLayout.LayoutParams(
                GridLayout.spec(GridLayout.UNDEFINED),
                GridLayout.spec(GridLayout.UNDEFINED, key.span, 1f),
            ).apply {
                width = 0
                height = dp(64)
                setMargins(dp(4), dp(4), dp(4), dp(4))
            }
            grid.addView(button, params)
        }
        root.addView(grid, LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT).apply { topMargin = dp(12) })

        historyView = TextView(this).apply {
            setTextColor(muted)
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 15f)
            typeface = Typeface.MONOSPACE
        }
        root.addView(historyView, LinearLayout.LayoutParams(MATCH_PARENT, 0, 1f).apply { topMargin = dp(16) })

        root.addView(TextView(this).apply {
            text = "Evaluated by Rust via UniFFI + JNA"
            gravity = Gravity.CENTER
            setTextColor(muted)
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 12f)
        })
        return root
    }

    private fun press(key: Key) {
        when {
            key.label == "C" -> setExpression("")
            key.label == "⌫" -> setExpression(expression.dropLast(1))
            key.label == "=" -> evaluate()
            else -> setExpression(expression + key.insert)
        }
    }

    private fun setExpression(next: String) {
        expression = next
        exprView.text = expression
    }

    private fun evaluate() {
        try {
            resultView.text = calculator.evaluate(expression)
            resultView.setTextSize(TypedValue.COMPLEX_UNIT_SP, 48f)
            resultView.setTextColor(if (isDarkMode()) Color.rgb(250, 250, 250) else Color.rgb(24, 24, 27))
            historyView.text = calculator.history().reversed()
                .joinToString("\n") { "${it.expression} = ${it.result}" }
            setExpression("")
        } catch (e: CalcException.Invalid) {
            resultView.text = e.reason
            resultView.setTextSize(TypedValue.COMPLEX_UNIT_SP, 22f)
            resultView.setTextColor(Color.rgb(220, 38, 38))
        }
    }

    private fun isDarkMode(): Boolean =
        (resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK) == Configuration.UI_MODE_NIGHT_YES

    private fun dp(v: Int): Int = (v * resources.displayMetrics.density).toInt()
}
