#!/bin/bash
# po/update-pot.sh
#
# Regenerates po/drill.pot by extracting translatable strings
# from ALL sources: Rust (.rs), Blueprint UI (.blp) and native UI (.ui).
# Syncs LINGUAS <-> .po files and runs msgmerge on every existing .po file.
#
# ────────────────────────────────────────────────────────────────────────────
# HOW IT WORKS
# ────────────────────────────────────────────────────────────────────────────
#
#  Step 1 — Rust source scan (.rs with gettext())
#  Step 2 — Blueprint scan (.blp) via grep lookbehind: _("..") pattern
#  Step 3 — Native UI scan (.ui with translatable="yes") via xgettext Glade
#  Step 4 — Project metadata scan (.desktop/.metainfo/.gschema)
#  Step 5 — Merge partial .pot files via Python (no msgcat dependency)
#  Step 6 — POTFILES.in regeneration
#  Step 7 — Bidirectional LINGUAS <-> .po sync + msgmerge
#
# USAGE
#   cd <repo root>
#   bash po/update-pot.sh

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PO_DIR="$ROOT/po"
PACKAGE_NAME="drill"
OUT="$PO_DIR/$PACKAGE_NAME.pot"
POTFILES="$PO_DIR/POTFILES.in"
LINGUAS_FILE="$PO_DIR/LINGUAS"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# ── Dependency check ─────────────────────────────────────────────────────────
REQUIRED_CMDS=(
    date
    find
    grep
    mktemp
    python3
    sed
    sort
    xgettext
    msginit
    msgmerge
)

for cmd in "${REQUIRED_CMDS[@]}"; do
    if ! command -v "$cmd" >/dev/null 2>&1; then
        echo "Error: required command not found: $cmd" >&2
        exit 1
    fi
done

mkdir -p "$PO_DIR"
[[ -f "$LINGUAS_FILE" ]] || touch "$LINGUAS_FILE"

echo "=== Drill — regenerating .pot ==="
echo ""

PKG_VER=$(grep -m1 '^version' "$ROOT/Cargo.toml" | sed 's/.*= *"//;s/"//')
DATE=$(date +"%Y-%m-%d %H:%M%z")

# ── Helpers ─────────────────────────────────────────────────────────────────
count_po_entries() {
    local file="$1"

    [[ -s "$file" ]] || {
        echo 0
        return
    }

    python3 - "$file" << 'PYEOF'
import ast
import re
import sys

path = sys.argv[1]

with open(path, encoding="utf-8", errors="replace") as f:
    content = f.read().strip()

if not content:
    print(0)
    raise SystemExit

blocks = re.split(r"\n{2,}", content)
count = 0

def decode_po_literal(value: str) -> str:
    try:
        return ast.literal_eval(value)
    except Exception:
        return value.strip('"')

for block in blocks:
    lines = block.splitlines()

    for i, line in enumerate(lines):
        if not line.startswith("msgid "):
            continue

        parts = []
        first = line[6:].strip()

        if first.startswith('"'):
            parts.append(first)

        j = i + 1
        while j < len(lines) and lines[j].startswith('"'):
            parts.append(lines[j].strip())
            j += 1

        msgid = "".join(decode_po_literal(part) for part in parts)

        # Do not count the PO/POT header.
        if msgid != "":
            count += 1

        break

print(count)
PYEOF
}

clean_linguas_file() {
    local file="$1"

    [[ -f "$file" ]] || return 0

    sed 's/#.*//' "$file" \
        | sed 's/^[[:space:]]*//;s/[[:space:]]*$//' \
        | sed '/^$/d' \
        | LC_ALL=C sort -u
}

write_sorted_linguas() {
    local file="$1"
    shift

    {
        echo "# Please keep this file sorted alphabetically."
        echo ""
        printf '%s\n' "$@" | sed '/^$/d' | LC_ALL=C sort -u
    } > "$file"
}

# ── 1. Rust files ───────────────────────────────────────────────────────────────
echo "[1/7] Scanning Rust files (.rs with gettext())..."
mapfile -t RUST_FILES < <(
    while IFS= read -r -d '' f; do
        if grep -q -- 'gettext(' "$f"; then
            printf '%s\n' "$f"
        fi
    done < <(find "$ROOT/src" -type f -name "*.rs" -print0) | LC_ALL=C sort
)
echo "   → ${#RUST_FILES[@]} .rs files found"

