//! Stage 1 of "Getting about on rough ground": the height data on its own
//! terms, and against the walkable map that exists.

use rift_sim::Grid;
use rift_sim::heights::{HeightMap, Q, Reading, agreement};

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

/// The promise of stage 1: read with today's step and head height, the height
/// data blocks the same cells as the walkable map that exists.
#[test]
fn it_blocks_what_the_walkable_map_blocks_for_every_kind_of_shape() {
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
    let shapes: Vec<(&str, Vec<Tri>)> = vec![
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
    ];
    for (name, shape) in &shapes {
        for body in [0, 28] {
            let found = agreement(shape, true, 45.0, 48.0, 160.0, body, 32);
            assert!(found.agrees(), "{name}, body {body}: {found:?}");
        }
    }
    // and the comparison is not empty-handed: a tower blocks a good many cells
    let found = agreement(&shapes[4].1, true, 45.0, 48.0, 160.0, 28, 32);
    assert!(found.both > 60, "{found:?}");
}

#[test]
fn the_finishing_step_is_the_same_whichever_way_the_cells_were_found() {
    // block_cells on the cells block_solid finds gives block_solid's result
    let shape = solid_box(0.0, 0.0, 110.0, 110.0, 0.0, 500.0);
    let mut whole = Grid::with_cell(32, 32, 32);
    whole.block_solid(&shape, (512.0, 512.0), 48.0, 160.0, 28);
    let mut bare = Grid::with_cell(32, 32, 32);
    bare.block_solid(&shape, (512.0, 512.0), 48.0, 160.0, 0);
    let cells: Vec<(i32, i32)> = (0..32)
        .flat_map(|y| (0..32).map(move |x| (x, y)))
        .filter(|&(x, y)| bare.is_blocked(x, y))
        .collect();
    let mut again = Grid::with_cell(32, 32, 32);
    again.block_cells(&cells, 28);
    for y in 0..32 {
        for x in 0..32 {
            assert_eq!(whole.is_blocked(x, y), again.is_blocked(x, y), "{x},{y}");
        }
    }
}

#[test]
fn two_surfaces_each_within_a_step_of_the_ground_are_not_within_a_step_of_each_other() {
    // a stone 40 high beside a ledge 130 high: 90 between them
    let mut shape = solid_box(-64.0, 0.0, 32.0, 48.0, 0.0, 40.0);
    shape.extend(solid_box(48.0, 0.0, 80.0, 48.0, 0.0, 130.0));
    let map = read(&shape, &WHOLE);
    let (stone, ledge) = (cell(-64, 0), cell(48, 0));
    let (tall, body) = (160 * Q, 0);

    // a step of 48: onto the stone from the ground, and no further
    assert_eq!(
        map.step_to(0, stone, 48 * Q, tall, body, 48 * Q),
        Some(40 * Q)
    );
    assert_eq!(map.step_to(40 * Q, ledge, 48 * Q, tall, body, 48 * Q), None);
    assert_eq!(map.step_to(0, ledge, 48 * Q, tall, body, 48 * Q), None);

    // a step of 96: the ground to the stone, the stone to the ledge, but not
    // the ground to the ledge in one
    assert_eq!(
        map.step_to(0, stone, 96 * Q, tall, body, 48 * Q),
        Some(40 * Q)
    );
    assert_eq!(
        map.step_to(40 * Q, ledge, 96 * Q, tall, body, 48 * Q),
        Some(130 * Q)
    );
    assert_eq!(map.step_to(0, ledge, 96 * Q, tall, body, 48 * Q), None);
    // and down again the same way
    assert_eq!(
        map.step_to(130 * Q, stone, 96 * Q, tall, body, 48 * Q),
        Some(40 * Q)
    );
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
