#!/bin/sh
# The `kind` column: what `file` says this is, one file per run.
#
# The path is the first argument (and FCMD_FILE); the first line printed is the cell.
# Printing nothing, or ending with a failing status, is an empty cell.
command -v file >/dev/null 2>&1 || exit 1
file -b -- "$1" 2>/dev/null | cut -d, -f1
