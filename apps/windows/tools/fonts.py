# Converts the website's trimmed WOFF2 fonts to TTF for Slint, with distinct family names
# (the three Bricolage files all call themselves "Bricolage Grotesque 96pt ExtraBold").
# The website's font files never change under the same name (browsers keep /fonts/* for a year and
# apps/web/test/fonts.test.ts pins each file's hash to its name): a rebuilt font gets a new name
# there, and the names below must follow it. The app's TTFs in ui/fonts keep their own names.
from fontTools.ttLib import TTFont
SRC = "../../../apps/web/public/fonts/"
OUT = "../ui/fonts/"
FONTS = {
    "bricolage-display-800": ("Fettle Display", 800),
    "bricolage-text-500": ("Fettle Text", 500),
    "bricolage-text-800": ("Fettle Text", 800),
    "fettle-mono-400": ("Fettle Mono", 400),
    "fettle-mono-600": ("Fettle Mono", 600),
}
STYLE = {400: "Regular", 500: "Medium", 600: "SemiBold", 800: "ExtraBold"}
for name, (family, weight) in FONTS.items():
    f = TTFont(SRC + name + ".woff2")
    f.flavor = None
    names = f["name"]
    for rec in list(names.names):
        if rec.nameID in (16, 17, 21, 22):
            names.removeNames(nameID=rec.nameID)
    style = STYLE[weight]
    for pid, eid, lid in [(3, 1, 0x409), (1, 0, 0)]:
        names.setName(family, 1, pid, eid, lid)
        names.setName(style, 2, pid, eid, lid)
        names.setName(f"{family} {style}", 4, pid, eid, lid)
        names.setName(f"{family.replace(' ', '')}-{style}", 6, pid, eid, lid)
    f["OS/2"].usWeightClass = weight
    f.save(OUT + name + ".ttf")
    print(name, "->", family, weight)
