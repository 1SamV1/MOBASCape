//! Stage 1 of "Getting about on rough ground": the height data on its own
//! terms, and against the walkable map that exists.

use rift_sim::Grid;
use rift_sim::heights::{HeightMap, Q, Reading, agreement, compare};

type Tri = [[f32; 3]; 3];

/// A closed box as the game would hold it: built in a model's own space,
/// where up is down, with every face wound to point outward, and then drawn
/// mirrored. `x`, `z` are its middle; it runs from `bottom` to `top`.
fn solid_box(x: f32, z: f32, half_x: f32, half_z: f32, bottom: f32, top: f32) -> Vec<Tri> {
    // in drawn space: a right-handed outward box, then every triangle turned
    // inside out, which is what drawing a model mirrored does to it
    let p = |sx: f32, y: f32, sz: f32| [x + sx * half_x, y, z + sz * half_z];
    let quad = |a: [f32; 3], b: [f32; 3], c: [f32; 3], d: [f32; 3]| vec![[a, c, b], [a, d, c]];
    let mut out = Vec::new();
    // lid (faces up) and base (faces down)
    out.extend(quad(
        p(-1., top, -1.),
        p(-1., top, 1.),
        p(1., top, 1.),
        p(1., top, -1.),
    ));
    out.extend(quad(
        p(-1., bottom, -1.),
        p(1., bottom, -1.),
        p(1., bottom, 1.),
        p(-1., bottom, 1.),
    ));
    // four walls
    out.extend(quad(
        p(-1., bottom, -1.),
        p(-1., top, -1.),
        p(1., top, -1.),
        p(1., bottom, -1.),
    ));
    out.extend(quad(
        p(1., bottom, 1.),
        p(1., top, 1.),
        p(-1., top, 1.),
        p(-1., bottom, 1.),
    ));
    out.extend(quad(
        p(-1., bottom, 1.),
        p(-1., top, 1.),
        p(-1., top, -1.),
        p(-1., bottom, -1.),
    ));
    out.extend(quad(
        p(1., bottom, -1.),
        p(1., top, -1.),
        p(1., top, 1.),
        p(1., bottom, 1.),
    ));
    out
}

const GAME: Reading = Reading {
    mirrored: true,
    standing_angle: 45.0,
    head: Some(160.0),
};
const WHOLE: Reading = Reading {
    mirrored: true,
    standing_angle: 45.0,
    head: None,
};

fn read(shape: &[Tri], reading: &Reading) -> HeightMap {
    let mut map = HeightMap::new(32, 32, 32);
    map.add_shape(shape, (512.0, 512.0), reading);
    map
}

/// The cell holding a point given from the shape's own middle.
fn cell(x: i32, z: i32) -> (i32, i32) {
    ((512 + x).div_euclid(32), (512 + z).div_euclid(32))
}

#[test]
fn a_closed_box_has_its_lid_and_only_its_lid_to_stand_on() {
    let shape = solid_box(0.0, 0.0, 80.0, 80.0, 0.0, 100.0);
    let map = read(&shape, &WHOLE);
    let (cx, cy) = cell(0, 0);
    let middle = map
        .column(cx, cy)
        .expect("the box stands over its own middle");
    assert_eq!(
        middle.surfaces,
        vec![100 * Q],
        "one place to stand: the lid"
    );
    assert_eq!(
        middle.solid,
        vec![(0, 100 * Q)],
        "solid from the ground to the lid"
    );
    for (_, column) in map.cells() {
        assert!(
            column.surfaces.iter().all(|&s| s == 100 * Q),
            "no wall or base is somewhere to stand: {column:?}"
        );
    }
}

#[test]
fn the_order_the_triangles_come_in_changes_nothing() {
    let shape = solid_box(10.0, -20.0, 70.0, 50.0, 0.0, 90.0);
    let mut backwards = shape.clone();
    backwards.reverse();
    let mut shuffled = shape.clone();
    shuffled.rotate_left(5);
    shuffled.swap(0, 7);
    let (a, b, c) = (
        read(&shape, &WHOLE),
        read(&backwards, &WHOLE),
        read(&shuffled, &WHOLE),
    );
    let all = |m: &HeightMap| {
        let mut v: Vec<_> = m.cells().map(|(k, col)| (*k, col.clone())).collect();
        v.sort_by_key(|(k, _)| *k);
        v
    };
    assert_eq!(all(&a), all(&b));
    assert_eq!(all(&a), all(&c));
}

