#!/usr/bin/env python3
"""MobaScape progress screen as a Matrix (2000) era BBS / scene .NFO.

Block-shaded logo, ░▒▓ fades, -=[ ]=- headers and digital rain behind it.
Writes <out>.txt (plain), <out>.ans (24-bit ANSI colour) and <out>.png.

Usage: python3 tools/matrix_bbs.py docs/PROGRESS_MATRIX
"""
import random
import sys

from PIL import Image, ImageDraw, ImageFont

W, H = 150, 62
out = sys.argv[1]

WHITE = (235, 255, 235)
HEAD = (200, 255, 200)
BRIGHT = (0, 255, 70)
MID = (0, 190, 45)
DIM = (0, 115, 28)
FAINT = (0, 52, 12)
AMBER = (210, 255, 120)
RED = (255, 90, 70)
CYAN = (110, 255, 210)
BLACK = (0, 0, 0)

grid = [[(" ", FAINT) for _ in range(W)] for _ in range(H)]


def put(x, y, ch, fg):
    if 0 <= x < W and 0 <= y < H:
        grid[y][x] = (ch, fg)


def text(x, y, s, fg=BRIGHT):
    for i, ch in enumerate(s):
        put(x + i, y, ch, fg)


def center(y, s, fg=BRIGHT):
    text((W - len(s)) // 2, y, s, fg)


def clear(x0, y0, x1, y1):
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            put(x, y, " ", BLACK)


# --- digital rain -----------------------------------------------------------
rng = random.Random(1999)
GLYPHS = "0123456789ABCDEF:.=*+-<>|^~$#@%&?!ZQXJ7kT" + "ｱｲｳｴｵｶｷｸｹｺ"
GLYPHS = "".join(c for c in GLYPHS if ord(c) < 0x2000)
for x in range(W):
    if rng.random() < 0.22:
        continue
    for _ in range(rng.choice([1, 2, 2])):
        head = rng.randrange(-4, H + 8)
        length = rng.randrange(6, 30)
        for k in range(length):
            y = head - k
            if not 0 <= y < H:
                continue
            col = HEAD if k == 0 else BRIGHT if k < 3 else MID if k < length * 0.45 else DIM if k < length * 0.8 else FAINT
            put(x, y, rng.choice(GLYPHS), col)


# --- NFO frame pieces -------------------------------------------------------
def fade_bar(y, x0, x1):
    """░▒▓█ ... █▓▒░ rule across a section."""
    clear(x0, y, x1, y)
    ends = "░▒▓"
    for i, ch in enumerate(ends):
        put(x0 + i, y, ch, [DIM, MID, BRIGHT][i])
        put(x1 - i, y, ch, [DIM, MID, BRIGHT][i])
    for x in range(x0 + 3, x1 - 2):
        put(x, y, "▄" if (x // 2) % 2 else "▀", MID if (x // 7) % 2 else DIM)


def header(y, label, x0=4, x1=W - 5):
    clear(x0, y, x1, y)
    s = f"-=[ {label} ]=-"
    mid = (x0 + x1) // 2
    sx = mid - len(s) // 2
    for x in range(x0, x1 + 1):
        put(x, y, "─", DIM)
    put(x0, y, "■", MID)
    put(x1, y, "■", MID)
    text(sx - 2, y, "  ", BLACK)
    text(sx, y, s, HEAD)
    text(sx + len(s), y, "  ", BLACK)


def box(x0, y0, x1, y1, label):
    clear(x0, y0, x1, y1)
    for x in range(x0, x1 + 1):
        put(x, y0, "▀", MID)
        put(x, y1, "▄", MID)
    for y in range(y0, y1 + 1):
        put(x0, y, "█", DIM)
        put(x1, y, "█", DIM)
    put(x0, y0, "▄", BRIGHT)
    put(x1, y0, "▄", BRIGHT)
    tag = f"▐ {label} ▌"
    text(x0 + 3, y0, tag, HEAD)


def elite(s):
    """sCeNe cAsE: lower vowels, upper the rest."""
    return "".join(c.lower() if c.lower() in "aeiou" else c.upper() for c in s)


# --- the logo: 7-row block font with a ▓▒░ drop shadow -----------------------
LOGO = {
    "M": ["██▄   ▄██", "███▄ ▄███", "██ ▀█▀ ██", "██     ██", "██     ██", "██     ██"],
    "O": [" ▄█████▄ ", "██     ██", "██     ██", "██     ██", "██     ██", " ▀█████▀ "],
    "B": ["███████▄ ", "██     ██", "███████▀ ", "██     ██", "██     ██", "███████▀ "],
    "A": [" ▄█████▄ ", "██     ██", "█████████", "██     ██", "██     ██", "██     ██"],
    "S": [" ▄██████▄", "██       ", " ▀█████▄ ", "       ██", "       ██", "▀██████▀ "],
    "C": [" ▄██████▄", "██       ", "██       ", "██       ", "██       ", " ▀██████▀"],
    "P": ["███████▄ ", "██     ██", "███████▀ ", "██       ", "██       ", "██       "],
    "E": ["█████████", "██       ", "███████  ", "██       ", "██       ", "█████████"],
}
LOGO_ROWS = 6
LOGO_GRAD = [WHITE, HEAD, BRIGHT, BRIGHT, MID, DIM]


def logo(y, word):
    width = len(word) * 10 - 1
    x0 = (W - width) // 2
    clear(x0 - 4, y - 1, x0 + width + 4, y + LOGO_ROWS)
    # Shadow first, one right and one down, then the face over it.
    for pass_ in ("shadow", "face"):
        for i, ch in enumerate(word):
            for r in range(LOGO_ROWS):
                for c, px in enumerate(LOGO[ch][r]):
                    if px == " ":
                        continue
                    x = x0 + i * 10 + c
                    if pass_ == "shadow":
                        if grid[y + r + 1][x + 1][0] == " ":
                            put(x + 1, y + r + 1, "░", DIM)
                    else:
                        put(x, y + r, px, LOGO_GRAD[r])
    return x0, width


# ============================================================================
GOLD = (255, 210, 90)
WOOD = (176, 128, 70)
WOOD_DIM = (110, 78, 40)
STONE = (206, 192, 156)
STONE_DIM = (128, 116, 92)
ORANGE = (255, 152, 31)
SPOON = [WHITE, (214, 255, 222), (150, 214, 170), (96, 160, 116), (70, 120, 86), (52, 92, 64), (40, 70, 50), (30, 54, 38)]
# One colour per lane, after its cache skill icon.
LANE_COL = {
    "Top": (196, 204, 210),      # Defence 199: steel
    "Jungle": (196, 150, 92),    # Hunter 220: paw-print brown
    "Mid": (255, 110, 90),       # Runecraft 215: the red rift
    "Carry": (120, 230, 120),    # Fletching 208: green fletch
    "Support": (90, 255, 150),   # Herblore 205: the herb
}
LANES = ["Top", "Jungle", "Mid", "Carry", "Support"]
RED_LANES = ["Top", "Mid", "Carry", "Support", "Jungle"]


def bot_lanes(player):
    """hero_select::bot_lanes: the lanes nobody called, in draft order."""
    taken = player or "Mid"
    return [lane for lane in RED_LANES if lane != taken][:4]


def bent_logo(y, word, bend=4):
    """The logo bent like the Oracle's spoon. Drawn at half-block height, so
    each column can drop by half a row: the ends sag `bend` half-rows on a
    parabola and the middle holds. Re-encoded to ▀ ▄ █ after; each cell
    keeps the gradient of the row it came from, so the letters stay lit
    from above while the word curves."""
    width = len(word) * 10 - 1
    x0 = (W - width) // 2
    halves = {"█": (1, 1), "▀": (1, 0), "▄": (0, 1), " ": (0, 0)}
    src = [[0] * width for _ in range(LOGO_ROWS * 2)]
    for i, letter in enumerate(word):
        for r in range(LOGO_ROWS):
            for c, px in enumerate(LOGO[letter][r]):
                top, bottom = halves[px]
                src[r * 2][i * 10 + c] = top
                src[r * 2 + 1][i * 10 + c] = bottom
    rows = LOGO_ROWS + (bend + 1) // 2
    mid = (width - 1) / 2
    drop = [round(bend * ((x - mid) / mid) ** 2) for x in range(width)]
    dst = [[None] * width for _ in range(rows * 2)]
    for x in range(width):
        for sy in range(LOGO_ROWS * 2):
            if src[sy][x]:
                dst[sy + drop[x]][x] = sy
    clear(x0 - 2, y, x0 + width + 2, y + rows - 1)
    cells = {}
    for r in range(rows):
        for x in range(width):
            t, b = dst[2 * r][x], dst[2 * r + 1][x]
            if t is None and b is None:
                continue
            ch = "█" if (t is not None and b is not None) else ("▀" if t is not None else "▄")
            cells[(x0 + x, y + r)] = (ch, LOGO_GRAD[(t if t is not None else b) // 2])
    for (x, yy) in cells:
        if (x + 1, yy + 1) not in cells:
            put(x + 1, yy + 1, "░", DIM)
    for (x, yy), (ch, col) in cells.items():
        put(x, yy, ch, col)
    return x0, width, rows


def plaque(x0, y0):
    """The carved sign over the Oracle's kitchen door."""
    inner = 20
    lines = [
        ("╔" + "═" * inner + "╗", WOOD),
        ("║" + "T E M E T".center(inner) + "║", GOLD),
        ("║" + "N O S C E".center(inner) + "║", GOLD),
        ("╟" + "─" * inner + "╢", WOOD_DIM),
        ("║" + "know thy lane:".center(inner) + "║", HEAD),
        ("║" + "call it, or don't.".center(inner) + "║", MID),
        ("║" + "it's not a rule.".center(inner) + "║", MID),
        ("╚" + "═" * inner + "╝", WOOD),
    ]
    clear(x0 - 1, y0, x0 + inner + 2, y0 + len(lines) - 1)
    for k, (line, col) in enumerate(lines):
        for j, ch in enumerate(line):
            put(x0 + j, y0 + k, ch, WOOD if ch in "╔╗╚╝═║╟╢" else WOOD_DIM if ch == "─" else col)


def spoon(x0, y0):
    """The spoon, bowl up, its neck bending away to the right."""
    art = [
        "    ▄▄▄▄▄",
        "  ▄█▀   ▀█▄",
        "  █▌     ▐█",
        "  ▀█▄   ▄█▀",
        "    ▀███▀",
        "      ▐█",
        "       ▀█▄",
        "         ▀▀█▄▄▄▄",
    ]
    clear(x0 - 1, y0, x0 + 18, y0 + len(art) - 1)
    for k, line in enumerate(art):
        for j, ch in enumerate(line):
            if ch != " ":
                put(x0 + j, y0 + k, ch, SPOON[k])


clear(2, 2, W - 3, 2)
bent_logo(3, "MOBASCAPE")
plaque(5, 3)
spoon(W - 24, 3)
text(5, 2, "▐ kITCHEN dOOR ▌", DIM)
text(W - 25, 2, "▐ nO sPOON. nO rULE. ▌", DIM)
center(2, "eNTERING: tHE oRACLE'S kITCHEN   [cOOKIES ▓▓▓▓▓▓▓▓▓▓ 100%]", MID)
fade_bar(1, 2, W - 3)
center(11, "  ░▒▓  n O D E   1 3 :   t H E   o R A C L E   //   c A L L   y O U R   l A N E ,   b E N D   t H E   r U L E  ▓▒░  ", AMBER)
center(12, "  " + elite("some rules can be bent. others can be broken. this one was never a rule.") + "  ", WHITE)
fade_bar(13, 2, W - 3)

# --- release info -------------------------------------------------------------
header(14, elite("release info"))
clear(4, 15, W - 5, 18)
info = [
    ("rELEASE dATE", "2026-11-24", "cOMMIT", "a5bb14e  ->  sam/main"),
    ("tYPE", "hERO sELECT: tHE lANE pICKER", "tESTS", "427 pass // clippy clean"),
    ("wAS", "BOTS IN FIXED SLOTS, NO WAY TO SAY", "nOW", "5 lane calls, bots fill the gaps"),
    ("wAS", "TIMER NOTE OVER THE STATS AT 720P", "nOW", "it bows out where it can't fit"),
]
for i, (k1, v1, k2, v2) in enumerate(info):
    yy = 15 + i
    text(8, yy, f"{k1:.<16}", DIM)
    text(25, yy, v1, RED if k1 == "wAS" else BRIGHT)
    text(76, yy, f"{k2:.<12}", DIM)
    text(89, yy, v2, HEAD)

# --- left: the lane bar, in stone ------------------------------------------------
box(4, 20, 74, 45, elite("kitchen.prg // the lane picker"))
IW = 60


def wall(left, middle, right, fill="═"):
    run = IW + 4 - len(left) - len(middle) - len(right)
    half = run // 2
    return " " + left + fill * half + middle + fill * (run - half) + right + " "


def row(inner):
    assert len(inner) <= IW, (len(inner), inner)
    return " ║ " + inner.ljust(IW) + " ║ "


def tab(lane, mine):
    short = {"Top": "Top", "Jungle": "Jng", "Mid": "Mid", "Carry": "Cry", "Support": "Sup"}[lane]
    return ("«▣" + short + "»") if mine else ("[▣" + short + "]")


mine = "Jungle"
bar = "Your lane: " + "".join(tab(lane, lane == mine) for lane in LANES) + "  Show: All ▣ ▣ ▣"
blue = [("You", "Awowogei", mine)] + [(f"Bot {k + 1}", hero, lane) for k, (hero, lane) in enumerate(
    zip(["Kree'arra", "Wise O.M.", "Cmdr Zil", "Big Frog"], bot_lanes(mine)))]
red = [(f"Bot {k + 1}", hero, lane) for k, (hero, lane) in enumerate(
    zip(["Kalph. Q", "A. Wyvern", "Vorkath", "Callisto", "Venenatis"], RED_LANES))]
CW = 27


def column_row(name, hero, lane):
    return "│" + f"▣ {name:<6}{hero:<10}◆ {lane:<7}"[:CW].ljust(CW) + "│"


mock = [
    wall("╔═[958]", "[954 x n]", "[959]═╗"),
    row(bar),
    row(""),
    row("┌─ Blue team " + "─" * (CW - 12) + "┐ ┌─ Red team " + "─" * (CW - 11) + "┐"),
]
for (bn, bh, bl), (rn, rh, rl) in zip(blue, red):
    mock.append(row(column_row(bn, bh, bl) + " " + column_row(rn, rh, rl)))
mock.append(row("└" + "─" * CW + "┘ └" + "─" * CW + "┘"))
mock.append(wall("╚═[960]", "[956 x n]", "[961]═╝"))
assert len({len(line) for line in mock}) == 1, [len(line) for line in mock]
legend = [
    "",
    " icons, cache index 8:  199 defence  TOP     220 hunter  JNG",
    "                        215 runecraft MID    208 fletch  CRY",
    "                        205 herblore SUP",
    "",
    " click your lane again: the call is withdrawn.",
    " nobody calls for you. your call rides into the next draft.",
    " lobby cards: name / class / stats / lane (portrait shrinks)",
    " --lane jungle   sets a call from the shell, for screenshots",
]
for i, line in enumerate(mock + legend):
    assert len(line) <= 68, (len(line), line)
    in_mock = i < len(mock)
    for j, ch in enumerate(line):
        if ch == " ":
            continue
        if in_mock:
            if ch in "╔╗╚╝═║":
                col = STONE
            elif ch in "┌┐└┘─│":
                col = STONE_DIM
            elif ch == "▣":
                col = GOLD
            elif ch in "«»":
                col = WHITE
            elif ch == "◆":
                col = ORANGE
            elif i in (0, len(mock) - 1):
                col = DIM
            elif i == 1:
                col = ORANGE if j < 14 or j > 46 else HEAD
            else:
                col = MID
        else:
            col = MID if i - len(mock) < 5 else HEAD
            if ch.isdigit():
                col = AMBER
        put(6 + j, 22 + i, ch, col)
# Colour the lane names in the mock by lane (and the called tab white-hot).
for i, line in enumerate(mock):
    for lane in LANES:
        for short in ({"Jungle": "Jng", "Carry": "Cry", "Support": "Sup"}.get(lane, lane), lane):
            k = line.find(short)
            while k != -1:
                hot = i == 1 and line[k - 2:k] == "«▣"
                text(6 + k, 22 + i, short, WHITE if hot else LANE_COL[lane])
                k = line.find(short, k + 1)

# --- right: the matrix of calls ---------------------------------------------------
box(76, 20, W - 5, 45, elite("oracle.dat // the matrix of calls"))
RX = 79
text(RX, 22, "YOU CALL", HEAD)
for k in range(4):
    text(RX + 12 + k * 10, 22, f"BOT {k + 1}", HEAD)
text(RX + 52, 22, "NOTE", HEAD)
text(RX, 23, "─" * 64, FAINT)
for k, call in enumerate([None] + LANES):
    yy = 24 + k
    text(RX, yy, call or "(nothing)", LANE_COL[call] if call else DIM)
    if call:
        put(RX - 2, yy, "►", WHITE)
    for j, lane in enumerate(bot_lanes(call)):
        text(RX + 12 + j * 10, yy, lane, LANE_COL[lane])
    short = {"Jungle": "jng", "Carry": "cry", "Support": "sup"}.get(call, (call or "").lower())
    note = "mid open" if call is None else "default" if call == "Mid" else f"bot: {short}>mid"
    assert len(note) <= 12, note
    text(RX + 52, yy, note, DIM)
text(RX, 30, "─" * 64, FAINT)
text(RX, 31, "red, every draft:", DIM)
for j, lane in enumerate(RED_LANES):
    text(RX + 18 + j * 9, 31, lane, LANE_COL[lane])

text(RX, 33, "SOME RULES CAN BE BENT", HEAD)
text(RX + 23, 33, "// morpheus, the construct, 1999", DIM)
rules = [
    ("BENT", "your call", "optional. click it again to take it back."),
    ("BENT", "two calls", "two humans may call one lane. fine."),
    ("KEPT", "bot calls", "bots spawn in the lane they called."),
    ("KEPT", "no call", "old roles: top, bot x2, jungle; mid's yours."),
    ("NONE", "in match", "nothing checks a call. ever."),
]
for k, (verdict, what, why) in enumerate(rules):
    assert RX + 19 + len(why) < W - 6, why
    yy = 34 + k
    text(RX + 2, yy, verdict, {"BENT": AMBER, "KEPT": BRIGHT, "NONE": RED}[verdict])
    text(RX + 8, yy, what, CYAN)
    text(RX + 19, yy, why, MID)

text(RX, 40, "ROLE MAP", HEAD)
text(RX + 10, 40, "top Lane(0)  mid Lane(1)  cry+sup Lane(2)  jng Jungle", MID)
text(RX, 42, "RIGIDITY", HEAD)
text(RX + 10, 42, "[", DIM)
text(RX + 11, 42, "░" * 20, FAINT)
text(RX + 31, 42, "]   0%", HEAD)
text(RX + 39, 42, "// a note to the team", DIM)
text(RX, 43, "+6 TESTS", HEAD)
text(RX + 10, 43, "fills, toggles, defaults, tabs at 1024/1280/1920", MID)

# --- sysop's note ------------------------------------------------------------
header(47, elite("sysop's note // don't worry about the vase"))
clear(4, 48, W - 5, 49)
center(48, "sTILL oPEN: bATTLE sPELLS (p4.3), pASSIVE rOW (p4.4), tHE lOBBY iN sTONE (p4.7), cLIFFS + bRIDGES + gOD sTATUES (p5), gRASS hUE 100 vS 111.", HEAD)
center(49, "hero_select::Lane // bot_lanes // RED_LANES // lane_rects // tabs_hold_labels // icon_tab // bot_lineup(lanes) // --lane", DIM)

# --- roadmap -------------------------------------------------------------------
header(51, elite("roadmap // docs/ROADMAPPLAN.md"))
clear(4, 52, W - 5, 56)
phases = [
    ("P1 LIGHT", 5), ("P2 MINIMAP", 6), ("P3 SHOP", 7), ("P4 SELECT", 5),
    ("P5 ARENA", 1), ("P6 HUD", 4), ("P7 POST", 0),
]
for i, (name, done) in enumerate(phases):
    x = 8 + (i % 4) * 35
    yy = 52 + (i // 4) * 2
    text(x, yy, f"{name:<11}", CYAN)
    text(x + 11, yy, "[", DIM)
    for j in range(10):
        put(x + 12 + j, yy, "▓" if j < done else "░", BRIGHT if j < done else FAINT)
    text(x + 22, yy, f"] {done * 10:>3}%", HEAD if done else FAINT)
text(8, 56, ">> next: battle spells (P4.3), the lobby in the stone kit (P4.7), cliffs/bridges/statues from cache objects (P5)", MID)

fade_bar(57, 2, W - 3)
clear(10, 58, W - 11, 61)
greetz = "gREETZ: sPRITE 199 (dEFENCE, tOP) * 220 (hUNTER, jUNGLE) * 215 (rUNECRAFT, mID) * 208 (fLETCHING, cARRY) * 205 (hERBLORE)"
assert len(greetz) <= 128, len(greetz)
center(58, greetz, MID)
center(59, "nOT AFFILIATED WITH JAGEX. cACHE ASSETS STAY OUT OF GIT. aLWAYS.", DIM)
center(60, "  n o   o n e   c a n   t e l l   y o u   y o u r   l a n e .   y o u   j u s t   k n o w   i t .  ", HEAD)
center(61, "-=[ mObAsCaPe // 2026 // nODE 13: tHE oRACLE // 57.6k // sYSoP: rIFT // 1 cOMMIT, 5 lANES, 0 rULES ]=-", AMBER)

# --- outputs ------------------------------------------------------------------
with open(out + ".txt", "w") as f:
    for row in grid:
        f.write("".join(c for c, _ in row).rstrip() + "\n")

with open(out + ".ans", "w") as f:
    for row in grid:
        last = None
        line = []
        for ch, fg in row:
            if fg != last:
                line.append(f"\x1b[38;2;{fg[0]};{fg[1]};{fg[2]}m")
                last = fg
            line.append(ch)
        f.write("".join(line) + "\x1b[0m\n")

font = ImageFont.truetype("/usr/share/fonts/TTF/DejaVuSansMono.ttf", 15)
bold = ImageFont.truetype("/usr/share/fonts/TTF/DejaVuSansMono-Bold.ttf", 15)
cw = int(round(font.getlength("M")))
ch_h = 18
img = Image.new("RGB", (W * cw + 16, H * ch_h + 16), BLACK)
d = ImageDraw.Draw(img)
for y, row in enumerate(grid):
    for x, (c, fg) in enumerate(row):
        if c == " ":
            continue
        px, py = 8 + x * cw, 8 + y * ch_h
        # Draw block elements as rectangles so they tile without gaps.
        if c in "█▀▄▌▐■":
            boxes = {
                "█": (0, 0, 1, 1), "▀": (0, 0, 1, 0.5), "▄": (0, 0.5, 1, 1),
                "▌": (0, 0, 0.5, 1), "▐": (0.5, 0, 1, 1), "■": (0.2, 0.3, 0.8, 0.75),
            }[c]
            d.rectangle(
                (px + boxes[0] * cw, py + boxes[1] * ch_h, px + boxes[2] * cw - 1, py + boxes[3] * ch_h - 1),
                fill=fg,
            )
        elif c in "░▒▓":
            density = {"░": 0.25, "▒": 0.5, "▓": 0.75}[c]
            for sy in range(0, ch_h, 2):
                for sx in range(0, cw, 2):
                    if ((sx // 2) + (sy // 2)) % 4 < density * 4:
                        d.rectangle((px + sx, py + sy, px + sx + 1, py + sy + 1), fill=fg)
        else:
            f = bold if fg in (HEAD, WHITE, BRIGHT, AMBER, CYAN) else font
            d.text((px, py), c, fill=fg, font=f)
img.save(out + ".png")
print(f"wrote {out}.txt/.ans/.png ({img.size[0]}x{img.size[1]})")
