#!/bin/bash
set -e

../../.cargo/ffs-fsck ffs.bin

DIR=$(mktemp -d)
../../.cargo/ffs-get ffs.bin generated.tar $DIR/generated.tar
echo $DIR
tar -C $DIR -xvf $DIR/generated.tar
ls -al $DIR/tar_gen__dir
ls -al $DIR/tar_gen__dir/etc
ls -al $DIR/tar_gen__dir/etc/skel

check_integrity() {
    local original_file="$1"
    local extracted_file="$2"

    # Check if both files exist.
    if [[ ! -f "$original_file" ]]; then
        echo "Error: Original file not found - $original_file"
        exit 1
    fi
    if [[ ! -f "$extracted_file" ]]; then
        echo "Error: Extracted file not found - $extracted_file"
        exit 1
    fi

    # Calculate the SHA256 hashes of both files.
    original_hash=$(sha256sum "$original_file" | awk '{print $1}')
    extracted_hash=$(sha256sum "$extracted_file" | awk '{print $1}')

    # Compare the hashes.
    if [[ "$original_hash" != "$extracted_hash" ]]; then
        echo "Integrity check failed for $extracted_file!"
        echo "Original hash: $original_hash"
        echo "Extracted hash: $extracted_hash"
        rm -rf $DIR
        exit 1
    else
        echo "Integrity check passed for $extracted_file."
    fi
}

check_integrity rootfs/os-release $DIR/tar_gen__dir/etc/os-release
check_integrity rootfs/ls $DIR/tar_gen__dir/bin/ls
check_integrity rootfs/sha256sum $DIR/tar_gen__dir/bin/sha256sum
check_integrity rootfs/tar $DIR/tar_gen__dir/bin/tar

rm -rf $DIR