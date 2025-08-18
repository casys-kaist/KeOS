#!/usr/bin/env bash
set -euo pipefail

TMPFILE="$0_"
curl -fL -o $TMPFILE https://raw.githubusercontent.com/casys-kaist/KeOS/refs/heads/main/get-projects.sh
if diff -q "$TMPFILE" "$0" >/dev/null; then
  rm $TMPFILE
else
  echo "[*] Updating install script..."
  mv $TMPFILE $0
  exec $0
fi

packages=(
  "https://github.com/casys-kaist/KeOS/releases/download/v1.0/base.tar.gz 2bd782143bcd426503e64571c331ded0474fe860"
)

mkdir -p package

for pkg in "${packages[@]}"; do
  URL=$(echo "$pkg" | awk '{print $1}')
  SHA1SUM=$(echo "$pkg" | awk '{print $2}')

  FILENAME=$(basename "$URL")
  FILEPATH="package/$FILENAME"

  if [ -f "$FILEPATH" ]; then
    CURRENT_HASH=$(sha1sum "$FILEPATH" | awk '{print $1}')
    if [ "$CURRENT_HASH" = "$SHA1SUM" ]; then
      echo "[*] $FILENAME is up-to-date, skipping download"
      continue
    else
      echo "[*] Updating $FILENAME..."
    fi
  else
    echo "[*] Downloading $FILENAME..."
  fi

  if ! curl -fL -o "$FILEPATH" "$URL" >/dev/null 2>&1; then
    echo "[*] Failed to download $FILENAME" >&2
    continue
  fi
  
  DOWN_HASH=$(sha1sum "$FILEPATH" | awk '{print $1}')
  if [ "$DOWN_HASH" != "$SHA1SUM" ]; then
    echo "[*] $FILENAME : Hash mismatch after download! "
    echo "[*] Please notify to the instructor."
  else
    tar -xzf $FILEPATH -C .
  fi
done

