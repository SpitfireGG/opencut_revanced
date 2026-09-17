// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Jareer and Concat contributors

package app.concat.editor;

import android.app.NativeActivity;
import android.graphics.Color;
import android.os.Build;
import android.os.Bundle;
import android.view.View;
import android.view.Window;
import android.view.WindowInsets;
import android.view.WindowInsetsController;

/**
 * Custom activity that extends NativeActivity to enable edge-to-edge display.
 * This ensures the window draws behind the status bar and navigation bar,
 * and properly reports safe area insets to Slint.
 * Uses only platform APIs (no AndroidX dependency).
 */
public class ConcatActivity extends NativeActivity {
    @Override
    protected void onCreate(Bundle savedInstanceState) {
        // Enable edge-to-edge display: the window draws behind system bars
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            getWindow().setDecorFitsSystemWindows(false);
        } else {
            // For older APIs, use legacy flags
            getWindow().getDecorView().setSystemUiVisibility(
                View.SYSTEM_UI_FLAG_LAYOUT_STABLE
                | View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
                | View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
            );
        }
        
        super.onCreate(savedInstanceState);
        
        // Configure window to draw behind status bar and navigation bar
        Window window = getWindow();
        window.setStatusBarColor(Color.TRANSPARENT);
        window.setNavigationBarColor(Color.TRANSPARENT);
        
        // For API 30+, use WindowInsetsController for better control
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            WindowInsetsController insetsController = window.getInsetsController();
            if (insetsController != null) {
                // Don't hide the status bar, just make it translucent
                insetsController.setSystemBarsBehavior(
                    WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
                );
            }
        }
        // Legacy flags already set above for API < 30
    }
}