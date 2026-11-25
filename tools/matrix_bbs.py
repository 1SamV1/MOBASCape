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
import colorsys

GOLD = (255, 210, 90)
ICE = (170, 230, 255)
STONE = (206, 192, 156)
STONE_DIM = (128, 116, 92)
ORANGE = (255, 152, 31)
CHROME = [WHITE, (214, 255, 222), (120, 196, 140), (0, 128, 36), (0, 205, 64), (176, 255, 196)]


def construct_logo(y, word):
    """Split-chrome logo standing in the Construct's white room: a light
    top, a dark band, a bright lip, a one-pixel slit cut through the fourth
    row, and the floor's reflection of the bottom two rows underneath."""
    width = len(word) * 10 - 1
    x0 = (W - width) // 2
    clear(x0 - 2, y, x0 + width + 2, y + LOGO_ROWS + 1)
    for i, letter in enumerate(word):
        for r in range(LOGO_ROWS):
            for c, px in enumerate(LOGO[letter][r]):
                if px == " ":
                    continue
                x = x0 + i * 10 + c
                # The slit: full blocks on row 3 keep only their lower half.
                ch = "▄" if (r == 3 and px == "█") else px
                put(x, y + r, ch, CHROME[r])
                # The floor: rows 5 and 4 mirrored, fading out.
                if r in (5, 4):
                    ry = y + LOGO_ROWS + (5 - r)
                    put(x, ry, "▒" if r == 5 else "░", (70, 110, 82) if r == 5 else (38, 62, 46))
    return x0, width


def rack(x0, y0, rows, colour):
    """A loading-program rack: three shelves of three-letter item codes."""
    clear(x0 - 1, y0, x0 + 21, y0 + 6)
    text(x0, y0, "╓───┬───┬───┬───┬───╖", ICE)
    for k, items in enumerate(rows):
        yy = y0 + 1 + k * 2
        text(x0, yy, "║", ICE)
        for j, item in enumerate(items):
            text(x0 + 1 + j * 4, yy, item, colour if (j + k) % 2 == 0 else WHITE)
            text(x0 + 4 + j * 4, yy, "│" if j < 4 else "║", ICE)
        if k < len(rows) - 1:
            text(x0, yy + 1, "╟───┼───┼───┼───┼───╢", (90, 150, 170))
    text(x0, y0 + 6, "╙───┴───┴───┴───┴───╜", ICE)


clear(2, 2, W - 3, 2)
construct_logo(3, "MOBASCAPE")
rack(5, 3, [["AGS", "DDS", "WHP", "TBW", "SCY"], ["GMA", "DCB", "KOD", "SNG", "BP "], ["RAP", "SPR", "ZCB", "MSB", "TRI"]], GOLD)
rack(W - 26, 3, [["TRV", "BCP", "TAS", "PEG", "PRI"], ["FRY", "ANG", "TRT", "ROS", "ULT"], ["GLY", "MAG", "ETR", "FTO", "DPB"]], CYAN)
text(5, 2, "▐ rACK 01 // wEAPONS ▌", DIM)
text(W - 26, 2, "▐ rACK 02 // gEAR    ▌", DIM)
center(2, "lOADING pROGRAM: cONSTRUCT  [▓▓▓▓▓▓▓▓▓▓] 100%", MID)
fade_bar(1, 2, W - 3)
center(11, "  ░▒▓  n O D E   1 2 :   t H E   c O N S T R U C T   //   l O A D   t H E   s T O N E ,   p A I N T   t H E   m A P  ▓▒░  ", AMBER)
center(12, "  " + elite("this is the construct. it's our loading program. we loaded the grand exchange.") + "  ", WHITE)
fade_bar(13, 2, W - 3)

# --- release info -------------------------------------------------------------
header(14, elite("release info"))
clear(4, 15, W - 5, 18)
info = [
    ("rELEASE dATE", "2026-11-24", "cOMMIT", "c2237b4  ->  sam/main"),
    ("tYPE", "fRONT eND + sHOP + mAP lOOK", "tESTS", "421 pass // clippy clean"),
    ("wAS", "FLAT DARK PANELS, A TEXT-LIST SHOP", "nOW", "cache stone kit, grand exchange"),
    ("wAS", "OLIVE HUE 73, BARK CONES, DEBUG TEXT", "nOW", "hue 86-100, trees in leaf, clean hud"),
]
for i, (k1, v1, k2, v2) in enumerate(info):
    yy = 15 + i
    text(8, yy, f"{k1:.<16}", DIM)
    text(25, yy, v1, RED if k1 == "wAS" else BRIGHT)
    text(76, yy, f"{k2:.<12}", DIM)
    text(89, yy, v2, HEAD)