#[test]
fn reading_it_inside_out_would_stand_on_its_base_instead() {
    // the check that "mirrored" means something: read the wrong way round,
    // the lid becomes an underside
    let shape = solid_box(0.0, 0.0, 80.0, 80.0, 0.0, 100.0);
    let wrong = Reading {
        mirrored: false,
        ..WHOLE
    };
    let map = read(&shape, &wrong);
    let (cx, cy) = cell(0, 0);
    let middle = map.column(cx, cy).unwrap();
    assert!(
        !middle.surfaces.contains(&(100 * Q)),
        "the lid is no longer a top: {middle:?}"
    );
}

#[test]
fn a_ledge_clear_air_and_a_branch_above_keep_the_air_between_them() {
    // a thin ledge from 40 to 50 with a branch from 120 to 130 over it
    let mut shape = solid_box(0.0, 0.0, 60.0, 60.0, 40.0, 50.0);
    shape.extend(solid_box(0.0, 0.0, 60.0, 60.0, 120.0, 130.0));
    let map = read(&shape, &WHOLE);
    let (cx, cy) = cell(0, 0);
    let column = map.column(cx, cy).unwrap();
    assert_eq!(
        column.solid,
        vec![(40 * Q, 50 * Q), (120 * Q, 130 * Q)],
        "two stretches, a gap between"
    );
    assert_eq!(
        column.surfaces,
        vec![50 * Q, 130 * Q],
        "the top of each is somewhere to stand"
    );
    // a short body fits on the ledge under the branch; a tall one does not
    assert!(
        !column.in_the_way(50 * Q, 60 * Q, 48 * Q),
        "60 tall fits in a gap of 70"
    );
    assert!(
        column.in_the_way(50 * Q, 160 * Q, 48 * Q),
        "160 tall does not"
    );
}

#[test]
fn a_rock_is_solid_to_the_ground_and_a_ledge_only_as_thick_as_it_is() {
    // a rock: sides and a top, no base, sitting on the ground
    let rock: Vec<Tri> = solid_box(0.0, 0.0, 80.0, 80.0, 0.0, 100.0)
        .into_iter()
        .enumerate()
        .filter(|(i, _)| !(2..4).contains(i))
        .map(|(_, t)| t)
        .collect();
    let map = read(&rock, &WHOLE);
    let (cx, cy) = cell(0, 0);
    assert_eq!(map.column(cx, cy).unwrap().solid, vec![(0, 100 * Q)]);

    let ledge = solid_box(0.0, 0.0, 80.0, 80.0, 90.0, 100.0);
    let map = read(&ledge, &WHOLE);
    assert_eq!(map.column(cx, cy).unwrap().solid, vec![(90 * Q, 100 * Q)]);
}

#[test]
fn a_trunk_has_nowhere_to_stand_and_leaves_overhead_are_not_read() {
    let mut tree = solid_box(0.0, 0.0, 12.0, 12.0, 0.0, 400.0);
    tree.extend(solid_box(0.0, 0.0, 200.0, 200.0, 300.0, 380.0));
    let map = read(&tree, &GAME);
    let (tx, ty) = cell(0, 0);
    let trunk = map.column(tx, ty).expect("the trunk is read");
    assert!(
        trunk.surfaces.is_empty(),
        "nothing on a trunk to stand on below head height"
    );
    assert!(
        !trunk.open_from_the_ground(48 * Q, 160 * Q, 48 * Q),
        "and it is in the way"
    );
    let (lx, ly) = cell(150, 150);
    assert!(
        map.column(lx, ly).is_none(),
        "under the leaves there is nothing"
    );
}

