# External Tool Dependencies

Drill uses Rust crates directly for internal detectors and terminal tools through
external adapters. The terminal tools must be available in `PATH` when an
adapter runs.

## Arch Linux Packages

These package names were verified with `pacman -Qo` on the development machine:

| Command | Package |
| --- | --- |
| `binwalk` | `binwalk` |
| `uefiextract` | `uefitool-bin` |
| `uefifind` | `uefitool-bin` |
| `ghidra-analyzeHeadless` | `ghidra` |
| `iasl` | `acpica` |
| `flashrom` | `flashrom` |
| `unsquashfs` | `squashfs-tools` |
| `mksquashfs` | `squashfs-tools` |
| `7z` | `7zip` |
| `ffprobe` | `ffmpeg` |
| `ffmpeg` | `ffmpeg` |

Install command for a regular Arch system:

```sh
sudo pacman -S binwalk ghidra acpica flashrom squashfs-tools 7zip ffmpeg
```

`uefitool-bin` may come from the AUR on many Arch setups:

```sh
paru -S uefitool-bin
```

The repository also includes a helper for Arch development machines:

```sh
./scripts/install-external-tools-arch.sh
```

## Flatpak Status

The current Flatpak manifest builds Drill and Blueprint Compiler only. It does
not bundle these external terminal tools, and a sandboxed Flatpak build will not
automatically see host `/usr/bin` tools.

If Drill is distributed as Flatpak, external-tool support needs one of these
approaches:

- bundle selected tools as Flatpak modules;
- call host tools through a controlled host-spawn integration;
- disable external adapters inside Flatpak and show the missing-tool message.

## Rust Crates

Rust crates such as `serde`, `sha2`, `hex`, and `zip` are Cargo dependencies and
are not installed through these system packages.
