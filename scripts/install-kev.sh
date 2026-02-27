#!/bin/sh
set -e
ROOT=$(pwd)
# Install dependency
sudo apt update
sudo apt install -yy qemu-system-x86 build-essential grub-pc grub2-common xorriso mtools curl git
if command -v rustc >/dev/null 2>&1; then
  :
else
  curl https://sh.rustup.rs | sh
fi

echo "[*] Downloading KeV..."
git clone https://github.com/casys-kaist/KeOS.git dist
cp -r dist template
mv dist/* dist/.gitignore dist/.git .
mv template keos-projects/.cargo
rm -rf dist

# Set git
git remote remove origin
git remote add upstream https://github.com/casys-kaist/KeOS.git

# For vscode users.
echo "[*] Setting environment for vscode"
mkdir .vscode
cat << EOF > .vscode/settings.json
{
    "rust-analyzer.linkedProjects": [
        "${ROOT}/keos-projects/Cargo.toml",
        "${ROOT}/keos/Cargo.toml",
        "${ROOT}/keos/abyss/Cargo.toml",
        "${ROOT}/fs/simple_fs/Cargo.toml",
        "${ROOT}/kev-projects/Cargo.toml",
        "${ROOT}/kev/Cargo.toml",
    ],
    "rust-analyzer.check.allTargets": false,
    "rust-analyzer.cargo.extraEnv": {
        "CARGO_BUILD_TARGET": "${ROOT}/keos-projects/.cargo/x86_64-unknown-keos.json"
    },
    "files.watcherExclude": {
        "**/target": true
    }
}
EOF

echo "[*] KeV is installed successfully."
echo "[*] Restart your shell to start."