#[test]
fn a_low_stone_is_stepped_onto_and_a_tall_one_is_not() {
    let low = read(&solid_box(0.0, 0.0, 40.0, 40.0, 0.0, 40.0), &GAME);
    let tall = read(&solid_box(0.0, 0.0, 40.0, 40.0, 0.0, 90.0), &GAME);
    for dx in [-36, 0, 36] {
        let (cx, cy) = cell(dx, 0);
        assert!(
            low.column(cx, cy)
                .unwrap()
                .open_from_the_ground(48 * Q, 160 * Q, 48 * Q),
            "a stone 40 high is no obstacle to a step of 48, middle or edge"
        );
    }
    let (cx, cy) = cell(0, 0);
    assert!(
        !tall
            .column(cx, cy)
            .unwrap()
            .open_from_the_ground(48 * Q, 160 * Q, 48 * Q)
    );
    // and with a bigger step, the same stone can be got onto
    assert!(
        tall.column(cx, cy)
            .unwrap()
            .open_from_the_ground(96 * Q, 160 * Q, 48 * Q)
    );
}

#[test]
fn a_steep_face_is_a_side_and_a_gentle_one_a_top() {
    // one sloping slab, as its top face only, drawn-space outward then turned
    // inside out as the game's models are
    let slab = |rise: f32| -> Vec<Tri> {
        let (a, b, c, d) = (
            [-60.0, 10.0, -60.0],
            [-60.0, 10.0, 60.0],
            [60.0, 10.0 + rise, 60.0],
            [60.0, 10.0 + rise, -60.0],
        );
        vec![[a, c, b], [a, d, c]]
    };
    let gentle = read(&slab(60.0), &WHOLE); // rises 60 in 120: about 27 degrees
    let steep = read(&slab(240.0), &WHOLE); // rises 240 in 120: about 63 degrees
    let (cx, cy) = cell(0, 0);
    assert!(
        !gentle.column(cx, cy).unwrap().surfaces.is_empty(),
        "gentle: somewhere to stand"
    );
    assert!(
        steep.column(cx, cy).unwrap().surfaces.is_empty(),
        "steep: nowhere to stand"
    );
    assert!(
        !steep.column(cx, cy).unwrap().solid.is_empty(),
        "but still in the way"
    );
}

/// One flat sheet with a single side, as leaves are modelled: `up` says which
/// way it faces once drawn.
fn sheet(height: f32, up: bool) -> Vec<Tri> {
    let (a, b, c, d) = (
        [-70.0, height, -70.0],
        [-70.0, height + 12.0, 70.0],
        [70.0, height + 12.0, 70.0],
        [70.0, height, -70.0],
    );
    // the same winding as a box's lid faces up; reversed, it faces down
    if up {
        vec![[a, c, b], [a, d, c]]
    } else {
        vec![[a, b, c], [a, c, d]]
    }
}

/// A palm as the game's are built: a trunk, leaves overhead, and single-sided
/// fronds hanging low enough to brush the ground, seen from underneath.
fn palm() -> Vec<Tri> {
    let mut tree = solid_box(0.0, 0.0, 12.0, 12.0, 0.0, 400.0);
    tree.extend(solid_box(0.0, 0.0, 180.0, 180.0, 300.0, 380.0));
    for (x, z) in [(90.0f32, 0.0f32), (-90.0, 20.0), (0.0, 100.0)] {
        let low: Vec<Tri> = sheet(20.0, false)
            .into_iter()
            .map(|t| t.map(|p| [p[0] * 0.5 + x, p[1], p[2] * 0.5 + z]))
            .collect();
        tree.extend(low);
    }
    tree
}

