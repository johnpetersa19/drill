# Drill

Recursive binary file analysis tool using a node tree.

Accepts any `.bin`, `.rom`, `.raw`, `.img` or `.dat` — firmware, executable,
memory dump, disk image, console ROM — and traverses the real content
using pluggable detectors, entropy analysis and structural validation,
producing a navigable JSON map (`skeleton/manifest.json`) without ever
modifying the original file.

## Architecture

```
src/
├── engine/       # recursive engine: Node, Tree, Entropy, Manifest
├── detectors/    # detection plugins (containers, compression, fs, executables...)
├── packers/      # symmetric packers for Reverse Drill
├── output/       # skeleton/, nodes/, reports generation
└── window.rs     # GTK4/libadwaita UI
```

## Output

```
DRILL_ANALYSIS/
├── original/          # untouched file + SHA256
├── skeleton/          # manifest.json + text reports
├── nodes/             # assembly, pseudo-code, filesystem listings
├── edits/             # pending / applied edits (Reverse Drill)
└── output/            # reconstructed file + round-trip validation
```

## License

GPL-3.0-or-later
