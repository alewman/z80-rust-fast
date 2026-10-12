#!/usr/bin/env bash
# Check out the z80-python reference at the pinned commit and install it into
# a virtual environment, so rung 1/3/5 diffs run against exactly that core.
set -euo pipefail
COMMIT=b62891b
DEST=${1:-"$(dirname "$0")/../external/z80-python"}
PYTHON=${PYTHON:-python3}
if [ ! -d "$DEST/.git" ]; then
    git clone -q https://github.com/alewman/z80-python.git "$DEST"
fi
git -C "$DEST" fetch -q origin
git -C "$DEST" checkout -q --detach "$COMMIT"
if [ ! -x "$DEST/.venv/bin/python" ]; then
    "$PYTHON" -m venv "$DEST/.venv"
fi
"$DEST/.venv/bin/python" -m pip install -q -e "$DEST"
echo "z80-python at $(git -C "$DEST" rev-parse --short HEAD) installed in $DEST/.venv"