/// The promise of stage 1: read with today's step and head height, the height
/// data blocks the same cells as the walkable map that exists.
/// One of every kind of shape the two readings have to agree about.
fn every_shape() -> Vec<(&'static str, Vec<Tri>)> {
    let mut tree = solid_box(0.0, 0.0, 12.0, 12.0, 0.0, 400.0);
    tree.extend(solid_box(0.0, 0.0, 200.0, 200.0, 300.0, 380.0));
    let mut tower = solid_box(0.0, 0.0, 150.0, 130.0, 0.0, 138.0);
    tower.extend(solid_box(0.0, 0.0, 60.0, 60.0, 98.0, 432.0));
    for (dx, dz, h) in [
        (-200.0, 110.0, 26.0),
        (190.0, -60.0, 46.0),
        (120.0, 150.0, 46.0),
    ] {
        tower.extend(solid_box(dx, dz, 30.0, 30.0, 0.0, h));
    }
    let mut arch = solid_box(-150.0, 0.0, 30.0, 30.0, 0.0, 400.0);
    arch.extend(solid_box(150.0, 0.0, 30.0, 30.0, 0.0, 400.0));
    arch.extend(solid_box(0.0, 0.0, 180.0, 30.0, 300.0, 400.0));
    let mut overhang = solid_box(0.0, 0.0, 30.0, 30.0, 0.0, 300.0);
    overhang.extend(solid_box(100.0, 0.0, 100.0, 40.0, 100.0, 300.0));
    vec![
        ("low stone", solid_box(0.0, 0.0, 40.0, 40.0, 0.0, 40.0)),
        ("tall stone", solid_box(0.0, 0.0, 40.0, 40.0, 0.0, 90.0)),
        (
            "off-centre rock",
            solid_box(150.0, -90.0, 30.0, 30.0, 0.0, 200.0),
        ),
        ("tree", tree),
        ("tower with stones", tower),
        (
            "hollow tower",
            solid_box(0.0, 0.0, 110.0, 110.0, 0.0, 500.0),
        ),
        ("arch", arch),
        ("overhang", overhang),
        (
            "ledge in the air",
            solid_box(0.0, 0.0, 80.0, 80.0, 90.0, 100.0),
        ),
        ("palm with drooping fronds", palm()),
        ("sheet seen from below, low", sheet(30.0, false)),
        (
            "sheet seen from below, at chest height",
            sheet(100.0, false),
        ),
        ("sheet seen from above, low", sheet(30.0, true)),
        // one low stone with a taller one leaning over it: the low one must
        // not lift a walker past the tall one
        ("two stones, one on the other", {
            let mut stones = solid_box(0.0, 0.0, 40.0, 40.0, 0.0, 34.0);
            stones.extend(solid_box(20.0, 10.0, 40.0, 40.0, 39.0, 75.0));
            stones
        }),
        // the map stops a walker at 48 exactly, and lets it over anything less
        (
            "stone exactly at step height",
            solid_box(0.0, 0.0, 50.0, 50.0, 0.0, 48.0),
        ),
        (
            "stone a hair under step height",
            solid_box(0.0, 0.0, 50.0, 50.0, 0.0, 47.9),
        ),
    ]
}

#[test]
fn it_blocks_what_the_walkable_map_blocks_for_every_kind_of_shape() {
    let shapes = every_shape();
    for (name, shape) in &shapes {
        for body in [0, 28] {
            // wherever in a cell the shape happens to stand
            for at in [
                (0.0, 0.0),
                (16.0, 16.0),
                (7.5, 29.0),
                (31.9, 0.1),
                (2848.3, 3098.7),
                (-13.0, 21.0),
            ] {
                let found = agreement(shape, at, true, 45.0, 48.0, 160.0, body, 32);
                assert!(found.agrees(), "{name}, body {body}, at {at:?}: {found:?}");
            }
        }
    }
    // and the comparison is not empty-handed: a tower blocks a good many cells
    let found = agreement(&shapes[4].1, (0.0, 0.0), true, 45.0, 48.0, 160.0, 28, 32);
    assert!(found.both > 60, "{found:?}");
}

#[test]
fn the_finishing_step_is_the_same_whichever_way_the_cells_were_found() {
    // block_cells on the cells block_solid finds gives block_solid's result
    let shape = solid_box(0.0, 0.0, 110.0, 110.0, 0.0, 500.0);
    let mut whole = Grid::with_cell(32, 32, 32);
    whole.block_solid(&shape, (512.0, 512.0), 48.0, 160.0, 28);
    let mut bare = Grid::with_cell(32, 32, 32);
    let counted = bare.block_solid(&shape, (512.0, 512.0), 48.0, 160.0, 0);
    let cells: Vec<(i32, i32)> = (0..32)
        .flat_map(|y| (0..32).map(move |x| (x, y)))
        .filter(|&(x, y)| bare.is_blocked(x, y))
        .collect();
    assert_eq!(
        counted,
        cells.len(),
        "the count it gives back is every cell the thing covers, its inside included"
    );
    let mut again = Grid::with_cell(32, 32, 32);
    again.block_cells(&cells, 28);
    for y in 0..32 {
        for x in 0..32 {
            assert_eq!(whole.is_blocked(x, y), again.is_blocked(x, y), "{x},{y}");
        }
    }
}

