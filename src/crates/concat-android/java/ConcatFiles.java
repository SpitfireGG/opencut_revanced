// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Jareer and Concat contributors

package app.concat.editor;

import android.app.Activity;
import android.app.Fragment;
import android.content.ClipData;
import android.content.ContentResolver;
import android.content.Context;
import android.content.Intent;
import android.database.Cursor;
import android.net.Uri;
import android.os.Build;
import android.os.ParcelFileDescriptor;
import android.os.VibrationEffect;
import android.os.Vibrator;
import android.provider.MediaStore;
import android.provider.OpenableColumns;
import android.util.Log;

import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.util.ArrayList;
import java.nio.channels.FileChannel;
import java.util.List;
import java.util.concurrent.CountDownLatch;

/**
 * The system's document picker, for an activity that has no Java of its
 * own to receive its answer.
 *
 * The window is a NativeActivity: the framework's own class, with nothing
 * of ours to override, and a picker's result comes back only through an
 * activity's or a fragment's onActivityResult. So this is a fragment - a
 * headless one, added to the activity for the length of one pick - and
 * the result comes back here.
 *
 * What is picked is a content URI, which the engine's decoder cannot open:
 * it reads files by path. Each is copied into the app's own external
 * files, under Imported/, keeping its display name, and the paths are
 * what the engine is handed - through the native method below, which the
 * Rust side registers before it asks for a pick.
 *
 * Compiled by build.rs with the SDK's javac and d8, and loaded at run time
 * from the dex inside the binary; see src/lib.rs.
 */
public class ConcatFiles extends Fragment {
    private static final int REQUEST_PICK = 0xC0C4;
    private static final String TAG = "concat-files";

    /** Registered from Rust: the picked files' paths, or none. */
    public static native void filesPicked(String[] paths);

    /** A short tick from the vibration motor. From any thread. */
    public static void tick(Activity activity) {
        Vibrator vibrator = (Vibrator) activity.getSystemService(Context.VIBRATOR_SERVICE);
        if (vibrator == null || !vibrator.hasVibrator()) {
            return;
        }
        try {
            if (Build.VERSION.SDK_INT >= 29) {
                vibrator.vibrate(VibrationEffect.createPredefined(VibrationEffect.EFFECT_TICK));
            } else {
                vibrator.vibrate(VibrationEffect.createOneShot(15, VibrationEffect.DEFAULT_AMPLITUDE));
            }
        } catch (Exception e) {
            Log.w(TAG, "Could not vibrate: " + e);
        }
    }

    /**
     * Shows the picker. From any thread; the fragment is added on the UI
     * thread. With `gallery`, the system's photo and video picker where
     * there is one (Android 13 on); otherwise the document picker, which
     * also offers sound.
     */
    public static void pick(final Activity activity, final boolean gallery) {
        activity.runOnUiThread(new Runnable() {
            @Override
            public void run() {
                ConcatFiles fragment = new ConcatFiles();
                activity.getFragmentManager()
                        .beginTransaction()
                        .add(fragment, TAG)
                        .commitAllowingStateLoss();
                activity.getFragmentManager().executePendingTransactions();

                Intent intent;
                if (gallery && Build.VERSION.SDK_INT >= 33) {
                    intent = new Intent(MediaStore.ACTION_PICK_IMAGES);
                    intent.putExtra(MediaStore.EXTRA_PICK_IMAGES_MAX,
                            MediaStore.getPickImagesMaxLimit());
                } else {
                    intent = new Intent(Intent.ACTION_OPEN_DOCUMENT);
                    intent.addCategory(Intent.CATEGORY_OPENABLE);
                    intent.setType("*/*");
                    intent.putExtra(Intent.EXTRA_MIME_TYPES,
                            gallery ? new String[] {"video/*", "image/*"}
                                    : new String[] {"video/*", "audio/*", "image/*"});
                    intent.putExtra(Intent.EXTRA_ALLOW_MULTIPLE, true);
                }
                try {
                    fragment.startActivityForResult(intent, REQUEST_PICK);
                } catch (Exception e) {
                    fragment.done(new ArrayList<Uri>());
                }
            }
        });
    }

    @Override
    public void onActivityResult(int request, int result, Intent data) {
        if (request != REQUEST_PICK) {
            return;
        }
        List<Uri> uris = new ArrayList<Uri>();
        if (result == Activity.RESULT_OK && data != null) {
            ClipData clip = data.getClipData();
            if (clip != null) {
                for (int i = 0; i < clip.getItemCount(); i++) {
                    Uri uri = clip.getItemAt(i).getUri();
                    if (uri != null) {
                        uris.add(uri);
                    }
                }
            } else if (data.getData() != null) {
                uris.add(data.getData());
            }
        }
        done(uris);
    }

