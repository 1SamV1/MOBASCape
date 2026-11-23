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
center(9, "F R O N T   E N D   //   M E N U ,   S P O T L I G H T ,   S T E E L   I N   H A N D", WHITE)

panel(6, 13, W - 7, 17, "TRANSMISSION")
text(9, 14, "> This is the construct. Our loading program.", BRIGHT)
text(9, 15, "> The AppImage now wakes in a braziers-and-gold main menu. Pick a mode, beat the clock, pick a hero.", MID)
text(9, 16, "> Guns. Lots of guns.  Humanoid heroes now hold the weapon in their inventory.", BRIGHT)

# Left: the flow.
panel(6, 19, 74, 33, "BOOT FLOW")
rows = [
    ("main menu", "NEW", "flames, stone tablet, orbit cam"),
    ("BOT MATCH", "LIVE", "5v5 vs bots"),
    ("PUBLIC MATCH", "STUB", "offline: bots fill"),
    ("PRIVATE MATCH", "STUB", "offline: bots fill"),
    ("pick timer", "NEW", "30 s, auto lock, beeps <= 5"),
    ("type roster", "NEW", "MELEE / RANGED / MAGIC"),
    ("spotlight", "NEW", "cast intro -> idle, 24 fps"),
    ("lobby -> match", "LIVE", "unchanged, live portrait rebake"),
]
for i, (k, n, v) in enumerate(rows):
    text(9, 21 + i, "[OK]", HEAD)
    text(14, 21 + i, f"{k} ".ljust(18, "."), MID)
    text(33, 21 + i, n, HEAD if n != "STUB" else AMBER)
    text(39, 21 + i, v, BRIGHT)
text(9, 30, "keys: arrows/WS + Enter, 1-3, Esc (select -> menu)", DIM)
text(9, 31, "AppImage passes --ui-live: menu now opens first", AMBER)
text(9, 32, "--start-select bot|public|private [--champion]", AMBER)

# Right: wielding.
panel(76, 19, W - 7, 33, "WIELD  //  ITEM WIELD MODELS ON THE PLAYER SKELETON")
ids = [
    ("kolodion", "1387", "staff of fire  [534, 567]"),
    ("wise old man", "1381", "staff of air   [534, 567]"),
    ("ranger", "12926", "toxic blowpipe  14403"),
    ("bosses (13)", "----", "own skeleton: cannot wield"),
]
for i, (use, n, src) in enumerate(ids):
    text(79, 21 + i, use.ljust(16), BRIGHT)
    text(96, 21 + i, n, HEAD)
    text(103, 21 + i, src, MID)
text(79, 26, "RULE  first weapon in inventory > cosmetic default", WHITE)
text(79, 27, "opcodes 23 (+offset) / 24 / 78 read from item config", BRIGHT)
text(79, 28, "merged before animation: swings with the hand", BRIGHT)
text(79, 29, "fix: face colour cache keyed (ptr, faces) - no green", BRIGHT)
text(79, 30, "spotlight loads a hero in ~68 ms, CPU raster", BRIGHT)
text(79, 31, "streams into HUD atlas slot (1024,760) 384x512", AMBER)
text(79, 32, "screens: hud/screens.rs   state: menu.rs", AMBER)

# Verification strip.
panel(6, 35, W - 7, 42, "OPERATOR CHECKS")
checks = [
    ("TESTS", "cargo test --workspace", "374 PASS / 0 FAIL", "+13: menu, roster groups, timer, wield, spotlight"),
    ("CLIPPY", "cargo clippy -D warnings", "CLEAN", "cargo fmt clean"),
    ("LIVE", "--ui-live / --start-select", "SCREENSHOTS", "menu, King Awowogei, Kolodion + staff of fire"),
    ("COMMITS", "d4984aa  client: main menu, hero spotlight select, wielded weapons", "", ""),
    ("", "eac4365  docs: main menu, spotlight hero select and wielding", "", ""),
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
text(9, 45, "1. Match server (M7) to light up Public / Private.   2. Menu music + click sounds by ear.", BRIGHT)
text(9, 46, "3. Settings / profile buttons on the tablet.   4. Off-hand shields & two-handers for humanoids.", BRIGHT)
text(9, 47, "Not clicked through by a person yet: menu -> select -> lobby was verified by screenshots and tests.", AMBER)

center(49, " THERE IS NO SPOON.  THERE IS A STAFF OF FIRE, AND IT IS IN YOUR HAND. ", HEAD)

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
