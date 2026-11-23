#!/usr/bin/env python3
"""Render the MobaScape progress screen in a Matrix (1999/2000) digital-rain style.

Writes <out>.txt (plain), <out>.ans (24-bit ANSI colour) and <out>.png.
"""
import random
import sys

from PIL import Image, ImageDraw, ImageFont

W, H = 150, 50
out = sys.argv[1]

HEAD = (225, 255, 225)
BRIGHT = (0, 255, 70)
MID = (0, 190, 45)
DIM = (0, 110, 25)
FAINT = (0, 55, 12)
WHITE = (235, 255, 235)
AMBER = (200, 255, 120)
BLACK = (0, 0, 0)

grid = [[(" ", FAINT, BLACK) for _ in range(W)] for _ in range(H)]


def put(x, y, ch, fg, bg=BLACK):
    if 0 <= x < W and 0 <= y < H:
        grid[y][x] = (ch, fg, bg)


def text(x, y, s, fg=BRIGHT, bg=BLACK):
    for i, ch in enumerate(s):
        put(x + i, y, ch, fg, bg)


def center(y, s, fg=BRIGHT, bg=BLACK):
    text((W - len(s)) // 2, y, s, fg, bg)


# --- digital rain --------------------------------------------------------
rng = random.Random(2000)
GLYPHS = "0123456789ABCDEFZ:.=*+-<>|\"'^~$#@%&?!" + "ZQXJ7kTL"
for x in range(W):
    if rng.random() < 0.18:
        continue
    for _ in range(rng.choice([1, 1, 2])):
        head = rng.randrange(-4, H + 6)
        length = rng.randrange(6, 26)
        for k in range(length):
            y = head - k
            if not 0 <= y < H:
                continue
            ch = rng.choice(GLYPHS)
            if k == 0:
                col = HEAD
            elif k < 3:
                col = BRIGHT
            elif k < length * 0.5:
                col = MID
            elif k < length * 0.8:
                col = DIM
            else:
                col = FAINT
            put(x, y, ch, col)


# --- panel ---------------------------------------------------------------
def panel(x0, y0, x1, y1, title=None):
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            put(x, y, " ", BRIGHT)
    for x in range(x0, x1 + 1):
        put(x, y0, "=", MID)
        put(x, y1, "=", MID)
    for y in range(y0, y1 + 1):
        put(x0, y, "|", MID)
        put(x1, y, "|", MID)
    for cx, cy in ((x0, y0), (x1, y0), (x0, y1), (x1, y1)):
        put(cx, cy, "+", BRIGHT)
    if title:
        text(x0 + 3, y0, f"[ {title} ]", HEAD)


# Big title in a 5-row block font.
FONT = {
    "M": ["#   #", "## ##", "# # #", "#   #", "#   #"],
    "O": [" ### ", "#   #", "#   #", "#   #", " ### "],
    "B": ["#### ", "#   #", "#### ", "#   #", "#### "],
    "A": [" ### ", "#   #", "#####", "#   #", "#   #"],
    "S": [" ####", "#    ", " ### ", "    #", "#### "],
    "C": [" ####", "#    ", "#    ", "#    ", " ####"],
    "P": ["#### ", "#   #", "#### ", "#    ", "#    "],
    "E": ["#####", "#    ", "#### ", "#    ", "#####"],
    " ": ["   ", "   ", "   ", "   ", "   "],
}


def big(y, word):
    rows = ["", "", "", "", ""]
    for ch in word:
        for r in range(5):
            rows[r] += FONT[ch][r] + " "
    width = len(rows[0])
    x0 = (W - width) // 2
    for r, row in enumerate(rows):
        for i, ch in enumerate(row):
            if ch == "#":
                put(x0 + i, y + r, "#", HEAD if r == 0 else BRIGHT)
            else:
                put(x0 + i, y + r, " ", BRIGHT)


panel(16, 1, W - 17, 11)
big(3, "MOBASCAPE")
center(9, "K I T   A U D I O   //   E V E R Y   S P E L L   H A S   A   S O U N D", WHITE)

panel(6, 13, W - 7, 17, "TRANSMISSION")
text(9, 14, "> I know kung fu.", BRIGHT)
text(9, 15, "> Show me.  Twelve kit champions cast with one shared whoosh. Now each action speaks for itself.", MID)
text(9, 16, "> Buffs chant the OSRS prayer chimes. Slams land with an anvil. Pools roar with fire.", BRIGHT)

# Left: action -> sound.
panel(6, 19, 74, 33, "KIT ACTION  ->  SOUND")
rows = [
    ("projectile", "1930", "whoosh, pitched up"),
    ("dash", "1930", "whoosh, pitched down"),
    ("area (landing)", "3791", "SMITH_ANVIL_TONK, deep"),
    ("zone", "2596", "FIRE_WOOSH"),
    ("summon", " 200", "TELEPORT_VWOOP"),
    ("transform", " 200", "TELEPORT_VWOOP, lower"),
    ("channel", "2670", "prayer: Mystic Will"),
    ("empower", "2691", "prayer: Ultimate Strength"),
]
for i, (k, n, v) in enumerate(rows):
    text(9, 21 + i, "[OK]", HEAD)
    text(14, 21 + i, f"{k} ".ljust(18, "."), MID)
    text(33, 21 + i, n, HEAD)
    text(39, 21 + i, v, BRIGHT)
text(9, 30, "names: kit_projectile ... kit_transform", DIM)
text(9, 31, "champion `sounds` overrides by name; q_* keys", AMBER)
text(9, 32, "silence a slot and play q_cast instead", AMBER)

# Right: buffs.
panel(76, 19, W - 7, 33, "BUFF  ->  STRONGEST EFFECT  ->  PRAYER CHIME")
ids = [
    ("heal / heal_missing", "2678", "Rapid Heal"),
    ("shield / dmg reduction", "2687", "Steel Skin"),
    ("reflect / untargetable", "2687", "Steel Skin"),
    ("attack / move speed", "2667", "Incredible Reflexes"),
    ("anything else", "2690", "Thick Skin"),
]
for i, (use, n, src) in enumerate(ids):
    text(79, 21 + i, use.ljust(26), BRIGHT)
    text(106, 21 + i, n, HEAD)
    text(112, 21 + i, src, MID)
text(79, 27, "LIVE CUES   --demo-kit --timescale 4   (W / E / R)", WHITE)
text(79, 28, "cerberus   2596  1930  3791", BRIGHT)
text(79, 29, "graardor    200  2687+3791  1930", BRIGHT)
text(79, 30, "venenatis  1930(+3791 pounce)  200  2596", BRIGHT)
text(79, 31, "awowogei   own kit sounds kept, no shared noise", BRIGHT)
text(79, 32, "rift-tools sounds --ids: 17 / 17 ids decode", AMBER)

# Verification strip.
panel(6, 35, W - 7, 42, "OPERATOR CHECKS")
checks = [
    ("TESTS", "cargo test --workspace", "361 PASS / 0 FAIL", "+2 new: kit actions covered, buff chime choice"),
    ("CLIPPY", "cargo clippy -D warnings", "CLEAN", "cargo fmt --check clean"),
    ("LIVE", "--demo-kit --timescale 4", "CUES TRACED", "3 kit heroes x W/E/R + hand-written king"),
    ("COMMITS", "9105a06  audio: kit abilities sound by action type", "", ""),
    ("", "d2a3733  docs: record kit action sounds", "", ""),
]
for i, (k, cmd, res, note) in enumerate(checks):
    y = 36 + i
    text(9, y, k, WHITE)
    text(19, y, cmd, MID)
    if res:
        text(50, y, res, HEAD)
    if note:
        text(70, y, note, DIM)

# Next steps.
panel(6, 44, W - 7, 48, "NEXT  //  THE ORACLE SAYS")
text(9, 45, "1. Signature sounds per champion (your ear, your call).   2. Announcer + music hook (M9).", BRIGHT)
text(9, 46, "3. By-ear volume pass on data/sounds.json.   4. Item tree: components, upgrades, actives.", BRIGHT)
text(9, 47, "Not heard by a person yet: the right ids fire at the right moments; the mix is untested.", AMBER)

center(49, " DO NOT TRY TO BEND THE SPOON.  ONLY TRY TO REALISE THE PRAYER CHIME. ", HEAD)

# --- outputs -------------------------------------------------------------
with open(out + ".txt", "w") as f:
    for row in grid:
        f.write("".join(c for c, _, _ in row).rstrip() + "\n")

with open(out + ".ans", "w") as f:
    for row in grid:
        last = None
        line = []
        for ch, fg, bg in row:
            key = (fg, bg)
            if key != last:
                line.append(f"\x1b[38;2;{fg[0]};{fg[1]};{fg[2]}m\x1b[48;2;{bg[0]};{bg[1]};{bg[2]}m")
                last = key
            line.append(ch)
        f.write("".join(line) + "\x1b[0m\n")

font = ImageFont.truetype("/usr/share/fonts/noto/NotoSansMono-Regular.ttf", 15)
bold = ImageFont.truetype("/usr/share/fonts/noto/NotoSansMono-Bold.ttf", 15)
cw = int(round(font.getlength("M")))
ch_h = 18
img = Image.new("RGB", (W * cw + 16, H * ch_h + 16), BLACK)
d = ImageDraw.Draw(img)
for y, row in enumerate(grid):
    for x, (c, fg, bg) in enumerate(row):
        if c != " ":
            f = bold if fg in (HEAD, WHITE, BRIGHT) else font
            d.text((8 + x * cw, 8 + y * ch_h), c, fill=fg, font=f)
img.save(out + ".png")
print(f"wrote {out}.txt/.ans/.png ({img.size[0]}x{img.size[1]})")