# --- left: the stone kit, loaded ----------------------------------------------
box(4, 20, 74, 45, elite("construct.prg // the stone kit"))
IW = 56  # the mock window's inner width
LW, RW_ = 12, 13
MW = IW - LW - RW_ - 2
left_col = ["Categories", "►Recommend", " All", " Melee", " Ranged"]
mid_col = [" Starter   ▣   ▣   ▣", " Core      ▣   ▣   ▣", " Upgrades  ▣ > ▣ > ▣",
           " Recipes   ▣ + ▣ > ▣", "           ▣ owned (green)"]
right_col = ["▣ Bronze sw", "  300 coins", "Builds into", " ▣ Iron sw", "[Buy][Sell]"]


def wall(left, middle, right, fill="═"):
    run = IW + 4 - len(left) - len(middle) - len(right)
    half = run // 2
    return " " + left + fill * half + middle + fill * (run - half) + right + " "


def row(inner):
    assert len(inner) <= IW, inner
    return " ║ " + inner.ljust(IW) + " ║ "


title = "G r a n d   E x c h a n g e"
ge = [
    wall("╔═[958]", "[954 x n]", "[959]═╗"),
    row(title.center(IW - 4) + "[X]"),
    row("┌" + "─" * (LW - 2) + "┐ ┌" + "─" * (MW - 2) + "┐ ┌" + "─" * (RW_ - 2) + "┐"),
]
for l, m, r in zip(left_col, mid_col, right_col):
    ge.append(row("│" + l.ljust(LW - 2) + "│ │" + m.ljust(MW - 2) + "│ │" + r.ljust(RW_ - 2) + "│"))
ge.append(row("└" + "─" * (LW - 2) + "┘ └" + "─" * (MW - 2) + "┘ └" + "─" * (RW_ - 2) + "┘"))
bag = "Your items  ▣210 ▣210 ▣245 ▣385"
ge.append(row(bag + "[Undo]  $ 6521".rjust(IW - len(bag))))
ge.append(wall("╚═[960]", "[956 x n]", "[961]═╝"))
assert len({len(line) for line in ge}) == 1, [len(line) for line in ge]
legend = [
    "",
    " tile 297 ░ under all    inset: 962/967 + 972-975 at 62%",
    " buttons 293/294 notched   parchment 1017   slot 1077",
    " fonts p11 494  p12 495  b12 496  q8 497   one kit, two UIs",
    "",
    " hero select: title bar + leave, both team columns, role",
    " tabs, best-fit roster with PICKED, 3d spotlight kept",
    " main menu:   the same stone tablet, notched buttons",
    "",
    " click a bag slot -> select it -> sell. undo refunds.",
]
for i, line in enumerate(ge + legend):
    assert len(line) <= 68, (len(line), line)
    for j, ch in enumerate(line):
        if ch == " ":
            continue
        if i < len(ge):
            if ch in "╔╗╚╝═║":
                col = STONE
            elif ch in "┌┐└┘─│":
                col = STONE_DIM
            elif ch == "▣":
                col = GOLD
            elif ch in "><+":
                col = ORANGE
            elif ch.isdigit() or ch == "$":
                col = (120, 255, 120) if i == len(ge) - 2 and j < 40 else AMBER
            elif i == 1 and ch != "X":
                col = ORANGE
            elif "[" in line[max(0, j - 6):j + 1] and ch.isdigit():
                col = DIM
            else:
                col = HEAD if i in (1, len(ge) - 2) else MID
            if i in (0, len(ge) - 1) and (ch.isdigit() or ch in "[]xn"):
                col = DIM
        else:
            col = DIM if ch in "░" else (MID if i - len(ge) < 4 else HEAD)
            if ch.isdigit():
                col = AMBER
        put(6 + j, 22 + i, ch, col)

