"""The layout arithmetic behind docs/INTERFACE_STAGE3_PLAN.md.

Places every rectangle of the planned interface by its anchor and margins and
tests them against each other. It is a calculation for review, not part of
the game: at step B of the plan it is replaced by tests on the real placing
code. Run it with `python3 docs/interface_stage3_layout.py`.
"""
# The planned interface's rectangles, from anchors and margins. Units: the interface's own pixels, times the scale.
GAP = 8
INV = (144, 160)      # our six slots (proposal, from parts); the reference's panel is 232 by 224
# Our own objectives list at the reference's own spacing. The reference in S2 is 350 tall: a title, the team
# names, a line of one word under each name, three lane lines, a rule, six objective lines and one second line,
# at 24 a line. Ours has the same title, names, lane lines, rule and six lines, no word line and no second line
# in its usual state: 350 less two lines.
OBJ_HEAD, OBJ_LINE = 158, 24            # title, team names, three lane lines, rule and edges; then 24 for every line
def obj_height(lines, second_lines): return OBJ_HEAD + OBJ_LINE * (lines + second_lines)
OBJ_LINES, OBJ_SECOND = 6, 4            # what the game has today: two nexus lines and four camps; a second line under each camp
OBJ = obj_height(OBJ_LINES, 0)          # 302, the usual list
OBJ_MAX = obj_height(OBJ_LINES, OBJ_SECOND)   # 398, the longest the game can show today. Not a permanent limit: it follows the two counts
CHAT_MIN = 280
CHAT_LINE_MIN = 200   # the chat's last line alone may be narrower still
def layout(W, H, s, st, slide):
    r = {}
    R = lambda x, y, w, h: (x, y, x + w, y + h)
    mm = R(W - (8 + 266) * s, H - (16 + 264) * s, 266 * s, 264 * s); r['minimap'] = mm
    if st['inv']: r['inventory'] = R(mm[0] - (GAP + INV[0]) * s, H - (18 + INV[1]) * s, INV[0] * s, INV[1] * s)
    r['chat'] = R(5 * s, H - (10 + st['chat_h']) * s, st['chat_w'] * s, st['chat_h'] * s)
    ax = (W - 458 * s) // 2
    if slide:
        limit = (r['inventory'][0] if st['inv'] else mm[0]) - GAP * s
        ax = min(ax, limit - 458 * s)
    r['ability'] = R(ax, H - (18 + 94) * s, 458 * s, 94 * s)
    if st['score']: r['score'] = R((W - 534 * s) // 2, 10 * s, 534 * s, 38 * s)
    if st['feed']: r['feed'] = R(W - (12 + 286) * s, st.get('feed_y', 18) * s, 286 * s, (30 * st['feed'] - 2) * s)
    tw = 60 if st['team_compact'] else 180
    if st.get('team', True): r['team'] = R(W - (12 + tw) * s, 168 * s, tw * s, 188 * s)
    if st['target']: r['target'] = R(8 * s, 82 * s, 332 * s, 158 * s)
    # The objectives. The target panel's place is kept for it always: whether the panel is showing or not, and
    # whether the objectives are a full list, their title alone, or not shown. The room the objectives have is
    # worked out first, from their neighbours (obj_room): from 8 below where the target panel ends, to 8 above
    # whatever is under them. Whatever is shown is placed inside it: at 38% of the height (in whole numbers, a
    # half going up), moved up if that is too low, and never above the room's top. Whether a thing fits the
    # room is asked before it is placed (collapse does the asking). Where something that does not fit is placed
    # anyway (the scale test does this, to see if it fits), it is held at the room's top and overruns downward,
    # where the clash test sees it.
    oh = st['obj_h']
    if oh:
        room_top, room_bottom = obj_room(r, s)
        top = max(room_top, min((38 * H + 50) // 100, room_bottom - oh * s))
        r['objectives'] = R(8 * s, top, 272 * s, oh * s)
    return r
def obj_room(r, s):
    """Top and bottom of the room the objectives may use, from the pieces already placed. The top is 8 below the
    target panel's place. The bottom is 8 above the chat, or above the ability bar if the bar has slid far
    enough left to come under the panel (within 8 of its right edge), whichever is higher."""
    under = [r['chat'][1]] + ([r['ability'][1]] if r['ability'][0] < (8 + 272 + GAP) * s else [])
    return (82 + 158 + GAP) * s, min(under) - GAP * s
def inter(a, b): return max(min(a[2], b[2]) - max(a[0], b[0]), 0) * max(min(a[3], b[3]) - max(a[1], b[1]), 0)
def clashes(r, W, H, s):
    n = list(r); out = []
    for i in range(len(n)):
        a = r[n[i]]
        if a[0] < 0 or a[1] < 0 or a[2] > W or a[3] > H: out.append((n[i], 'window'))
        for j in range(i + 1, len(n)):
            b = r[n[j]]; g = GAP * s
            # Rectangles hold their first pixel and stop before their last coordinate, so the space between two
            # is b's first less a's last. Less than the clearance is a clash; exactly the clearance is not.
            if inter((a[0] - g, a[1] - g, a[2] + g, a[3] + g), b):
                out.append((n[i], n[j]))
    return out
def unique(rects, W, H):
    xs = sorted({v for a in rects for v in (a[0], a[2])}); ys = sorted({v for a in rects for v in (a[1], a[3])}); area = 0
    for i in range(len(xs) - 1):
        for j in range(len(ys) - 1):
            cx, cy = (xs[i] + xs[i + 1]) / 2, (ys[j] + ys[j + 1]) / 2
            if any(a[0] <= cx < a[2] and a[1] <= cy < a[3] for a in rects): area += (xs[i + 1] - xs[i]) * (ys[j + 1] - ys[j])
    return area / (W * H)
USUAL = dict(chat_w=440, chat_h=140, target=False, obj_h=OBJ, feed=1, inv=True, score=True, team_compact=False)
BUSY = dict(USUAL, target=True, feed=4, obj_h=OBJ_MAX)
def fits_full(W, H, s, budget):
    """The whole interface, bar centred, nothing given up: no clash even with everything showing, and the usual state within budget."""
    return (not clashes(layout(W, H, s, BUSY, False), W, H, s)
            and unique(list(layout(W, H, s, USUAL, False).values()), W, H) <= budget)
def choose_scale(W, H, budget):
    for s in (4, 3, 2):
        if fits_full(W, H, s, budget): return s
    return 1
def holds(W, H, s, budget=0.25):
    """Can the window hold the layout at this scale at all: with everything given up that may be, is no clash left?
    Asked of the usual state, so it depends on the window alone; the pieces that come and go are only ever
    shown where they are clear, and cannot add a clash. Since the rules for clashes leave no clash by themselves,
    the answer does not depend on the budget either (output section 15 tries it with the budget off)."""
    return not collapse(W, H, s, USUAL, budget, True)[2]
def effective_scale(W, H, requested, budget=0.25, can_hold=holds):
    """The scale drawn. With nothing requested, the scale chosen by fit. With a scale requested, that scale if the
    window can hold it, else the largest lower scale it can hold. Nothing is remembered: the answer comes from the
    window and the request alone, so the requested scale is drawn again as soon as the window can hold it.
    Scale 1 is the floor. (can_hold is holds(); the sweep passes the same function with its answers remembered.)"""
    if requested is None: return choose_scale(W, H, budget)
    for s in range(requested, 1, -1):
        if can_hold(W, H, s, budget): return s
    return 1
def collapse(W, H, s, want, budget, slide):
    """Two passes. First the layout is settled for the usual state, from the window alone. Then what comes and
    goes with play (the target panel, more feed entries, a longer objectives list) is added where it fits."""
    st = dict(want, target=False, feed=min(want['feed'], 1), obj_h=min(want['obj_h'], OBJ)); steps = []
    lay = lambda: layout(W, H, s, st, slide)
    # a piece pushed off the window counts as trouble in its own row or column
    # A clash belongs to a row or column when every piece in it does (the window's edge belongs to all of them).
    # Both pieces, not either: the chat is in the bottom row and the left column, and a clash between the chat
    # and the objectives is the left column's to settle, not the bottom row's. The test does not depend on the
    # order the pieces are listed in.
    bad = lambda group: [p for p in clashes(lay(), W, H, s) if set(p) - {'window'} <= group]
    cover = lambda: unique(list(lay().values()), W, H)
    B, L, Rr = {'chat', 'ability', 'inventory', 'minimap'}, {'target', 'objectives', 'chat'}, {'feed', 'team', 'minimap', 'score'}
    # --- pass 1: the usual state
    # The bottom row is settled in one go. The minimap never moves; the inventory sits against it; the bar is
    # centred, or slid left only as far as leaves the clearance to the inventory (layout() does that); the chat
    # takes what is left. The arrangements are tried in this order and the first with no clash in the row is
    # kept, each with the widest chat that fits, so nothing is ever left smaller than the window requires.
    if bad(B):
        tries = [(inv, h, lo) for inv in (True, False) for h, lo in ((140, CHAT_MIN), (24, CHAT_LINE_MIN))]
        for inv, h, lo in tries:
            st['inv'], st['chat_h'] = inv, h
            for w in range(440, lo - 1, -1):
                st['chat_w'] = w
                if not bad(B): break
            if not bad(B): break
        if st['chat_w'] < 440: steps.append(f"chat narrows to {st['chat_w']}")
        if st['chat_h'] == 24: steps.append('chat to its last line')
        if not st['inv']: steps.append('inventory hidden')
    # The objectives are asked for only what their room can hold: the list, else the title, else nothing.
    def room(): top, bottom = obj_room(layout(W, H, s, dict(st, obj_h=0), slide), s); return bottom - top
    fits = lambda h: h * s <= room()
    if not fits(st['obj_h']): st['obj_h'] = 24; steps.append('objectives to its title')
    if not fits(st['obj_h']): st['obj_h'] = 0; steps.append('objectives hidden')
    if [p for p in bad(Rr) if set(p) == {'score', 'feed'}]: st['feed_y'] = 56; steps.append('kill feed moves under the score strip')
    # A piece of the top row still within 8 of the minimap is not shown: the kill feed first, then the score strip.
    # The minimap is never hidden, so in a window short enough for it to reach the top row it is the other that goes.
    # With this the rules for clashes leave no clash by themselves; nothing waits for the budget to separate two pieces.
    near_map = lambda name: [p for p in clashes(lay(), W, H, s) if set(p) == {name, 'minimap'}]
    if st['feed'] and near_map('feed'): st['feed'] = 0; steps.append('kill feed not shown: the minimap reaches it')
    if st['score'] and near_map('score'): st['score'] = False; steps.append('score strip not shown: the minimap reaches it')
    if bad(Rr): st['team_compact'] = True; steps.append('team frames to portraits')
    if bad(Rr): st['team'] = False; steps.append('team frames hidden')
    for name, do, can in [('objectives to its title', lambda: st.__setitem__('obj_h', 24), lambda: st['obj_h'] > 24),
                          ('team frames to portraits', lambda: st.__setitem__('team_compact', True), lambda: not st['team_compact']),
                          ('chat to its last line', lambda: st.__setitem__('chat_h', 24), lambda: st['chat_h'] > 24),
                          ('kill feed hidden', lambda: st.__setitem__('feed', 0), lambda: st['feed']),
                          ('score strip hidden', lambda: st.__setitem__('score', False), lambda: st['score']),
                          ('inventory hidden', lambda: st.__setitem__('inv', False), lambda: st['inv'])]:
        if cover() <= budget: break
        if can(): do(); steps.append(name + ' (budget)')
    # A budget step may have shortened the chat and made room. If the objectives were hidden for want of room,
    # their title comes back where it now fits and the budget allows, so nothing stays smaller than it must.
    if st['obj_h'] == 0:
        st['obj_h'] = 24
        if not fits(24) or cover() > budget: st['obj_h'] = 0
        else: steps[steps.index('objectives hidden')] = 'objectives hidden, then back to its title once the chat had shrunk'
    # --- pass 2: what comes and goes with play, each only where it fits
    # A conditional piece is shown only if it is clear of every other piece, whichever row or column that piece is in.
    touching = lambda name: [p for p in clashes(lay(), W, H, s) if name in p]
    if st['obj_h'] > 24 and want['obj_h'] > st['obj_h']:
        st['obj_h'] = want['obj_h']
        if not fits(st['obj_h']): st['obj_h'] = 24; steps.append('objectives, when long, to its title')
    if want['target']:
        st['target'] = True
        if touching('target'): st['target'] = False; steps.append('target panel not shown')
    if st['feed'] and want['feed'] > 1:
        for n in range(want['feed'], 0, -1):
            st['feed'] = n
            if not touching('feed'): break
        if st['feed'] < want['feed']: steps.append(f"kill feed to {st['feed']} entr{'y' if st['feed'] == 1 else 'ies'}")
    return steps, st, clashes(lay(), W, H, s)
def main():
    global OBJ, OBJ_MAX, USUAL, BUSY
    SIZES = [(1280, 720), (1366, 768), (1600, 900), (1920, 1080), (2560, 1440), (3200, 1800), (3840, 2160)]
    SHAPES = [(2560, 1080, 'wide, 21:9'), (3440, 1440, 'wide, 21:9'), (5120, 1440, 'very wide, 32:9'), (1920, 1200, '16:10'), (1600, 1200, '4:3'), (1280, 1024, '5:4'), (1024, 768, '4:3, under the smallest supported'), (1080, 1920, 'tall, 9:16'), (1440, 2560, 'tall, 9:16')]
    print('--- 1. rectangles at 1920x1080, scale 1, everything showing (compare with the measurements)')
    for k, v in layout(1920, 1080, 1, BUSY, False).items(): print(f'   {k:10} {v[0]:5} to {v[2]:5}, {v[1]:5} to {v[3]:5}')
    print('--- 2. least window at scale 1 for the whole interface, bar centred, 8 clear between pieces')
    print('   width :', next(W for W in range(900, 2400) if not clashes(layout(W, 1200, 1, BUSY, False), W, 1200, 1)))
    print('   height:', next(H for H in range(400, 1400) if not clashes(layout(2400, H, 1, BUSY, False), 2400, H, 1)))
    print('   bar centred and minimap alone, width:', next(W for W in range(600, 1400) if (lambda r: not inter((r['ability'][0], 0, r['ability'][2] + GAP, 9999), r['minimap']))(layout(W, 600, 1, dict(USUAL, inv=False), False))))
    print('   bar sliding and minimap alone, width:', 5 + 0 + 458 + GAP + 266 + 8)
    for budget in (0.25, 0.30):
        print(f'--- 3. budget {int(budget*100)}% of the window, measured on the usual state')
        for W, H in SIZES:
            s = choose_scale(W, H, budget)
            for slide in (False, True):
                row = []
                for label, st in (('usual', USUAL), ('busy', BUSY)):
                    steps, end, left = collapse(W, H, s, st, budget, slide)
                    r = layout(W, H, s, end, slide); rects = list(r.values())
                    ssum = sum((a[2] - a[0]) * (a[3] - a[1]) for a in rects) / (W * H)
                    row.append(f"{label}: {'; '.join(steps) or 'nothing gives'} | sum {100*ssum:.1f}% unique {100*unique(rects, W, H):.1f}%{' | STILL CLASHING ' + str(left) if left else ''}")
                print(f"   {W}x{H} scale {s} {'bar slides ' if slide else 'bar centred'}  " + '  ||  '.join(row))
    print('--- 4. where each scale first fits (16:9 windows)')
    for budget in (0.25, 0.30, 1.0):
        out = []
        for s in (1, 2, 3):
            W = next(W for W in range(1000, 8000, 2) if fits_full(W, W * 9 // 16, s, budget)); out.append(f'scale {s} from {W}x{W*9//16}')
        print('   budget', 'none' if budget == 1 else f'{int(budget*100)}%', ':', ', '.join(out))
    print('--- 5. other shapes of window, a quarter, bar may slide')
    for W, H, kind in SHAPES:
        s = choose_scale(W, H, 0.25); print(f'   {W}x{H} ({kind}) scale {s}')
        for label, st in (('usual', USUAL), ('busy', BUSY)):
            steps, end, left = collapse(W, H, s, st, 0.25, True)
            r = layout(W, H, s, end, True); o = r.get('objectives', (0, 0, 0, 0))
            print(f"      {label}: {'; '.join(steps) or 'nothing gives'} | covers {100*unique(list(r.values()), W, H):.1f}% | objectives {o[1]} to {o[3]}, chat from {r['chat'][1]}{' | STILL CLASHING ' + str(left) if left else ''}")
    print('--- 6. every window from 960x384 to 3840x2400, width and height varied independently: any clash left after collapse?')
    left = [(W, H) for W in range(960, 3841, 32) for H in range(384, 2401, 32)
            if any(collapse(W, H, choose_scale(W, H, 0.25), st, 0.25, True)[2] for st in (USUAL, BUSY))]
    print('   windows tried:', len(range(960, 3841, 32)) * len(range(384, 2401, 32)), '; left with a clash:', len(left), left[:6])
    clean = lambda W, H: not any(collapse(W, H, choose_scale(W, H, 0.25), st, 0.25, True)[2] for st in (USUAL, BUSY))
    print('   least width with no clash left, at 720 high:', next(W for W in range(700, 1400) if clean(W, 720)))
    print('   least height with no clash left, at 1280 wide:', next(H for H in range(200, 900) if clean(1280, H)))
    print('   these are two measurements, each with the other side large. The two together, 953x280: clash left?', not clean(953, 280),
          '; and every window from 953x280 to 1280x720 in steps of 1 across and 8 down: left with a clash:',
          sum(not clean(W, H) for W in range(953, 1281) for H in range(280, 721, 8)))
    print('--- 7. the scale never falls as a window grows (same sweep)')
    falls = 0
    for W in range(960, 3841, 32):
        for H in range(384, 2401, 32):
            s0 = choose_scale(W, H, 0.25)
            if choose_scale(W + 32, H, 0.25) < s0 or choose_scale(W, H + 32, 0.25) < s0: falls += 1
    print('   times the scale fell:', falls)
    print('--- 8. clearances, all with 8 clear, bar centred')
    print('   bar clears our inventory (144 wide) from width', next(W for W in range(900, 2400) if (W - 458) // 2 + 458 + GAP <= W - (8 + 266 + GAP + 144)))
    print('   bar clears the reference inventory (232 wide) from width', next(W for W in range(900, 2400) if (W - 458) // 2 + 458 + GAP <= W - (8 + 266 + GAP + 232)))
    print('   bar clears the chat at 440 from width', next(W for W in range(900, 2400) if 5 + 440 + GAP <= (W - 458) // 2))
    print('   bar clears the minimap alone from width', next(W for W in range(900, 2400) if (W - 458) // 2 + 458 + GAP <= W - (8 + 266)))
    print('--- 9. the same window gives the same rectangles whatever came before')
    # collapse() is given the window and what play wants to show and nothing else, so this can only fail if
    # something is carried over by mistake. One walk keeps going from size to size; each size it reaches is
    # then worked out again on its own, and the two sets of rectangles are compared.
    import random
    rng = random.Random(7); W, H = 1920, 1080; differ = 0; walk = []
    for _ in range(400):
        W = max(960, min(3840, W + rng.choice((-400, -64, -8, 8, 64, 400)))); H = max(384, min(2400, H + rng.choice((-300, -48, -6, 6, 48, 300))))
        want = rng.choice((USUAL, BUSY)); s = choose_scale(W, H, 0.25)
        walk.append((W, H, want, s, layout(W, H, s, collapse(W, H, s, want, 0.25, True)[1], True)))
    for W, H, want, s, seen in reversed(walk):
        s2 = choose_scale(W, H, 0.25)
        differ += (s2, layout(W, H, s2, collapse(W, H, s2, dict(want), 0.25, True)[1], True)) != (s, seen)
    print('   sizes reached by a wandering resize:', len(walk), '; differing when worked out again on their own, in another order:', differ)
    print('--- 10. the bar and the chat together at scale 1 (inventory showing): where the bar starts, how wide the chat is')
    for W in (1920, 1600, 1366, 1364, 1362, 1325, 1324, 1300, 1280, 1200, 1100, 1024, 960):
        steps, end, left = collapse(W, 720, 1, USUAL, 1.0, True); r = layout(W, 720, 1, end, True)
        inv = r.get('inventory'); a = r['ability']; c = r['chat']
        print(f"   {W:5}: bar {a[0]:4} to {a[2]:4} (centre would be {(W - 458) // 2:4}), moved {(W - 458) // 2 - a[0]:3}; chat {c[2] - c[0]:3} wide, {a[0] - c[2]:3} from the bar; "
              f"{'inventory ' + str(inv[0] - a[2]) + ' from the bar' if inv else 'inventory hidden'}{' | CLASH ' + str(left) if left else ''}")
    print('--- 11. why 5120x1440 stays at scale 1: at scale 2 the left column does not fit in the height')
    W, H, s = 5120, 1440, 2; r = layout(W, H, s, BUSY, False); g = GAP * s
    print(f"   target panel ends at {r['target'][3]}; the objectives may start no higher than {r['target'][3] + g}")
    print(f"   chat starts at {r['chat'][1]}; the objectives, {OBJ_MAX * s} tall, must start no lower than {r['chat'][1] - g - OBJ_MAX * s}")
    print(f"   short by {r['target'][3] + g - (r['chat'][1] - g - OBJ_MAX * s)}; clashes at scale 2: {clashes(r, W, H, s)}")
    print(f"   least height for scale 2 by clashes alone, any width: {next(h for h in range(1000, 2400) if not clashes(layout(6000, h, 2, BUSY, False), 6000, h, 2))}")
    print('--- 12. if the game gains objectives: each one more line, and (taking the worst) one more second line')
    print('   the heights, fit and scale all follow the counts; nothing here is a fixed 398')
    keep = (OBJ, OBJ_MAX, USUAL, BUSY)
    for extra in range(0, 9):
        OBJ, OBJ_MAX = obj_height(OBJ_LINES + extra, 0), obj_height(OBJ_LINES + extra, OBJ_SECOND + extra)
        USUAL = dict(keep[2], obj_h=OBJ); BUSY = dict(keep[3], obj_h=OBJ_MAX)
        least = next(H for H in range(400, 2400) if not clashes(layout(2400, H, 1, BUSY, False), 2400, H, 1))
        W1 = next(W for W in range(1000, 8000, 2) if fits_full(W, W * 9 // 16, 1, 0.25))
        row = []
        for W, H in ((1920, 1080), (2560, 1440), (3840, 2160)):
            sc = choose_scale(W, H, 0.25); steps, end, left = collapse(W, H, sc, BUSY, 0.25, True)
            o = layout(W, H, sc, end, True).get('objectives', (0, 0, 0, 0))
            how = ('longest in full, top ' + str(o[1] // sc) if end['obj_h'] == OBJ_MAX else
                   'usual in full, longest to its title' if end['obj_h'] == 24 and 'objectives, when long, to its title' in steps else 'title only')
            row.append(f"{W}x{H} scale {sc} {how}, target panel {'shown' if end['target'] else 'NOT SHOWN'}{' CLASH' if left else ''}")
        print(f"   +{extra}: usual {OBJ}, longest {OBJ_MAX}; least height by clashes {least}; all fits at scale 1 from {W1}x{W1 * 9 // 16}; " + '; '.join(row))
    OBJ, OBJ_MAX, USUAL, BUSY = keep
    print('--- 13. the target panel\'s place is never taken by the objectives: scale chosen, and scale forced')
    place = lambda s: (82 + 158 + GAP) * s
    def look(W, H, s):
        out = []
        for st in (USUAL, BUSY):
            steps, end, left = collapse(W, H, s, st, 0.25, True); r = layout(W, H, s, end, True); o = r.get('objectives')
            out.append((bool(o) and o[1] < place(s), bool(left), st['target'] and not end['target'], end['obj_h']))
        return out
    rows = {}
    for mode in ('chosen', 1, 2, 3, 4):
        n = took = clash = lost = 0; lost_at = []
        for W in range(960, 3841, 32):
            for H in range(384, 2401, 32):
                s = choose_scale(W, H, 0.25) if mode == 'chosen' else mode
                if mode != 'chosen' and (W < 953 * s or H < 280 * s): continue      # a forced scale the window cannot hold at all
                n += 1
                for a, b, c, d in look(W, H, s):
                    took += a; clash += b
                    if c: lost += 1; lost_at.append(H)
        print(f"   scale {mode if mode == 'chosen' else 'forced to ' + str(mode)}: {n} windows; objectives inside the target panel's place: {took}; "
              f"left with a clash: {clash}; target panel wanted and not shown: {lost}" + (f", all at heights of {min(lost_at)} to {max(lost_at)}" if lost_at else ''))
    print('   the cases that were wrong before this rule, and some like them:')
    for W, H, s in ((1920, 1080, 2), (2240, 1260, 2), (2560, 1440, 2), (3200, 1300, 2), (3840, 2160, 3), (3840, 2160, 4), (1280, 600, 1), (1280, 500, 1), (1280, 400, 1), (960, 384, 1)):
        steps, end, left = collapse(W, H, s, BUSY, 0.25, True); r = layout(W, H, s, end, True); o = r.get('objectives')
        what = 'not shown' if not o else ('title' if end['obj_h'] == 24 else 'list') + f' from {o[1]}'
        print(f"   {W}x{H} at scale {s}{' (forced)' if s != choose_scale(W, H, 0.25) else ''}: the place ends at {place(s)}; objectives {what}; "
              f"target panel {'shown ' + str(r['target'][1]) + ' to ' + str(r['target'][3]) if end['target'] else 'NOT SHOWN'}; chat from {r['chat'][1]}{'; CLASH ' + str(left) if left else ''}")
    print('--- 14. a requested scale the window cannot hold: the largest lower scale it can hold is drawn')
    seen, left = {}, {}
    def held(W, H, s, budget=0.25):     # holds(), with its answers remembered: the sweep asks about the same window for several requests
        if (W, H, s) not in seen: seen[(W, H, s)] = holds(W, H, s, budget)
        return seen[(W, H, s)]
    def clash_at(W, H, s):              # is a clash left at this scale, in the usual or the busy state; remembered likewise
        if (W, H, s) not in left: left[(W, H, s)] = any(collapse(W, H, s, st, 0.25, True)[2] for st in (USUAL, BUSY))
        return left[(W, H, s)]
    for req in (2, 3, 4):
        drawn = {(W, H): effective_scale(W, H, req, can_hold=held) for W in range(960, 3841 + 32, 32) for H in range(384, 2401 + 32, 32)}
        n = lowered = clash = above = fell = notrect = 0
        for W in range(960, 3841, 32):
            for H in range(384, 2401, 32):
                e = drawn[(W, H)]; n += 1
                lowered += e < req; above += e > req
                clash += clash_at(W, H, e)
                fell += drawn[(W + 32, H)] < e or drawn[(W, H + 32)] < e
                notrect += (e == req) != (W >= 953 * req and H >= 280 * req)
        print(f"   requested {req}: {n} windows; drawn at the request in {n - lowered}, lowered in {lowered}; ever above the request: {above}; "
              f"left with a clash: {clash}; times it fell as the window grew: {fell}; "
              f"windows where 'held' differs from 'at least {953 * req} wide and {280 * req} high': {notrect}")
    for W, H, req in ((1811, 1040, 2), (1905, 1040, 2), (1906, 1040, 2), (1920, 1080, 2), (1920, 559, 2), (1920, 560, 2), (2240, 1260, 3), (2859, 1260, 3), (3840, 2160, 4), (3840, 2160, 3), (3840, 2160, 1), (1920, 1080, 1)):
        e = effective_scale(W, H, req); a = choose_scale(W, H, 0.25)
        print(f"   {W}x{H}, {req} requested: drawn at {e}{'' if e == req else ' (cannot hold ' + str(req) + ': needs ' + str(953 * req) + ' by ' + str(280 * req) + ')'}; left to itself it would choose {a}")
    print('--- 15. the rules for clashes leave no clash by themselves: the budget switched off, windows no earlier sweep covered, and the edges')
    # Whether the minimap reaches the top row is written out here from the anchors, and not taken from layout() or
    # clashes(), so that what the rule does is checked against separate arithmetic.
    def near(a, b, g): return max(b[0] - a[2], a[0] - b[2]) < g and max(b[1] - a[3], a[1] - b[3]) < g   # two rectangles closer than g, across and down
    def strip(W, s): return ((W - 534 * s) // 2, 10 * s, (W - 534 * s) // 2 + 534 * s, 48 * s)
    def moved(W, s): return near(strip(W, s), (W - 298 * s, 18 * s, W - 12 * s, 46 * s), GAP * s)       # the feed goes under the score strip if the two meet
    def reached(W, H, s):
        mm = (W - 274 * s, H - 280 * s, W - 8 * s, H - 16 * s); y = 56 if moved(W, s) else 18
        return near((W - 298 * s, y * s, W - 12 * s, (y + 28) * s), mm, GAP * s), near(strip(W, s), mm, GAP * s)   # (the kill feed, the score strip)
    feed_rule, score_rule = 'kill feed not shown: the minimap reaches it', 'score strip not shown: the minimap reaches it'
    # With the budget at 100% no budget step is ever taken, so whatever is left is what the rules for clashes leave.
    wide = sorted(set(list(range(953, 1300, 9)) + list(range(1300, 3841, 96)) + [4480, 5120, 6400, 7680]))
    high = sorted(set(list(range(280, 760, 8)) + list(range(760, 2401, 82))))
    for s in (1, 2, 3, 4):
        n = bad = gone_f = gone_s = causeless = kept = 0
        for W in wide:
            for H in high:
                n += 1; goes = reached(W * s, H * s, s); clash = False
                for st in (USUAL, BUSY):
                    steps, end, left = collapse(W * s, H * s, s, st, 1.0, True); clash = clash or bool(left)
                    gone = (end['feed'] == 0, not end['score'])        # with the budget off, only the rule can have taken them away
                    gone_f += gone[0]; gone_s += gone[1]
                    causeless += (gone[0] and not goes[0]) + (gone[1] and not goes[1])
                    kept += (goes[0] and not gone[0]) + (goes[1] and not gone[1])
                bad += clash
        print(f'   scale {s}, budget off, both states: {n} windows from {wide[0] * s} by {high[0] * s} to {wide[-1] * s} by {high[-1] * s}; left with a clash: {bad}; '
              f'kill feed not shown in {gone_f} cases, score strip in {gone_s}; not shown though not within 8 of the minimap: {causeless}; within 8 and still shown: {kept}')
    for s in (1, 2, 3, 4):
        n = bad_q = bad_off = misused = 0
        for W in list(range(960, 3841, 32)) + [4480, 5120, 6400, 7680]:
            for H in range(280, 384, 4):
                n += 1; goes = reached(W * s, H * s, s); cq = co = False
                for st in (USUAL, BUSY):
                    steps, end, left = collapse(W * s, H * s, s, st, 0.25, True); cq = cq or bool(left)
                    misused += (feed_rule in steps and not goes[0]) + (score_rule in steps and not goes[1])
                    co = co or bool(collapse(W * s, H * s, s, st, 1.0, True)[2])
                bad_q += cq; bad_off += co
        print(f'   scale {s}, wide and short, both states: {n} windows, {960 * s} to {7680 * s} wide and {280 * s} to {380 * s} high; '
              f'left with a clash, budget a quarter: {bad_q}; budget off: {bad_off}; the rule used on a piece not within 8 of the minimap: {misused}')
    tried = wrong = 0
    for s in (1, 2, 3, 4):
        for W in (953, 1000, 1144, 1146, 1280, 1920, 2105, 2400, 3840, 7680):
            edge = ((84 if moved(W * s, s) else 46) + GAP + 280) * s        # the least height at which the feed is 8 clear of the minimap: 334 or 372, times the scale
            for b in (0.25, 1.0):
                for st in (USUAL, BUSY):
                    under = collapse(W * s, edge - 1, s, st, b, True); at = collapse(W * s, edge, s, st, b, True); r = layout(W * s, edge, s, at[1], True)
                    tried += 2
                    wrong += bool(under[2]) or under[1]['feed'] > 0                                                           # one pixel under: not shown, nothing clashing
                    wrong += bool(at[2]) or (b == 1.0 and not ('feed' in r and r['minimap'][1] - r['feed'][3] == GAP * s))    # at it: nothing clashing; budget off, shown and exactly 8 clear
    print(f"   the kill feed at its edge, scales 1 to 4, both states, budget a quarter and off: one pixel under 334 high (372 where it has moved under the "
          f"score strip), times the scale, it is not shown; at that height, with the budget off, it is shown exactly 8 clear of the minimap; nothing clashes "
          f"either side: {tried} checks; wrong: {wrong}")
    tried = wrong = 0
    for s in (1, 2, 3, 4):
        for W in (953, 1000, 1096, 1098, 1280, 3840):
            beside = reached(W * s, 336 * s - 1, s)[1]                     # is the strip beside the minimap at this width: true under about 1098 times the scale
            for st in (USUAL, BUSY):
                under = collapse(W * s, 336 * s - 1, s, st, 1.0, True); at = collapse(W * s, 336 * s, s, st, 1.0, True)
                tried += 2; wrong += bool(under[2]) or (under[1]['score'] == beside); wrong += bool(at[2]) or not at[1]['score']
    print(f"   the score strip at its edge, scales 1 to 4, both states, budget off: one pixel under 336 high, times the scale, it is not shown where it is "
          f"beside the minimap (953, 1000 and 1096 wide) and is shown where it is not (1098, 1280 and 3840 wide); at 336 it is shown at every width: {tried} checks; wrong: {wrong}")
    tried = wrong = 0
    for s in (1, 2, 3, 4):
        for b in (0.25, 1.0):
            for H in high: tried += 2; wrong += holds(953 * s - 1, H * s, s, b) + (not holds(953 * s, H * s, s, b))
            for W in wide: tried += 2; wrong += holds(W * s, 280 * s - 1, s, b) + (not holds(W * s, 280 * s, s, b))
            for k in (2, 5, 50, 200):
                for H in (280, 334, 720, 2400): tried += 1; wrong += holds(953 * s - k, H * s, s, b)
                for W in (953, 1280, 3840, 7680): tried += 1; wrong += holds(W * s, 280 * s - k, s, b)
    print(f"   'can hold' at its edge, scales 1 to 4, budget a quarter and off: exactly 953 wide holds and one pixel under does not, at each of {len(high)} heights; "
          f"exactly 280 high holds and one pixel under does not, at each of {len(wide)} widths; 2, 5, 50 and 200 under either do not hold (all times the scale): {tried} checks; wrong: {wrong}")
    print('   windows that were left with a clash before the rule, the height where that stops, and two that only the budget was saving (scale 1, usual state, a quarter):')
    for W, H in ((2105, 333), (2112, 332), (3840, 300), (7680, 280), (2400, 333), (2400, 334), (1000, 300), (1097, 335)):
        steps, end, left = collapse(W, H, 1, USUAL, 0.25, True); r = layout(W, H, 1, end, True)
        print(f"   {W}x{H}: kill feed {'shown ' + str(r['feed'][1]) + ' to ' + str(r['feed'][3]) if 'feed' in r else 'not shown'}; "
              f"score strip {'shown' if 'score' in r else 'not shown'}; minimap from {r['minimap'][1]}; clash left: {left or 'none'}")
if __name__ == '__main__': main()