    /** Copies what was picked and reports it; then the fragment goes. */
    private void done(final List<Uri> uris) {
        final Activity activity = getActivity();
        try {
            getFragmentManager().beginTransaction().remove(this).commitAllowingStateLoss();
        } catch (Exception ignored) {
        }
        if (activity == null || uris.isEmpty()) {
            filesPicked(new String[0]);
            return;
        }
        
        // Take persistable URI permission so we can access the files even if the activity is destroyed
        final ContentResolver resolver = activity.getContentResolver();
        for (Uri uri : uris) {
            try {
                resolver.takePersistableUriPermission(uri, 
                        Intent.FLAG_GRANT_READ_URI_PERMISSION);
            } catch (Exception e) {
                Log.w(TAG, "Failed to take persistable URI permission: " + e);
            }
        }
        
        // Off the UI thread: a video is big and the copy takes a while.
        final Context appContext = activity.getApplicationContext();
        new Thread(new Runnable() {
            @Override
            public void run() {
                final File dir = new File(appContext.getExternalFilesDir(null), "Imported");
                dir.mkdirs();
                // Every file at once, each on its own thread; the answer
                // keeps the order they were picked in.
                final String[] found = new String[uris.size()];
                final CountDownLatch left = new CountDownLatch(uris.size());
                for (int i = 0; i < uris.size(); i++) {
                    final int index = i;
                    final Uri uri = uris.get(i);
                    new Thread(new Runnable() {
                        @Override
                        public void run() {
                            try {
                                File out = place(appContext, dir, uri);
                                if (out != null) {
                                    found[index] = out.getAbsolutePath();
                                }
                            } finally {
                                left.countDown();
                            }
                        }
                    }, TAG).start();
                }
                try {
                    left.await();
                } catch (InterruptedException ignored) {
                }
                List<String> paths = new ArrayList<String>();
                for (String path : found) {
                    if (path != null) {
                        paths.add(path);
                    }
                }
                filesPicked(paths.toArray(new String[0]));
            }
        }, TAG).start();
    }

    /**
     * Where a picked file lives in Imported/: the copy already there when
     * one of the same name and size is, else a fresh copy. Null when the
     * copy fails.
     */
    private static File place(Context context, File dir, Uri uri) {
        String name = displayName(context, uri);
        long size = size(context, uri);
        File same = new File(dir, name);
        if (size > 0 && same.isFile() && same.length() == size) {
            return same;
        }
        File out = unique(dir, name);
        return copy(context, uri, out) ? out : null;
    }

    /** The size the provider reports, or -1. */
    private static long size(Context context, Uri uri) {
        Cursor cursor = null;
        try {
            cursor = context.getContentResolver().query(
                    uri, new String[] {OpenableColumns.SIZE}, null, null, null);
            if (cursor != null && cursor.moveToFirst() && !cursor.isNull(0)) {
                return cursor.getLong(0);
            }
        } catch (Exception ignored) {
        } finally {
            if (cursor != null) {
                cursor.close();
            }
        }
        return -1;
    }

    /** The name the picker showed for the file, or one made from the URI. */
    private static String displayName(Context context, Uri uri) {
        String name = null;
        Cursor cursor = null;
        try {
            cursor = context.getContentResolver().query(uri, null, null, null, null);
            if (cursor != null && cursor.moveToFirst()) {
                int column = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME);
                if (column >= 0) {
                    name = cursor.getString(column);
                }
            }
        } catch (Exception ignored) {
        } finally {
            if (cursor != null) {
                cursor.close();
            }
        }
        if (name == null || name.isEmpty()) {
            name = uri.getLastPathSegment();
        }
        if (name == null || name.isEmpty()) {
            name = "import";
        }
        // Whatever the provider called it, it is one file name.
        return name.replace('/', '_').replace('\\', '_');
    }

    /** `name`, or `name (2)`, `name (3)`... until nothing is overwritten. */
    private static File unique(File dir, String name) {
        File out = new File(dir, name);
        if (!out.exists()) {
            return out;
        }
        int dot = name.lastIndexOf('.');
        String stem = dot > 0 ? name.substring(0, dot) : name;
        String ext = dot > 0 ? name.substring(dot) : "";
        for (int n = 2; ; n++) {
            out = new File(dir, stem + " (" + n + ")" + ext);
            if (!out.exists()) {
                return out;
            }
        }
    }

    private static boolean copy(Context context, Uri uri, File out) {
        // A file behind the URI copies in the kernel, without passing
        // through a buffer here; a pipe falls back to the stream below.
        ParcelFileDescriptor descriptor = null;
        try {
            descriptor = context.getContentResolver().openFileDescriptor(uri, "r");
            if (descriptor != null && descriptor.getStatSize() > 0) {
                FileChannel from = new FileInputStream(descriptor.getFileDescriptor()).getChannel();
                FileChannel to = new FileOutputStream(out).getChannel();
                try {
                    long total = descriptor.getStatSize();
                    long done = 0;
                    while (done < total) {
                        long moved = to.transferFrom(from, done, total - done);
                        if (moved <= 0) {
                            break;
                        }
                        done += moved;
                    }
                    if (done == total) {
                        return true;
                    }
                } finally {
                    to.close();
                    from.close();
                }
            }
        } catch (Exception e) {
            Log.w(TAG, "Channel copy failed, streaming instead: " + e);
        } finally {
            try {
                if (descriptor != null) descriptor.close();
            } catch (IOException ignored) {
            }
        }
        return stream(context, uri, out);
    }

    private static boolean stream(Context context, Uri uri, File out) {
        InputStream in = null;
        OutputStream os = null;
        try {
            in = context.getContentResolver().openInputStream(uri);
            if (in == null) {
                return false;
            }
            os = new FileOutputStream(out);
            byte[] buffer = new byte[1 << 20];
            int read;
            while ((read = in.read(buffer)) > 0) {
                os.write(buffer, 0, read);
            }
            return true;
        } catch (IOException e) {
            out.delete();
            return false;
        } finally {
            try {
                if (in != null) in.close();
            } catch (IOException ignored) {
            }
            try {
                if (os != null) os.close();
            } catch (IOException ignored) {
            }
        }
    }
}