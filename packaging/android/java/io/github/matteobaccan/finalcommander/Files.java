// FinalCommander -- an orthodox file manager for the terminal.
// Copyright (c) Matteo Baccan -- https://github.com/matteobaccan/FinalCommander
// SPDX-License-Identifier: MIT

package io.github.matteobaccan.finalcommander;

import android.content.ContentProvider;
import android.content.ContentValues;
import android.content.Context;
import android.database.Cursor;
import android.database.MatrixCursor;
import android.net.Uri;
import android.os.ParcelFileDescriptor;
import android.provider.OpenableColumns;
import android.webkit.MimeTypeMap;
import java.io.File;
import java.io.FileNotFoundException;
import java.util.Locale;

/**
 * The files fcmd opens with another app, as content: addresses: since Android 7 an app may
 * not hand another a path, only an address the receiving app is given the right to read, one
 * file at a time ({@code FLAG_GRANT_READ_URI_PERMISSION}). Read only; not exported, so no app
 * reads through it without that grant. The address is the file's path:
 * {@code content://io.github.matteobaccan.finalcommander.files/storage/emulated/0/a.pdf}.
 */
public final class Files extends ContentProvider {
    static Uri uriFor(Context context, File file) {
        return new Uri.Builder()
                .scheme("content")
                .authority(context.getPackageName() + ".files")
                .path(file.getAbsolutePath())
                .build();
    }

    /** The MIME type by the name's extension; a type no app claims for a name without one. */
    static String typeOf(String name) {
        int dot = name.lastIndexOf('.');
        if (dot >= 0) {
            String ext = name.substring(dot + 1).toLowerCase(Locale.ROOT);
            String type = MimeTypeMap.getSingleton().getMimeTypeFromExtension(ext);
            if (type != null) {
                return type;
            }
        }
        return "application/octet-stream";
    }

    private static File fileOf(Uri uri) throws FileNotFoundException {
        String path = uri.getPath();
        if (path == null) {
            throw new FileNotFoundException(String.valueOf(uri));
        }
        File file = new File(path);
        if (!file.isFile()) {
            throw new FileNotFoundException(path);
        }
        return file;
    }

    @Override
    public boolean onCreate() {
        return true;
    }

    @Override
    public ParcelFileDescriptor openFile(Uri uri, String mode) throws FileNotFoundException {
        if (mode.contains("w")) {
            throw new FileNotFoundException("read only: " + uri);
        }
        return ParcelFileDescriptor.open(fileOf(uri), ParcelFileDescriptor.MODE_READ_ONLY);
    }

    @Override
    public String getType(Uri uri) {
        return typeOf(String.valueOf(uri.getLastPathSegment()));
    }

    /** The name and the size, which the receiving app shows. */
    @Override
    public Cursor query(Uri uri, String[] projection, String selection, String[] args,
            String order) {
        File file;
        try {
            file = fileOf(uri);
        } catch (FileNotFoundException e) {
            return null;
        }
        String[] columns = projection != null ? projection
                : new String[] {OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE};
        Object[] row = new Object[columns.length];
        for (int i = 0; i < columns.length; i++) {
            if (OpenableColumns.DISPLAY_NAME.equals(columns[i])) {
                row[i] = file.getName();
            } else if (OpenableColumns.SIZE.equals(columns[i])) {
                row[i] = file.length();
            }
        }
        MatrixCursor cursor = new MatrixCursor(columns, 1);
        cursor.addRow(row);
        return cursor;
    }

    @Override
    public Uri insert(Uri uri, ContentValues values) {
        throw new UnsupportedOperationException("read only");
    }

    @Override
    public int delete(Uri uri, String selection, String[] args) {
        throw new UnsupportedOperationException("read only");
    }

    @Override
    public int update(Uri uri, ContentValues values, String selection, String[] args) {
        throw new UnsupportedOperationException("read only");
    }
}