#[test]
fn a_low_stone_is_not_a_level_and_gives_no_help_up() {
    // a stone 40 high beside a ledge 130 high
    let mut shape = solid_box(-64.0, 0.0, 32.0, 48.0, 0.0, 40.0);
    shape.extend(solid_box(48.0, 0.0, 80.0, 48.0, 0.0, 130.0));
    let map = read(&shape, &WHOLE);
    let (stone, ledge) = (cell(-64, 0), cell(48, 0));
    let (tall, body) = (160 * Q, 0);

    // anyone walks over the stone, and is still at the level of the ground
    for step in [48 * Q - 1, 96 * Q] {
        assert_eq!(map.step_to(0, stone, step, tall, body, 48 * Q), Some(0));
    }
    assert_eq!(
        map.column(stone.0, stone.1).unwrap().levels(48 * Q),
        vec![0],
        "the stone's top is not a level"
    );
    // so the ledge is 130 up from the stone, not 90, and a step of 96 does
    // not reach it from there any more than from the ground
    assert_eq!(map.step_to(0, ledge, 96 * Q, tall, body, 48 * Q), None);
    // from the ledge, down to the stone's cell is down to the ground: 130
    assert_eq!(
        map.step_to(130 * Q, stone, 96 * Q, tall, body, 48 * Q),
        None
    );
}

#[test]
fn real_levels_are_climbed_one_from_the_next() {
    // a block 80 high beside one 160 high: each a real standing surface
    let mut shape = solid_box(-70.0, 0.0, 34.0, 48.0, 0.0, 80.0);
    shape.extend(solid_box(48.0, 0.0, 80.0, 48.0, 0.0, 160.0));
    let map = read(&shape, &WHOLE);
    let (low, high) = (cell(-70, 0), cell(48, 0));
    let (tall, body) = (160 * Q, 0);
    // a step of 96: the ground to the low block, the low block to the high
    // one, but not the ground to the high one in one
    assert_eq!(
        map.step_to(0, low, 96 * Q, tall, body, 48 * Q),
        Some(80 * Q)
    );
    assert_eq!(
        map.step_to(80 * Q, high, 96 * Q, tall, body, 48 * Q),
        Some(160 * Q)
    );
    assert_eq!(map.step_to(0, high, 96 * Q, tall, body, 48 * Q), None);
    // and down again the same way
    assert_eq!(
        map.step_to(160 * Q, low, 96 * Q, tall, body, 48 * Q),
        Some(80 * Q)
    );
    // a step under 48 gets onto neither
    assert_eq!(map.step_to(0, low, 48 * Q - 1, tall, body, 48 * Q), None);
}

#[test]
fn a_surface_at_exactly_48_is_a_level_a_step_under_48_cannot_reach() {
    let at = read(&solid_box(0.0, 0.0, 50.0, 50.0, 0.0, 48.0), &WHOLE);
    let under = read(&solid_box(0.0, 0.0, 50.0, 50.0, 0.0, 47.9), &WHOLE);
    let here = cell(0, 0);
    let (tall, kerb) = (160 * Q, 48 * Q);
    assert_eq!(
        at.column(here.0, here.1).unwrap().levels(kerb),
        vec![0, 48 * Q]
    );
    assert_eq!(at.step_to(0, here, kerb - 1, tall, 0, kerb), None);
    assert_eq!(at.step_to(0, here, kerb, tall, 0, kerb), Some(48 * Q));
    // a hair lower and it is walked over at the level of the ground
    assert_eq!(under.column(here.0, here.1).unwrap().levels(kerb), vec![0]);
    assert_eq!(under.step_to(0, here, kerb - 1, tall, 0, kerb), Some(0));
}