if [[ ${#RUST_FILES[@]} -gt 0 ]]; then
    RUST_REL=()
    for f in "${RUST_FILES[@]}"; do RUST_REL+=("${f#"$ROOT/"}"); done

    (
        cd "$ROOT"
        xgettext \
            --from-code=UTF-8 \
            --language=C \
            --keyword=gettext \
            --add-comments=translators \
            --package-name="$PACKAGE_NAME" \
            --package-version="$PKG_VER" \
            --output="$TMP/rust.pot" \
            "${RUST_REL[@]}" 2>/dev/null
    )
    RS_COUNT=$(count_po_entries "$TMP/rust.pot")
    echo "   → rust.pot: $RS_COUNT entries"
else
    touch "$TMP/rust.pot"
fi

# ── 2. Blueprint files (.blp) ───────────────────────────────────────────────────
echo "[2/7] Scanning Blueprint files (.blp with _(\"...\"))..."
mapfile -t BLP_FILES < <(
    while IFS= read -r -d '' f; do
        if grep -q -- '_("' "$f"; then
            printf '%s\n' "$f"
        fi
    done < <(find "$ROOT/src" -type f -name "*.blp" -print0) | LC_ALL=C sort
)
echo "   → ${#BLP_FILES[@]} .blp files found"

python3 - "$ROOT" "${BLP_FILES[@]}" > "$TMP/blp.entries" << 'PYEOF'
import ast
import re
import sys
from pathlib import Path

root = Path(sys.argv[1])
files = [Path(p) for p in sys.argv[2:]]

def po_escape(s: str) -> str:
    return (
        s.replace("\\", "\\\\")
         .replace('"', '\\"')
         .replace("\t", "\\t")
         .replace("\r", "\\r")
         .replace("\n", "\\n")
    )

# Match Blueprint _("...") and C_("context", "...") strings,
# including escaped quotes/backslashes.
gettext_pattern = re.compile(r'_\("((?:\\.|[^"\\])*)"\)')
pgettext_pattern = re.compile(
    r'C_\("((?:\\.|[^"\\])*)"\s*,\s*"((?:\\.|[^"\\])*)"\)'
)

for blp in files:
    rel = blp.relative_to(root).as_posix()
    content = blp.read_text(encoding="utf-8", errors="replace")

    for match in gettext_pattern.finditer(content):
        raw = match.group(1)

        # Decode Blueprint-style escapes, then re-escape for PO syntax.
        try:
            decoded = ast.literal_eval(f'"{raw}"')
        except Exception:
            decoded = raw

        line_no = content.count("\n", 0, match.start()) + 1

        print(f"#: {rel}:{line_no}")
        print(f'msgid "{po_escape(decoded)}"')
        print('msgstr ""')
        print()

    for match in pgettext_pattern.finditer(content):
        raw_context = match.group(1)
        raw_msgid = match.group(2)

        try:
            context = ast.literal_eval(f'"{raw_context}"')
        except Exception:
            context = raw_context

        try:
            msgid = ast.literal_eval(f'"{raw_msgid}"')
        except Exception:
            msgid = raw_msgid

        line_no = content.count("\n", 0, match.start()) + 1

        print(f"#: {rel}:{line_no}")
        print(f'msgctxt "{po_escape(context)}"')
        print(f'msgid "{po_escape(msgid)}"')
        print('msgstr ""')
        print()
PYEOF
BLP_COUNT=$(count_po_entries "$TMP/blp.entries")
echo "   → blp.entries: $BLP_COUNT entries"

# ── 3. Native UI files (.ui with translatable="yes") ────────────────────────
echo "[3/7] Scanning native UI files (.ui with translatable=\"yes\")..."
mapfile -t UI_FILES < <(
    while IFS= read -r -d '' f; do
        if grep -q -- 'translatable="yes"' "$f"; then
            printf '%s\n' "$f"
        fi
    done < <(find "$ROOT/src" -type f -name "*.ui" -print0) | LC_ALL=C sort
)
echo "   → ${#UI_FILES[@]} .ui files found"

if [[ ${#UI_FILES[@]} -gt 0 ]]; then
    UI_REL=()
    for f in "${UI_FILES[@]}"; do UI_REL+=("${f#"$ROOT/"}"); done

    (
        cd "$ROOT"
        xgettext \
            --from-code=UTF-8 \
            --language=Glade \
            --add-comments=translators \
            --package-name="$PACKAGE_NAME" \
            --package-version="$PKG_VER" \
            --output="$TMP/ui.pot" \
            "${UI_REL[@]}" 2>/dev/null || true
    )
fi
[[ -f "$TMP/ui.pot" ]] || touch "$TMP/ui.pot"
UI_COUNT=$(count_po_entries "$TMP/ui.pot")
echo "   → ui.pot: $UI_COUNT entries"

# ── 4. Project metadata files ────────────────────────────────────────────────
echo "[4/7] Scanning project metadata files..."

mapfile -t DESKTOP_FILES < <(find "$ROOT/data" -maxdepth 1 -type f -name "*.desktop.in" | LC_ALL=C sort)
if [[ ${#DESKTOP_FILES[@]} -gt 0 ]]; then
    DESKTOP_REL=()
    for f in "${DESKTOP_FILES[@]}"; do DESKTOP_REL+=("${f#"$ROOT/"}"); done

    (
        cd "$ROOT"
        xgettext \
            --from-code=UTF-8 \
            --language=Desktop \
            --add-comments=translators \
            --package-name="$PACKAGE_NAME" \
            --package-version="$PKG_VER" \
            --output="$TMP/desktop.pot" \
            "${DESKTOP_REL[@]}" 2>/dev/null || true
    )
fi
[[ -f "$TMP/desktop.pot" ]] || touch "$TMP/desktop.pot"
DESKTOP_COUNT=$(count_po_entries "$TMP/desktop.pot")
echo "   → desktop.pot: $DESKTOP_COUNT entries"

mapfile -t METAINFO_FILES < <(find "$ROOT/data" -maxdepth 1 -type f -name "*.metainfo.xml.in" | LC_ALL=C sort)
if [[ ${#METAINFO_FILES[@]} -gt 0 && -f /usr/share/gettext/its/metainfo.its ]]; then
    METAINFO_REL=()
    for f in "${METAINFO_FILES[@]}"; do METAINFO_REL+=("${f#"$ROOT/"}"); done

    (
        cd "$ROOT"
        xgettext \
            --from-code=UTF-8 \
            --its=/usr/share/gettext/its/metainfo.its \
            --add-comments=translators \
            --package-name="$PACKAGE_NAME" \
            --package-version="$PKG_VER" \
            --output="$TMP/metainfo.pot" \
            "${METAINFO_REL[@]}" 2>/dev/null || true
    )
fi
[[ -f "$TMP/metainfo.pot" ]] || touch "$TMP/metainfo.pot"
METAINFO_COUNT=$(count_po_entries "$TMP/metainfo.pot")
echo "   → metainfo.pot: $METAINFO_COUNT entries"

mapfile -t GSCHEMA_FILES < <(find "$ROOT/data" -maxdepth 1 -type f -name "*.gschema.xml" | LC_ALL=C sort)
if [[ ${#GSCHEMA_FILES[@]} -gt 0 && -f /usr/share/gettext/its/gschema.its ]]; then
    GSCHEMA_REL=()
    for f in "${GSCHEMA_FILES[@]}"; do GSCHEMA_REL+=("${f#"$ROOT/"}"); done

    (
        cd "$ROOT"
        xgettext \
            --from-code=UTF-8 \
            --its=/usr/share/gettext/its/gschema.its \
            --add-comments=translators \
            --package-name="$PACKAGE_NAME" \
            --package-version="$PKG_VER" \
            --output="$TMP/gschema.pot" \
            "${GSCHEMA_REL[@]}" 2>/dev/null || true
    )
fi
[[ -f "$TMP/gschema.pot" ]] || touch "$TMP/gschema.pot"
GSCHEMA_COUNT=$(count_po_entries "$TMP/gschema.pot")
echo "   → gschema.pot: $GSCHEMA_COUNT entries"

# ── 5. Merge via Python (without depending on msgcat) ────────────────────────
echo "[5/7] Merging partial POT files..."
python3 - "$TMP/rust.pot" "$TMP/blp.entries" "$TMP/ui.pot" \
         "$TMP/desktop.pot" "$TMP/metainfo.pot" "$TMP/gschema.pot" \
         "$OUT" "$PACKAGE_NAME" "$PKG_VER" "$DATE" << 'PYEOF'
import sys, re, os

*input_paths, out_path, package_name, pkg_ver, date = sys.argv[1:]

def extract_entries(path):
    """Return list of (refs, msgctxt, msgid) from a .pot/.entries file, skip header."""
    entries = []
    if not os.path.exists(path) or os.path.getsize(path) == 0:
        return entries
    with open(path, encoding='utf-8') as f:
        content = f.read()
    # Split on blank lines
    blocks = re.split(r'\n{2,}', content.strip())
    for block in blocks:
        lines = block.strip().splitlines()
        msgctxt_match = re.search(r'^msgctxt "(.+)"$', block, re.MULTILINE)
        msgid_match = re.search(r'^msgid "(.+)"$', block, re.MULTILINE)
        if not msgid_match:
            continue  # skip header block (msgid "")
        msgctxt = msgctxt_match.group(1) if msgctxt_match else None
        msgid = msgid_match.group(1)
        refs = [l.strip() for l in lines if l.startswith('#:')]
        entries.append((refs, msgctxt, msgid))
    return entries

def po_escape(s):
    return (
        s.replace("\\", "\\\\")
         .replace('"', '\\"')
         .replace("\t", "\\t")
         .replace("\r", "\\r")
         .replace("\n", "\\n")
    )

seen = {}
ordered = []

for path in input_paths:
    for refs, msgctxt, msgid in extract_entries(path):
        key = (msgctxt, msgid)
        if key not in seen:
            seen[key] = refs
            ordered.append(key)
        else:
            seen[key] = seen[key] + refs

with open(out_path, 'w', encoding='utf-8') as f:
    f.write('msgid ""\n')
    f.write('msgstr ""\n')
    f.write(f'"Project-Id-Version: {package_name} {pkg_ver}\\n"\n')
    f.write(f'"POT-Creation-Date: {date}\\n"\n')
    f.write('"PO-Revision-Date: YEAR-MO-DA HO:MI+ZONE\\n"\n')
    f.write('"Last-Translator: FULL NAME <EMAIL@ADDRESS>\\n"\n')
    f.write('"Language-Team: LANGUAGE <LL@li.org>\\n"\n')
    f.write('"Language: \\n"\n')
    f.write('"MIME-Version: 1.0\\n"\n')
    f.write('"Content-Type: text/plain; charset=UTF-8\\n"\n')
    f.write('"Content-Transfer-Encoding: 8bit\\n"\n')
    f.write('\n')
    for msgctxt, msgid in ordered:
        refs = seen[(msgctxt, msgid)]
        for r in refs:
            f.write(f'{r}\n')
        if msgctxt is not None:
            f.write(f'msgctxt "{po_escape(msgctxt)}"\n')
        f.write(f'msgid "{po_escape(msgid)}"\n')
        f.write('msgstr ""\n')
        f.write('\n')
print(f'   → {out_path} written ({len(ordered)} total entries)')
PYEOF

TOTAL=$(count_po_entries "$OUT")

# ── 6. Update POTFILES.in ────────────────────────────────────────────────────
echo "[6/7] Updating POTFILES.in..."
{
    echo "# Auto-generated by po/update-pot.sh — do not edit manually"
    echo ""
    for f in "${DESKTOP_FILES[@]}";  do echo "${f#"$ROOT/"}"; done
    for f in "${GSCHEMA_FILES[@]}";  do echo "${f#"$ROOT/"}"; done
    for f in "${METAINFO_FILES[@]}"; do echo "${f#"$ROOT/"}"; done
    for f in "${RUST_FILES[@]}"; do echo "${f#"$ROOT/"}"; done
    for f in "${BLP_FILES[@]}";  do echo "${f#"$ROOT/"}"; done
    for f in "${UI_FILES[@]}";   do echo "${f#"$ROOT/"}"; done
} | grep -v '^#\|^$' | sort -u > "$TMP/pf_sorted"
{
    echo "# Auto-generated by po/update-pot.sh — do not edit manually"
    echo ""
    cat "$TMP/pf_sorted"
} > "$POTFILES"
PF_COUNT=$(grep -Ec '^(data|src)/' "$POTFILES" 2>/dev/null || echo 0)
echo "   → POTFILES.in updated ($PF_COUNT files)"

# ── 7. Sync LINGUAS <-> .po + msgmerge ───────────────────────────────────────
echo "[7/7] Syncing LINGUAS <-> .po files..."

declare -A LINGUAS_SET

mapfile -t CLEAN_LANGS < <(clean_linguas_file "$LINGUAS_FILE")

for lang in "${CLEAN_LANGS[@]}"; do
    LINGUAS_SET["$lang"]=1
done

ADDED_TO_LINGUAS=()
CREATED_PO=()

for po_file in "$PO_DIR"/*.po; do
    [[ -f "$po_file" ]] || continue
    lang=$(basename "$po_file" .po)
    if [[ -z "${LINGUAS_SET[$lang]:-}" ]]; then
        echo "   + LINGUAS: adding '$lang'"
        LINGUAS_SET["$lang"]=1
        ADDED_TO_LINGUAS+=("$lang")
    fi
done

for lang in "${!LINGUAS_SET[@]}"; do
    po_file="$PO_DIR/$lang.po"
    if [[ ! -f "$po_file" ]]; then
        echo "   + Creating $lang.po via msginit..."
        msginit \
            --input="$OUT" \
            --locale="$lang" \
            --output="$po_file" \
            --no-translator 2>/dev/null || true
        [[ -f "$po_file" ]] && CREATED_PO+=("$lang") || \
            echo "   ⚠ msginit failed for '$lang'"
    fi
done

mapfile -t FINAL_LANGS < <(
    printf '%s\n' "${!LINGUAS_SET[@]}" | sed '/^$/d' | LC_ALL=C sort -u
)

write_sorted_linguas "$LINGUAS_FILE" "${FINAL_LANGS[@]}"

[[ ${#ADDED_TO_LINGUAS[@]} -gt 0 ]] && echo "   → Added to LINGUAS: ${ADDED_TO_LINGUAS[*]}"
[[ ${#CREATED_PO[@]}       -gt 0 ]] && echo "   → .po files created: ${CREATED_PO[*]}"
[[ ${#ADDED_TO_LINGUAS[@]} -eq 0 && ${#CREATED_PO[@]} -eq 0 ]] && \
    echo "   → LINGUAS and .po files already in sync"

echo ""
echo "✓ Generated : $OUT"
echo "✓ Entries   : $TOTAL"
echo "✓ Version   : $PACKAGE_NAME $PKG_VER"
LANGUAGE_LIST="$(clean_linguas_file "$LINGUAS_FILE" | tr '\n' ' ' | xargs)"
echo "✓ Languages : $LANGUAGE_LIST"

# ── Update ALL .po files ───────────────────────────────────────────────────────────
echo ""
echo "=== Updating all .po files with msgmerge ==="
for po in "$PO_DIR"/*.po; do
    [[ -f "$po" ]] || continue
    lang=$(basename "$po" .po)
    printf "  → %-14s" "$lang.po"
    msgmerge --quiet --update --backup=none "$po" "$OUT"
    UNTRANSLATED=$(grep -c '^msgstr ""' "$po" 2>/dev/null || echo 0)
    TOTAL_ENTRIES=$(count_po_entries "$po")

    # Remove PO header from untranslated count.
    if [[ "$UNTRANSLATED" -gt 0 ]]; then
        UNTRANSLATED=$((UNTRANSLATED - 1))
    fi

    echo " ($UNTRANSLATED/$TOTAL_ENTRIES untranslated)"
done
echo "✓ All .po files updated!"
