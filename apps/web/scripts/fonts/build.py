"""Trims Bricolage Grotesque (OFL, no Reserved Font Name) into three small static faces for the site.

The full variable font is 77 KB; the headline only needs one weight at one optical size. Run by hand
when the font or the character set changes (outputs are committed):

    python apps/web/scripts/fonts/build.py

Needs fonttools with brotli (pip install fonttools brotli).
"""
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
SOURCE = ROOT / "node_modules/@fontsource-variable/bricolage-grotesque/files/bricolage-grotesque-latin-opsz-normal.woff2"
LICENCE = ROOT / "node_modules/@fontsource-variable/bricolage-grotesque/LICENSE"
PLEX = ROOT / "node_modules/@fontsource/ibm-plex-mono"
OUT = ROOT / "apps/web/public/fonts"

# Basic Latin, Latin-1 (Italian accents), dashes, quotes, bullet, ellipsis, euro, arrows, ×.
UNICODES = "U+0020-007E,U+00A0-00FF,U+2013-2014,U+2018-201E,U+2022,U+2026,U+2039-203A,U+20AC,U+2190-2193,U+00D7"

FACES = {
    # Headlines (34–128 px): heavy, display optical size. The only preloaded font.
    "bricolage-display-800": {"wght": "800", "opsz": "72"},
    # Body copy (15–21 px).
    "bricolage-text-500": {"wght": "500", "opsz": "18"},
    # Small bold titles, buttons, the receipt summary (16–30 px).
    "bricolage-text-800": {"wght": "800", "opsz": "24"},
}


def run(*args: str) -> None:
    subprocess.run([sys.executable, "-m", *args], check=True, capture_output=True)


def rename(path: Path, family: str, style: str) -> None:
    """Gives a modified font its own name (OFL: no Reserved Font Name on modified versions)."""
    from fontTools.ttLib import TTFont

    font = TTFont(path)
    names = {1: family, 2: style, 3: f"{family.replace(' ', '')}-{style};subset", 4: f"{family} {style}", 6: f"{family.replace(' ', '')}-{style}", 16: family, 17: style}
    table = font["name"]
    for rec in list(table.names):
        if rec.nameID in names:
            table.setName(names[rec.nameID], rec.nameID, rec.platformID, rec.platEncID, rec.langID)
    font.flavor = "woff2"
    font.save(path)


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as tmp:
        for name, axes in FACES.items():
            instance = Path(tmp) / f"{name}.ttf"
            run("fontTools.varLib.instancer", str(SOURCE), *(f"{k}={v}" for k, v in axes.items()), "-o", str(instance))
            run("fontTools.subset", str(instance), f"--unicodes={UNICODES}", "--layout-features=*", "--flavor=woff2",
                f"--output-file={OUT / f'{name}.woff2'}")
            print(f"{name}.woff2: {(OUT / f'{name}.woff2').stat().st_size} bytes")
        # IBM Plex Mono (OFL): two weights cover the site; browsers use 400 for 500 and 600 for 700.
        # IBM's upstream licence reserves the name "Plex", so the trimmed (modified) copies are renamed
        # "Fettle Mono" inside the files, as the OFL requires; IBM's licence ships next to them.
        for weight in ("400", "600"):
            name = f"fettle-mono-{weight}"
            out = OUT / f"{name}.woff2"
            run("fontTools.subset", str(PLEX / f"files/ibm-plex-mono-latin-{weight}-normal.woff2"), f"--unicodes={UNICODES}",
                "--layout-features=*", "--flavor=woff2", f"--output-file={out}")
            rename(out, "Fettle Mono", "Regular" if weight == "400" else "SemiBold")
            print(f"{name}.woff2: {out.stat().st_size} bytes")
    shutil.copyfile(LICENCE, OUT / "OFL-bricolage-grotesque.txt")
    # IBM's upstream licence, word for word (Fontsource's copy drops the Reserved Font Name line).
    shutil.copyfile(Path(__file__).with_name("OFL-ibm-plex-mono.txt"), OUT / "OFL-ibm-plex-mono.txt")


if __name__ == "__main__":
    main()
