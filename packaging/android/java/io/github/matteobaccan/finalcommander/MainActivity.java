// FinalCommander -- an orthodox file manager for the terminal.
// Copyright (c) Matteo Baccan -- https://github.com/matteobaccan/FinalCommander
// SPDX-License-Identifier: MIT

package io.github.matteobaccan.finalcommander;

import android.Manifest;
import android.app.Activity;
import android.content.ActivityNotFoundException;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.os.Environment;
import android.os.Handler;
import android.os.Looper;
import android.provider.Settings;
import android.util.Log;
import android.util.TypedValue;
import android.view.View;
import android.view.ViewGroup;
import android.webkit.WebResourceRequest;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import android.widget.Button;
import android.widget.FrameLayout;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;
import java.io.BufferedReader;
import java.io.File;
import java.io.IOException;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.util.ArrayDeque;
import java.util.Locale;
import java.util.Map;

/**
 * The app (packaging/android/README.md): one activity that starts
 * {@code fcmd --web-terminal 0} and shows the page it serves, on 127.0.0.1 with a token, in
 * a web view. The page is a terminal joined to a pseudo-terminal on which fcmd runs; the app
 * adds nothing to the program but a place to show it, the permission to see the phone's
 * files, and the Back button sent as Esc.
 *
 * <p>The executable travels as lib/&lt;abi&gt;/libfcmd.so, the one place Android lets an
 * app run a file it ships. Its first line on standard output is the address to open.
 */
public final class MainActivity extends Activity {
    private static final String TAG = "FinalCommander";
    private static final int START_SECONDS = 30;
    private static final int LINES_KEPT = 40;
    private static final int ASK_STORAGE = 1;

    private final Handler main = new Handler(Looper.getMainLooper());
    private final ArrayDeque<String> lines = new ArrayDeque<>();
    /** The server this activity started; one per process, whatever the activity's life. */
    private static Process server;
    private static String address;

    private WebView web;
    private ScrollView statusBox;
    private TextView status;
    private Button allow;
    private Button retry;

    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        web = new WebView(this);
        WebSettings ws = web.getSettings();
        ws.setJavaScriptEnabled(true);
        ws.setDomStorageEnabled(true);
        ws.setAllowFileAccess(false);
        ws.setAllowContentAccess(false);
        web.setWebViewClient(new WebViewClient() {
            @Override
            public boolean shouldOverrideUrlLoading(WebView view, WebResourceRequest request) {
                Uri uri = request.getUrl();
                if ("127.0.0.1".equals(uri.getHost())) {
                    return false;
                }
                // A link that leaves the phone (the help's, a release page) goes to the
                // browser, never into the terminal's view.
                try {
                    startActivity(new Intent(Intent.ACTION_VIEW, uri));
                } catch (ActivityNotFoundException e) {
                    Log.w(TAG, "nothing opens " + uri, e);
                }
                return true;
            }
        });
        web.setVisibility(View.GONE);