#[test]
fn room_for_a_body_is_the_cells_at_its_sides_and_not_at_its_corners() {
    // one thin post, wholly inside one cell
    let map = read(&solid_box(16.0, 16.0, 6.0, 6.0, 0.0, 200.0), &WHOLE);
    let post = cell(16, 16);
    let (step, tall, kerb) = (48 * Q - 1, 160 * Q, 48 * Q);
    let from_post = |dx: i32, dy: i32, body: i32| {
        map.step_to(0, (post.0 + dx, post.1 + dy), step, tall, body, kerb)
    };
    // the room itself: a body of 28 on cells of 32 is kept from four cells
    let mut room = map.room(28);
    room.sort_unstable();
    assert_eq!(room, vec![(-1, 0), (0, -1), (0, 0), (0, 1), (1, 0)]);
    assert_eq!(map.room(0), vec![(0, 0)], "a point is kept from nothing");

    assert_eq!(from_post(0, 0, 28), None, "not in the post's own cell");
    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        assert_eq!(from_post(dx, dy, 28), None, "not beside it: {dx},{dy}");
        assert_eq!(from_post(dx, dy, 0), Some(0), "a point is: {dx},{dy}");
    }
    for (dx, dy) in [(1, 1), (-1, 1), (1, -1), (-1, -1)] {
        assert_eq!(from_post(dx, dy, 28), Some(0), "at its corner: {dx},{dy}");
    }
    for (dx, dy) in [(2, 0), (-2, 0), (0, 2), (0, -2)] {
        assert_eq!(from_post(dx, dy, 28), Some(0), "two cells off: {dx},{dy}");
    }
    // the edge of it: the next cell's middle is 32 away, so a body whose
    // half-width and half a cell come to 32 is just kept from it, and one a
    // unit narrower is just not. At the corner, 45.25 away: 30 is kept
    // from it (46), 29 is not (45).
    assert_eq!(from_post(1, 0, 16), None, "16 + 16 reaches 32");
    assert_eq!(from_post(1, 0, 15), Some(0), "15 + 16 does not");
    assert_eq!(from_post(1, 1, 30), None, "30 + 16 reaches the corner cell");
    assert_eq!(from_post(1, 1, 29), Some(0), "29 + 16 does not");
}

#[test]
fn low_stones_in_a_row_do_not_add_up() {
    // stones 40, 80 and 120 high in a row, each 40 above the last, too
    // narrow to be read as anything but stones standing on the ground
    let mut shape = solid_box(-64.0, 0.0, 30.0, 30.0, 0.0, 40.0);
    shape.extend(solid_box(0.0, 0.0, 30.0, 30.0, 0.0, 80.0));
    shape.extend(solid_box(64.0, 0.0, 30.0, 30.0, 0.0, 120.0));
    let map = read(&shape, &WHOLE);
    let (tall, kerb) = (160 * Q, 48 * Q);
    let step = kerb - 1;
    assert_eq!(map.step_to(0, cell(-64, 0), step, tall, 0, kerb), Some(0));
    // from the first stone the walker is at the ground's level, and the
    // second is 80 above that, not 40
    assert_eq!(map.step_to(0, cell(0, 0), step, tall, 0, kerb), None);
}

#[test]
fn a_body_needs_room_beside_it_as_well_as_under_it() {
    let map = read(&solid_box(0.0, 0.0, 16.0, 16.0, 0.0, 200.0), &WHOLE);
    let beside = cell(40, 0);
    assert!(
        map.column(beside.0, beside.1).is_none(),
        "nothing stands over the next cell"
    );
    assert_eq!(
        map.step_to(0, beside, 48 * Q, 160 * Q, 0, 48 * Q),
        Some(0),
        "a point fits"
    );
    assert_eq!(
        map.step_to(0, beside, 48 * Q, 160 * Q, 40, 48 * Q),
        None,
        "a body 40 wide each way does not"
    );
    let clear = cell(120, 0);
    assert_eq!(map.step_to(0, clear, 48 * Q, 160 * Q, 40, 48 * Q), Some(0));
}

