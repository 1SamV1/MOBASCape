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
rng = random.Random(1999)
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
center(9, "S Y S T E M   A U D I O   //   T H E   A R E N A   H A S   A   V O I C E", WHITE)

panel(6, 13, W - 7, 17, "TRANSMISSION")
text(9, 14, "> Wake up, Champion...", BRIGHT)
text(9, 15, "> The arena was silent. Fifteen of sixteen heroes made no sound at all.", MID)
text(9, 16, "> Follow the white rabbit:  index 4  ->  opcode 14  ->  rodio  ->  your ears.", BRIGHT)

# Left: what loaded.
panel(6, 19, 74, 33, "CONSTRUCT LOADED")
rows = [
    ("CACHE FRAME SOUNDS", "sounds_between: loops + one-shots"),
    ("TICK SNAPSHOT", "AnimClock at the top of update()"),
    ("SHARED TABLE", "data/sounds.json, 12 named sounds"),
    ("OVERRIDES", "champion `sounds` wins by name"),
    ("FOG SAFE", "hidden units never make a sound"),
    ("LANE QUIET", "only your fights + champion duels hit"),
    ("DE-DUPE", "one copy of each id per frame"),
    ("KIT RESPECT", "Awowogei keeps his own q_/w_/e_/r_"),
    ("PRELOAD", "table + every unit clip's frame sounds"),
]
for i, (k, v) in enumerate(rows):
    text(9, 21 + i, "[OK]", HEAD)
    text(14, 21 + i, f"{k} ".ljust(20, "."), MID)
    text(35, 21 + i, v, BRIGHT)
text(9, 31, "every champion now: hits / shots / casts / buys / ranks", AMBER)

# Right: sound id table.
panel(76, 19, W - 7, 33, "SOUND ID MATRIX  (RuneLite SoundEffectID)")
ids = [
    ("2498", "melee_hit / ranged_hit", "ATTACK_HIT"),
    (" 511", "blocked_hit", "ZERO_DAMAGE_SPLAT"),
    (" 510", "player_hurt", "TAKE_DAMAGE_SPLAT"),
    ("1930", "release / cast (3 pitches)", "NPC_TELEPORT_WOOSH"),
    ("3924", "shop_buy", "GE_COIN_TINKLE"),
    ("3928", "shop_sell", "GE_COLLECT_BLOOP"),
    ("2266", "skill_point", "UI_BOOP"),
    ("2734", "structure_fall", "TREE_FALLING"),
    ("3843", "Graardor swing   (cache frame 7)", "SEQ 7018"),
    ("3834", "Graardor/K'ril stomp (cache)", "SEQ 7016/4070"),
    ("3865", "K'ril claw       (cache frame 6)", "SEQ 6948"),
]
for i, (n, use, src) in enumerate(ids):
    text(79, 21 + i, n, HEAD)
    text(85, 21 + i, use.ljust(34), BRIGHT)
    text(120, 21 + i, src, DIM if src.startswith("SEQ") else MID)

# Verification strip.
panel(6, 35, W - 7, 42, "OPERATOR CHECKS")
checks = [
    ("TESTS", "cargo test --workspace", "359 PASS / 0 FAIL", "+3 new: frame windows, wrap, table"),
    ("CLIPPY", "cargo clippy -D warnings", "CLEAN", ""),
    ("LIVE", "--demo-duel --timescale 4", "CUES TRACED", "graardor 3843+2498+510 / kril 3834 / ranger 1930+510"),
    ("COMMITS", "e82257f  audio: shared combat sounds + cache frame sounds", "", ""),
    ("", "e1f21fb  docs: record shared combat sounds and cache frame sounds", "", ""),
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
text(9, 45, "1. Hand-pick ability sounds for the other 15 kits (needs your ear: creative call).", BRIGHT)
text(9, 46, "2. Announcer + music hook (M9).   3. Tune volumes in data/sounds.json after a real playtest.", BRIGHT)
text(9, 47, "Not playtested by ear yet: the code queues the right ids; nobody has listened to the mix.", AMBER)

center(49, " THERE IS NO SPOON.  THERE IS ONLY SOUND 2498. ", HEAD)

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