        int pad = dp(16);
        status = new TextView(this);
        status.setTextIsSelectable(true);
        status.setTextSize(TypedValue.COMPLEX_UNIT_SP, 14);
        allow = new Button(this);
        allow.setText(R.string.allow_files);
        allow.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                askForFiles();
            }
        });
        retry = new Button(this);
        retry.setText(R.string.retry);
        retry.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                start();
            }
        });
        LinearLayout column = new LinearLayout(this);
        column.setOrientation(LinearLayout.VERTICAL);
        column.setPadding(pad, pad, pad, pad);
        column.addView(status);
        column.addView(allow);
        column.addView(retry);
        statusBox = new ScrollView(this);
        statusBox.addView(column);

        FrameLayout root = new FrameLayout(this);
        ViewGroup.LayoutParams fill = new ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT);
        root.addView(web, fill);
        root.addView(statusBox, fill);
        setContentView(root);

        if (!canSeeFiles()) {
            // The program works without the permission, in the app's own folder; it is
            // asked for once, and the choice is the user's.
            status.setText(R.string.files_why);
            allow.setVisibility(View.VISIBLE);
            retry.setText(R.string.without_files);
            retry.setVisibility(View.VISIBLE);
        } else {
            start();
        }
    }

    @Override
    protected void onResume() {
        super.onResume();
        // Back from the system's settings with the permission given: start in the files.
        if (allow.getVisibility() == View.VISIBLE && canSeeFiles()) {
            start();
        }
    }

    /** Whether fcmd may see the phone's shared storage. */
    private boolean canSeeFiles() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            return Environment.isExternalStorageManager();
        }
        return checkSelfPermission(Manifest.permission.WRITE_EXTERNAL_STORAGE)
                == PackageManager.PERMISSION_GRANTED;
    }

    private void askForFiles() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            Intent intent = new Intent(Settings.ACTION_MANAGE_APP_ALL_FILES_ACCESS_PERMISSION,
                    Uri.parse("package:" + getPackageName()));
            try {
                startActivity(intent);
            } catch (ActivityNotFoundException e) {
                startActivity(new Intent(Settings.ACTION_MANAGE_ALL_FILES_ACCESS_PERMISSION));
            }
        } else {
            requestPermissions(new String[] {
                Manifest.permission.READ_EXTERNAL_STORAGE,
                Manifest.permission.WRITE_EXTERNAL_STORAGE
            }, ASK_STORAGE);
        }
    }

    @Override
    public void onRequestPermissionsResult(int code, String[] permissions, int[] results) {
        if (code == ASK_STORAGE) {
            start();
        }
    }

    private void start() {
        allow.setVisibility(View.GONE);
        retry.setVisibility(View.GONE);
        retry.setText(R.string.retry);
        synchronized (MainActivity.class) {
            if (server != null && server.isAlive() && address != null) {
                // The activity was made again (a rotation, coming back): same server.
                show(address);
                return;
            }
        }
        stopServer();
        synchronized (lines) {
            lines.clear();
        }
        web.setVisibility(View.GONE);
        statusBox.setVisibility(View.VISIBLE);
        status.setText(R.string.starting);
        new Thread(new Runnable() {
            @Override
            public void run() {
                launch();
            }
        }, "fcmd-start").start();
    }

    private void launch() {
        Process p;
        try {
            File home = new File(getFilesDir(), "home");
            File tmp = new File(getCacheDir(), "tmp");
            if (!home.isDirectory() && !home.mkdirs()) {
                throw new IOException("cannot create " + home);
            }
            if (!tmp.isDirectory() && !tmp.mkdirs()) {
                throw new IOException("cannot create " + tmp);
            }
            // Where the panels open: the phone's files when they may be seen, else home.
            File first = canSeeFiles() ? Environment.getExternalStorageDirectory() : home;
            String exe = getApplicationInfo().nativeLibraryDir + "/libfcmd.so";
            ProcessBuilder pb = new ProcessBuilder(exe, "--web-terminal", "0",
                    first.getAbsolutePath());
            Map<String, String> env = pb.environment();
            env.put("HOME", home.getAbsolutePath());
            env.put("TMPDIR", tmp.getAbsolutePath());
            env.put("SHELL", "/system/bin/sh");
            // fcmd's language follows LANG, in the POSIX spelling: it_IT.UTF-8.
            Locale l = Locale.getDefault();
            env.put("LANG", l.getLanguage() + (l.getCountry().isEmpty() ? "" : "_" + l.getCountry())
                    + ".UTF-8");
            pb.directory(home);
            pb.redirectErrorStream(true);
            p = pb.start();
        } catch (IOException e) {
            Log.e(TAG, "fcmd did not start", e);
            fail(getString(R.string.not_started, String.valueOf(e.getMessage())));
            return;
        }
        synchronized (MainActivity.class) {
            server = p;
            address = null;
        }
        long deadline = System.currentTimeMillis() + START_SECONDS * 1000L;
        try (BufferedReader r = new BufferedReader(
                new InputStreamReader(p.getInputStream(), StandardCharsets.UTF_8))) {
            String line;
            while ((line = r.readLine()) != null) {
                Log.i(TAG, line);
                keep(line);
                if (line.startsWith("http://127.0.0.1:")) {
                    final String url = line.trim();
                    synchronized (MainActivity.class) {
                        address = url;
                    }
                    // Logged without the token, for the emulator's test: the port alone.
                    Log.i(TAG, "fcmd --web-terminal on port " + Uri.parse(url).getPort());
                    main.post(new Runnable() {
                        @Override
                        public void run() {
                            show(url);
                        }
                    });
                    // Reading goes on, so whatever fcmd says later is kept for a failure
                    // and its output never fills up.
                    continue;
                }
                synchronized (MainActivity.class) {
                    if (address == null && System.currentTimeMillis() > deadline) {
                        break;
                    }
                }
            }
        } catch (IOException e) {
            Log.w(TAG, "fcmd's output closed", e);
        }
        synchronized (MainActivity.class) {
            if (server != p) {
                return;
            }
            if (address != null) {
                // fcmd's server ended while the app was showing it.
                address = null;
            }
        }
        fail(p.isAlive() ? getString(R.string.no_answer, START_SECONDS)
                : getString(R.string.stopped, p.exitValue()));
    }

    private void keep(String line) {
        synchronized (lines) {
            lines.addLast(line);
            while (lines.size() > LINES_KEPT) {
                lines.removeFirst();
            }
        }
    }

    private void show(String url) {
        statusBox.setVisibility(View.GONE);
        web.setVisibility(View.VISIBLE);
        web.loadUrl(url);
        web.requestFocus();
    }

    private void fail(String what) {
        final String text;
        synchronized (lines) {
            text = what + "\n\n" + String.join("\n", lines);
        }
        main.post(new Runnable() {
            @Override
            public void run() {
                web.setVisibility(View.GONE);
                statusBox.setVisibility(View.VISIBLE);
                status.setText(text);
                retry.setVisibility(View.VISIBLE);
            }
        });
    }

    /** Back is Esc: it closes what is open in fcmd, as it would on a keyboard. */
    @Override
    public void onBackPressed() {
        if (web.getVisibility() == View.VISIBLE) {
            web.evaluateJavascript("window.fcmdKey && window.fcmdKey('Esc')", null);
        } else {
            super.onBackPressed();
        }
    }

    @Override
    protected void onDestroy() {
        if (isFinishing()) {
            stopServer();
        }
        super.onDestroy();
    }

    private static void stopServer() {
        synchronized (MainActivity.class) {
            if (server != null) {
                server.destroy();
                server = null;
            }
            address = null;
        }
    }

    private int dp(int value) {
        return (int) TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_DIP, value,
                getResources().getDisplayMetrics());
    }
}