#[test]
fn of_two_places_to_stand_the_nearer_in_level_is_taken() {
    // a ledge at 50 with a second at 130 above it, far enough apart to stand between
    let mut shape = solid_box(0.0, 0.0, 60.0, 60.0, 40.0, 50.0);
    shape.extend(solid_box(0.0, 0.0, 60.0, 60.0, 120.0, 130.0));
    let map = read(&shape, &WHOLE);
    let here = cell(0, 0);
    // from 60 up, with a short body and a long step: the lower ledge is nearer
    assert_eq!(
        map.step_to(60 * Q, here, 96 * Q, 40 * Q, 0, 48 * Q),
        Some(50 * Q)
    );
    // from 120 up: the upper one is
    assert_eq!(
        map.step_to(120 * Q, here, 96 * Q, 40 * Q, 0, 48 * Q),
        Some(130 * Q)
    );
}

#[test]
fn the_ground_is_read_off_the_triangles_a_tile_is_drawn_with() {
    use rift_sim::heights::Ground;
    // one tile, 128 across; corners (0,0)=0, (1,0)=160, (0,1)=320, (1,1)=480, in sixteenths
    let ground = Ground::new(2, 128, vec![0, 160, 320, 480]);
    let span = 128 * Q;
    assert_eq!(ground.at(0, 0), 0);
    assert_eq!(ground.at(span, 0), 160);
    assert_eq!(ground.at(0, span), 320);
    assert_eq!(ground.at(span, span), 480);
    assert_eq!(ground.at(span / 2, 0), 80, "half-way along an edge");
    assert_eq!(
        ground.at(span / 2, span / 2),
        240,
        "on the split, half-way between its ends"
    );
    // a tile that is not flat across its split: the two halves are different planes
    let folded = Ground::new(2, 128, vec![0, 0, 0, 1600]);
    assert_eq!(folded.at(span / 4, span / 4), 0, "the near half lies flat");
    assert_eq!(
        folded.at(3 * span / 4, 3 * span / 4),
        800,
        "the far half rises to its corner"
    );
    // beyond the map's edge the last corner's height carries on
    assert_eq!(ground.at(-500, -500), 0);
}

#[test]
fn the_same_number_of_cells_in_different_places_is_not_agreement() {
    // the comparison is of the cells themselves, never of how many there are
    let mut map = Grid::with_cell(8, 8, 32);
    let mut other = Grid::with_cell(8, 8, 32);
    for cell in [(2, 2), (3, 2), (4, 2)] {
        map.set_blocked(cell.0, cell.1, true);
    }
    for cell in [(2, 2), (3, 2), (4, 3)] {
        other.set_blocked(cell.0, cell.1, true);
    }
    let count = |grid: &Grid| {
        (0..8)
            .flat_map(|y| (0..8).map(move |x| (x, y)))
            .filter(|&(x, y)| grid.is_blocked(x, y))
            .count()
    };
    assert_eq!(count(&map), count(&other));
    let found = compare(&map, &other);
    assert!(!found.agrees(), "{found:?}");
    assert_eq!((found.both, found.only_map, found.only_heights), (2, 1, 1));
    // and a map agrees with itself
    assert!(compare(&map, &map).agrees());
}

#[test]
fn two_kerbs_are_not_one() {
    // A stone 34 high with another over it reaching 75. Each is less than a
    // kerb above the one before, but a walker on the ground does not get to
    // add them up: the cell is closed to it, as the walkable map has it.
    let mut stones = solid_box(0.0, 0.0, 40.0, 40.0, 0.0, 34.0);
    stones.extend(solid_box(0.0, 0.0, 40.0, 40.0, 39.0, 75.0));
    let map = read(&stones, &WHOLE);
    let (cx, cy) = cell(0, 0);
    let column = map.column(cx, cy).unwrap();
    assert!(!column.open_from_the_ground(48 * Q - 1, 160 * Q, 48 * Q));
    assert_eq!(
        map.step_to(0, (cx, cy), 48 * Q - 1, 160 * Q, 0, 48 * Q),
        None
    );
    // a champion whose step reaches the upper stone's top goes there
    assert_eq!(
        map.step_to(0, (cx, cy), 96 * Q, 160 * Q, 0, 48 * Q),
        Some(75 * Q)
    );
}

