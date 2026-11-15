package dev.mashuda.calc.app

import android.app.Activity
import android.content.res.Configuration
import android.os.Bundle
import android.view.KeyEvent
import android.view.WindowInsetsController
import dev.mashuda.calc.CalculatorUi
import java.io.File

/**
 * The Activity: lifecycle, and where the history file lives. What is on the
 * screen is drawn by [Renderer].
 */
class MainActivity : Activity() {
    private lateinit var ui: CalculatorUi
    private lateinit var renderer: Renderer

    private val dark get() =
        (resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK) == Configuration.UI_MODE_NIGHT_YES

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // A rotation or a light/dark switch recreates the Activity. Take the
        // Rust calculator from the outgoing instance so the expression,
        // display and history survive instead of silently resetting.
        // History is a SQLite file in the app's private storage, so it is
        // still there after the process is killed. Rust falls back to an
        // in-memory history (and says so in the footer) if it cannot be
        // opened, so there is nothing to handle here.
        ui = lastNonConfigurationInstance as? CalculatorUi
            ?: CalculatorUi.open(File(filesDir, "history.db").absolutePath)
        renderer = Renderer(this, ui)
        setContentView(renderer.root)
        // Dark status-bar icons on the light background (light mode only).
        if (!dark) {
            window.insetsController?.setSystemBarsAppearance(
                WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS,
                WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS,
            )
        }
        renderer.render(ui.view())
    }

    /** Hands the live Rust object to the Activity that replaces this one. */
    override fun onRetainNonConfigurationInstance(): Any = ui

    override fun onDestroy() {
        // On a configuration change the next Activity takes it over instead.
        if (!isChangingConfigurations) ui.close()
        super.onDestroy()
    }

    override fun onKeyDown(keyCode: Int, event: KeyEvent): Boolean =
        renderer.onKeyDown(keyCode, event) || super.onKeyDown(keyCode, event)
}
