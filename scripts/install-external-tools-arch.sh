#!/usr/bin/env bash
set -euo pipefail

official_packages=(
  binwalk
  ghidra
  acpica
  flashrom
  squashfs-tools
  7zip
  ffmpeg
)

aur_packages=(
  uefitool-bin
)

missing_commands=()
for command in \
  binwalk \
  uefiextract \
  uefifind \
  ghidra-analyzeHeadless \
  iasl \
  flashrom \
  unsquashfs \
  mksquashfs \
  7z \
  ffprobe \
  ffmpeg
do
  if ! command -v "$command" >/dev/null 2>&1; then
    missing_commands+=("$command")
  fi
done

if [[ ${#missing_commands[@]} -eq 0 ]]; then
  echo "All Drill external tools are already installed."
  exit 0
fi

echo "Missing commands: ${missing_commands[*]}"
echo "Installing official Arch packages: ${official_packages[*]}"
sudo pacman -S --needed "${official_packages[@]}"

for package in "${aur_packages[@]}"; do
  if pacman -Q "$package" >/dev/null 2>&1; then
    continue
  fi

  if command -v paru >/dev/null 2>&1; then
    paru -S --needed "$package"
  elif command -v yay >/dev/null 2>&1; then
    yay -S --needed "$package"
  else
    echo "AUR helper not found. Install $package manually to provide uefiextract and uefifind." >&2
  fi
done