#[test]
fn a_walk_in_from_open_ground_leaves_open_what_the_walkable_map_leaves_open() {
    // every kind of shape, at several places within a cell: with a step just
    // under 48 and a body of 28, the cells the walk can be in are the cells
    // the walkable map has open, one for one
    let (low, high, body, cell_size) = (48.0f32, 160.0f32, 28, 32);
    let (kerb, tall) = (48 * Q, 160 * Q);
    for (name, shape) in &every_shape() {
        for within in [
            (0.0f32, 0.0f32),
            (16.0, 16.0),
            (7.5, 29.0),
            (31.9, 0.1),
            (20.3, 11.7),
        ] {
            let at = (512.0 + within.0, 512.0 + within.1);
            let mut grid = Grid::with_cell(32, 32, cell_size);
            grid.block_solid(shape, at, low, high, body);
            let mut map = HeightMap::new(32, 32, cell_size);
            map.add_shape(shape, at, &GAME);
            let reach = map.reach(kerb - 1, tall, body, kerb, 1);
            for cy in 0..32 {
                for cx in 0..32 {
                    assert_eq!(
                        reach.open((cx, cy)),
                        !grid.is_blocked(cx, cy),
                        "{name} at {within:?}, cell {cx},{cy}"
                    );
                }
            }
        }
    }
}

#[test]
fn what_a_bigger_step_reaches_is_counted_in_patches() {
    // a broad block 80 high: a step of 96 gets onto it and a step under 48
    // does not; a thin post beside it gains nobody anything
    let mut shape = solid_box(0.0, 0.0, 120.0, 120.0, 0.0, 80.0);
    shape.extend(solid_box(300.0, 0.0, 10.0, 10.0, 0.0, 300.0));
    let map = read(&shape, &WHOLE);
    let (kerb, tall) = (48 * Q, 160 * Q);
    let walker = map.reach(kerb - 1, tall, 28, kerb, 1);
    let climber = map.reach(96 * Q, tall, 28, kerb, 1);
    let gained = climber.beyond(&walker);
    assert_eq!(gained.len(), 1, "one patch: the block's top");
    let (cells, top) = &gained[0];
    assert_eq!(*top, 80 * Q);
    // 240 across is seven or eight cells each way
    assert!(cells.len() >= 36 && cells.len() <= 81, "{}", cells.len());
    assert!(walker.beyond(&climber).is_empty());
    assert!(!walker.open(cell(0, 0)) && climber.open(cell(0, 0)));

    // A body on the ground cannot be in the cell beside the block, and the
    // block's edge is the cell beyond that: getting up is a move of two
    // cells. Allowed only moves to the next cell, a body gets onto nothing,
    // though a point does.
    let next_cell_only = map.reach(96 * Q, tall, 28, kerb, 0);
    assert!(next_cell_only.beyond(&walker).is_empty());
    let point = map.reach(96 * Q, tall, 0, kerb, 0);
    assert!(
        !point
            .beyond(&map.reach(kerb - 1, tall, 0, kerb, 0))
            .is_empty()
    );
}

#[test]
fn a_change_of_level_does_not_pass_through_a_wall() {
    // a block 80 high with a thin wall 300 high along one side of it
    let mut shape = solid_box(0.0, 0.0, 120.0, 120.0, 0.0, 80.0);
    shape.extend(solid_box(-136.0, 0.0, 10.0, 400.0, 0.0, 300.0));
    let map = read(&shape, &WHOLE);
    let (kerb, tall) = (48 * Q, 160 * Q);
    let climber = map.reach(96 * Q, tall, 28, kerb, 1);
    // it gets up from the other sides, but stands nowhere beside the wall
    assert!(climber.open(cell(60, 0)));
    assert!(climber.levels(cell(-110, 0)).is_empty());
    assert!(climber.levels(cell(-80, 0)).is_empty());
}