# --- right: the grade, measured -------------------------------------------------
box(76, 20, W - 5, 45, elite("grade.dat // measured, not guessed"))
RX = 79
text(RX, 22, "HUE OF LIT GRASS", HEAD)
ruler_x = RX + 20
HUE_LO, HUE_HI, RW = 60, 140, 32
for hue in range(HUE_LO, HUE_HI + 1, 20):
    x = ruler_x + (hue - HUE_LO) * RW // (HUE_HI - HUE_LO)
    label = str(hue)
    text(x - (len(label) - 1) // 2, 22, label, DIM)
text(ruler_x, 23, "├" + "─" * (RW - 1) + "┤", FAINT)
for k in range(1, 4):
    put(ruler_x + k * RW // 4, 23, "┼", FAINT)


def hue_bar(yy, label, lo, hi, note, col):
    text(RX + 2, yy, label, col)
    a = ruler_x + round((lo - HUE_LO) / (HUE_HI - HUE_LO) * RW)
    b = ruler_x + round((hi - HUE_LO) / (HUE_HI - HUE_LO) * RW)
    for x in range(a, max(a + 1, b)):
        hue = HUE_LO + (x - ruler_x) / RW * (HUE_HI - HUE_LO)
        r, g, bl = colorsys.hsv_to_rgb(hue / 360, 0.62, 0.78)
        put(x, yy, "█", (int(r * 255), int(g * 255), int(bl * 255)))
    text(ruler_x + RW + 3, yy, note.strip(), DIM)


hue_bar(24, "reference", 111, 114, "111-114", WHITE)
hue_bar(25, "was 11-23", 71, 83, " 71-83", RED)
hue_bar(26, "now 11-24", 86, 100, " 86-100", BRIGHT)
text(RX, 28, "SATURATION", HEAD)
for x0, label, n, val, col in ((RX + 12, "ref", 6, "0.6", WHITE), (RX + 30, "was", 7, "0.7*", RED), (RX + 48, "now", 6, "0.6", BRIGHT)):
    text(x0, 28, label, DIM)
    text(x0 + 4, 28, "▮" * n, col)
    text(x0 + 5 + n, 28, val, col)
text(RX + 2, 29, "* blue clipped to zero in the dark: chroma that wasn't", DIM)
text(RX, 31, "VALUE p10..p90", HEAD)
for k, (label, lo, hi, col) in enumerate((("ref", 0.17, 0.76, WHITE), ("was", 0.22, 0.39, RED), ("now", 0.19, 0.49, BRIGHT))):
    yy = 31 + k
    text(RX + 16, yy, label, DIM)
    a, b = RX + 20 + round(lo * 32), RX + 20 + round(hi * 32)
    text(RX + 20, yy, "·" * 32, FAINT)
    for x in range(a, b):
        put(x, yy, "▓" if a < x < b - 1 else "░", col)
    text(RX + 55, yy, f"{lo:.2f}-{hi:.2f}", DIM)
text(RX, 35, "CAUSES", HEAD)
causes = [
    ("sun  ", "0.92 .82 .63", "0.96 .93 .82", "yellow key"),
    ("haze ", "0.40 .42 .35", "0.33 .41 .38", "olive fog"),
    ("grade", "rgb contrast", "luma in sqrt", "sat x1.12"),
]
for k, (what, was, now, why) in enumerate(causes):
    yy = 36 + k
    text(RX + 2, yy, what, CYAN)
    text(RX + 8, yy, was, RED)
    text(RX + 22, yy, "->", DIM)
    text(RX + 25, yy, now, BRIGHT)
    text(RX + 40, yy, f"({why})", DIM)
text(RX, 40, "MINIMAP", HEAD)
text(RX + 8, 40, "// residual self image: the map as the shader sees it", DIM)
text(RX + 2, 41, "grass 129 203 18 197 195 205  soil 32  stone 10  water 1", MID)
text(RX + 2, 42, "ref", DIM)
text(RX + 6, 42, "████", (39, 48, 30))
text(RX + 11, 42, "(39,48,30)", DIM)
text(RX + 24, 42, "ours", DIM)
text(RX + 29, 42, "████", (41, 50, 26))
text(RX + 34, 42, "(41,50,26)", DIM)
text(RX + 47, 42, "was ████ grey", DIM)
text(RX + 51, 42, "████", (92, 88, 70))
text(RX + 2, 43, "veil follows ally sight in real time, soft edge, walls cast", MID)

# --- sysop's note ------------------------------------------------------------
header(47, elite("sysop's note // there is no spoon, only sprite 297"))
clear(4, 48, W - 5, 49)
center(48, "sTILL oPEN: lANE pICKER + bATTLE sPELLS, cLIFFS + bRIDGES + gOD sTATUES (tHE rEFERENCE jUNGLE iS wALLED), gRASS hUE 100 vS 111.", HEAD)
center(49, "stone.rs // hud/kit.rs // shop::layout // minimap::Palette // post.wgsl grade // place_tree // sketch::pieces_without", DIM)

# --- roadmap -------------------------------------------------------------------
header(51, elite("roadmap // docs/ROADMAPPLAN.md"))
clear(4, 52, W - 5, 56)
phases = [
    ("P1 LIGHT", 5), ("P2 MINIMAP", 6), ("P3 SHOP", 7), ("P4 SELECT", 4),
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
text(8, 56, ">> next: lane picker + battle spells (P4), cliffs/bridges/statues from cache objects (P5), cache floor colours (P1.3)", MID)

fade_bar(57, 2, W - 3)
clear(10, 58, W - 11, 61)
center(58, "gREETZ: sPRITE 297 (tHE sTONE uNDER eVERYTHING) * tEXTURE 32 (eVERY lANE) * tHE dIRE tREES (iN lEAF, aT lAST)", MID)
center(59, "nOT AFFILIATED WITH JAGEX. cACHE ASSETS STAY OUT OF GIT. aLWAYS.", DIM)
center(60, "  w e   c a n   l o a d   a n y t h i n g .   s t o n e ,   c o i n s ,   a   w h o l e   m a p .  ", HEAD)
center(61, "-=[ mObAsCaPe // 2026 // nODE 12: tHE cONSTRUCT // 57.6k // sYSoP: rIFT // 8 cOMMITS, 1 pAINTED mAP ]=-", AMBER)

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
