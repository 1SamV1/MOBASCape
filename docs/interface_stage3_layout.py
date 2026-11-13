"""The layout arithmetic behind docs/INTERFACE_STAGE3_PLAN.md.

Places every rectangle of the planned interface by its anchor and margins and
tests them against each other. It is a calculation for review, not part of
the game: at step B of the plan it is replaced by tests on the real placing
code. Run it with `python3 docs/interface_stage3_layout.py`.
"""
# The planned interface's rectangles, from anchors and margins. Units: the interface's own pixels, times the scale.
GAP = 8
INV = (144, 160)      # our six slots (proposal, from parts); the reference's panel is 232 by 224
OBJ = 290             # our own objectives list (proposal, from parts); the reference's is 350 to 400
CHAT_MIN = 280
def layout(W, H, s, st, slide):
    r = {}
    R = lambda x, y, w, h: (x, y, x + w, y + h)
    mm = R(W - (8 + 266) * s, H - (16 + 264) * s, 266 * s, 264 * s); r['minimap'] = mm
    if st['inv']: r['inventory'] = R(mm[0] - (6 + INV[0]) * s, H - (18 + INV[1]) * s, INV[0] * s, INV[1] * s)
    r['chat'] = R(5 * s, H - (10 + st['chat_h']) * s, st['chat_w'] * s, st['chat_h'] * s)
    ax = (W - 458 * s) // 2
    if slide:
        limit = (r['inventory'][0] if st['inv'] else mm[0]) - GAP * s
        ax = min(ax, limit - 458 * s)
    r['ability'] = R(ax, H - (18 + 94) * s, 458 * s, 94 * s)
    if st['score']: r['score'] = R((W - 534 * s) // 2, 10 * s, 534 * s, 38 * s)
    if st['feed']: r['feed'] = R(W - (12 + 286) * s, 18 * s, 286 * s, (30 * st['feed'] - 2) * s)
    tw = 60 if st['team_compact'] else 180
    r['team'] = R(W - (12 + tw) * s, 168 * s, tw * s, 188 * s)
    if st['target']: r['target'] = R(8 * s, 82 * s, 332 * s, 158 * s)
    oh = st['obj_h']; top = min(round(0.38 * H), r['chat'][1] - (GAP + oh) * s)
    r['objectives'] = R(8 * s, top, 272 * s, oh * s)
    return r
def inter(a, b): return max(min(a[2], b[2]) - max(a[0], b[0]), 0) * max(min(a[3], b[3]) - max(a[1], b[1]), 0)
def clashes(r, W, H, s):
    n = list(r); out = []
    for i in range(len(n)):
        a = r[n[i]]
        if a[0] < 0 or a[1] < 0 or a[2] > W or a[3] > H: out.append((n[i], 'window'))
        for j in range(i + 1, len(n)):
            b = r[n[j]]; g = GAP * s
            if inter((a[0] - g, a[1] - g, a[2] + g, a[3] + g), b):   # closer than the clearance counts as a clash
                if not ({n[i], n[j]} == {'inventory', 'minimap'}): out.append((n[i], n[j]))
    return out
def unique(rects, W, H):
    xs = sorted({v for a in rects for v in (a[0], a[2])}); ys = sorted({v for a in rects for v in (a[1], a[3])}); area = 0
    for i in range(len(xs) - 1):
        for j in range(len(ys) - 1):
            cx, cy = (xs[i] + xs[i + 1]) / 2, (ys[j] + ys[j + 1]) / 2
            if any(a[0] <= cx < a[2] and a[1] <= cy < a[3] for a in rects): area += (xs[i + 1] - xs[i]) * (ys[j + 1] - ys[j])
    return area / (W * H)
USUAL = dict(chat_w=440, chat_h=140, target=False, obj_h=OBJ, feed=1, inv=True, score=True, team_compact=False)
BUSY = dict(USUAL, target=True, feed=4)
def fits_full(W, H, s, budget):
    """The whole interface, bar centred, nothing given up: no clash even with everything showing, and the usual state within budget."""
    return (not clashes(layout(W, H, s, BUSY, False), W, H, s)
            and unique(list(layout(W, H, s, USUAL, False).values()), W, H) <= budget)
def choose_scale(W, H, budget):
    for s in (4, 3, 2):
        if fits_full(W, H, s, budget): return s
    return 1
def collapse(W, H, s, st, budget, slide):
    st = dict(st); steps = []
    lay = lambda: layout(W, H, s, st, slide)
    bad = lambda group: [p for p in clashes(lay(), W, H, s) if len(p) == 2 and p[1] != 'window' and p[0] in group and p[1] in group]
    cover = lambda: unique(list(layout(W, H, s, dict(st, target=False, feed=min(st['feed'], 1)), slide).values()), W, H)
    B, L, Rr = {'chat', 'ability', 'inventory', 'minimap'}, {'target', 'objectives', 'chat'}, {'feed', 'team', 'minimap', 'score'}
    if bad(B):
        for w in range(440, CHAT_MIN - 1, -2):
            st['chat_w'] = w
            if not [p for p in bad(B) if 'chat' in p]: break
        steps.append(f"chat narrows to {st['chat_w']}")
    if [p for p in bad(B) if 'chat' in p]: st['chat_h'] = 24; steps.append('chat to its last line')
    if bad(B): st['inv'] = False; steps.append('inventory hidden')
    if bad(L) and st['target']: st['target'] = False; steps.append('target panel not shown')
    if bad(L): st['obj_h'] = 24; steps.append('objectives to its title')
    if bad(Rr) and st['feed'] > 1: st['feed'] = 1; steps.append('kill feed to one entry')
    if bad(Rr): st['team_compact'] = True; steps.append('team frames to portraits')
    for name, do, can in [('objectives to its title', lambda: st.__setitem__('obj_h', 24), lambda: st['obj_h'] > 24),
                          ('team frames to portraits', lambda: st.__setitem__('team_compact', True), lambda: not st['team_compact']),
                          ('chat to its last line', lambda: st.__setitem__('chat_h', 24), lambda: st['chat_h'] > 24),
                          ('kill feed hidden', lambda: st.__setitem__('feed', 0), lambda: st['feed']),
                          ('score strip hidden', lambda: st.__setitem__('score', False), lambda: st['score']),
                          ('inventory hidden', lambda: st.__setitem__('inv', False), lambda: st['inv'])]:
        if cover() <= budget: break
        if can(): do(); steps.append(name + ' (budget)')
    left = [p for p in clashes(lay(), W, H, s)]
    return steps, st, left
if __name__ == '__main__':
    SIZES = [(1280, 720), (1366, 768), (1600, 900), (1920, 1080), (2560, 1440), (3200, 1800), (3840, 2160)]
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
