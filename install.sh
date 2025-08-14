#!/bin/sh

# Install dependency
sudo apt install -yy qemu grub-pc grub2-common xorriso mtools curl
if command -v rustc >/dev/null 2>&1; then
  :
else
  curl https://sh.rustup.rs | sh
fi

curl https://raw.githubusercontent.com/casys-kaist/KeOS/refs/heads/main/get-projects.sh > get_projects.sh
chmod +x get_projects.sh

echo "KeOS is installed."
