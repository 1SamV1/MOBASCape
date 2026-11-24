//! Skeleton binding for models built from several cache parts.
//!
//! OSRS heroes are often several models: an NPC's body plus extra parts
//! (Graardor's second model, Kree'arra's wings) or a player body built from
//! identity kits (head, torso, arms, legs, feet). Every vertex carries a
//! skeleton label, and an animation frame moves labels, not models. If a
//! part loses its labels it stays frozen in the rest pose while the body
//! animates: heads and feet come apart from the torso, hands from arms.
//!
//! [`assemble`] merges the parts and binds every attachment one of three
//! ways, checked against the clips the hero actually plays:
//!
//! * **Native**: keep the part's own labels, exactly as the OSRS client does
//!   for parts that share the body's skeleton.
//! * **Follow**: the part comes from another skeleton (a mixed-era kit, a
//!   weapon's model) or has no labels. Each vertex is bound to the label of
//!   the nearest already-bound vertex *in its own limb region* and moves
//!   with it, without counting toward its pivot ([`Model::follow_groups`]),
//!   so it cannot drag the joints. A robe hem by the hands follows the
//!   hips, not the swinging arm.
//! * **Rigid**: the whole part follows one label (the closest touch), for
//!   solid objects such as a staff. Never for a pair (both hands, both
//!   boots) spanning both sides of the body.
//!
//! The skeleton grows as parts bind: a kit hero's first part may be only
//! its sleeves, so later parts may anchor to any vertex bound before them.
//!
//! The choice is measured, not guessed. Each candidate scores how far its
//! touching vertices drift from the body ([`DRIFT_LIMIT`] guides the tests)
//! plus how much its own mesh edges stretch. Native labels win unless they
//! tear the body at its own joints ([`TEAR_LIMIT`]) or score worse than the
//! best follower by more than [`NATIVE_BIAS`]. Moving the body as a whole is
//! allowed: the OSRS client merges every part's labels, so the pivots it
//! computes include them.
//!
//! [`identify_limbs`] names each label's limb (head, torso, arms, legs,
//! tail) from where its vertices sit in the rest pose, and the same rules
//! name the region of every part vertex, which limits what it may follow.
//! `rift-tools skeleton <hero|all> [--sheet out.png]` prints the bindings
//! and limb map and draws old-merge vs bound poses side by side.

use crate::model::Model;
use crate::sequence::AnimationClip;

/// How much (fraction of model height) native labels may widen the gaps at
/// the body's own joints (touching vertices in different limbs) compared
/// with the body animated without them.
pub const TEAR_LIMIT: f32 = 0.01;
/// Joint pairs sampled from the body to measure tearing.
const JOINT_PAIRS: usize = 400;
/// How far (fraction of model height) a part's touching vertices may drift
/// from the body vertex they touch at rest, averaged over the clips.
pub const DRIFT_LIMIT: f32 = 0.03;
/// Part vertices closer than this (fraction of height) to a body vertex at
/// rest count as touching the body.
pub const CONTACT: f32 = 0.05;
/// Animation samples taken per clip when scoring a binding.
const SAMPLES: usize = 10;
/// How much worse (fraction of height) native labels may score than the
/// best follower and still be kept.
pub const NATIVE_BIAS: f32 = 0.01;
/// How much better rigid must score than regional follow to be chosen.
const SCORE_MARGIN: f32 = 0.002;
/// Mesh edges sampled per part to measure stretch.
const EDGE_SAMPLES: usize = 600;

/// What a skeleton label moves, judged from the rest pose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Limb {
    /// Spans most of the model: a whole-body or root label.
    Root,
    Head,
    Torso,
    ArmLeft,
    ArmRight,
    LegLeft,
    LegRight,
    /// Behind the body (tails, wings' trailing edge).
    Tail,
    /// No vertices.
    Empty,
}

impl Limb {
    pub fn label(self) -> &'static str {
        match self {
            Limb::Root => "root",
            Limb::Head => "head",
            Limb::Torso => "torso",
            Limb::ArmLeft => "arm L",
            Limb::ArmRight => "arm R",
            Limb::LegLeft => "leg L",
            Limb::LegRight => "leg R",
            Limb::Tail => "tail",
            Limb::Empty => "-",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binding {
    /// The first part: its skeleton is the reference.
    Base,
    Native,
    /// Each vertex follows the nearest body label in its own region (a robe
    /// hem follows the legs, a cuff the arm).
    Follow,
    /// The whole part follows one body label, keeping its shape (weapons,
    /// helmets, shells).
    Rigid,
}

/// What [`assemble`] decided for one part, with the numbers behind it.
#[derive(Debug, Clone)]
pub struct PartReport {
    pub index: usize,
    pub model_id: u32,
    pub vertices: usize,
    /// Vertices that carry a skeleton label.
    pub labelled: usize,
    /// Vertices touching the body at rest.
    pub contacts: usize,
    pub binding: Binding,
    /// How much native labels widen the body's own joints (fraction of
    /// height); above [`TEAR_LIMIT`] the part is a foreign skeleton.
    pub native_tear: f32,
    /// Contact drift with native labels / when bound to the nearest limb /
    /// with no labels at all (the old behaviour), fractions of height.
    pub native_drift: f32,
    pub follow_drift: f32,
    pub static_drift: f32,
    /// How much the part's own edges stretch (fraction of height) with
    /// native labels / regional follow / rigid follow.
    pub native_stretch: f32,
    pub follow_stretch: f32,
    pub rigid_drift: f32,
}

/// Merge `parts` (the body first) and bind each attachment to the skeleton,
/// scored against `clips` (idle, walk, attack...). Returns the model and one
/// report per part.
pub fn assemble(parts: &[Model], clips: &[&AnimationClip]) -> (Model, Vec<PartReport>) {
    let mut model = Model::merge(parts);
    let mut reports = Vec::with_capacity(parts.len());
    let Some(base) = parts.first() else {
        return (model, reports);
    };
    reports.push(PartReport {
        index: 0,
        model_id: base.id,
        vertices: base.vertex_count,
        labelled: labelled_count(base),
        contacts: 0,
        binding: Binding::Base,
        native_tear: 0.0,
        native_drift: 0.0,
        follow_drift: 0.0,
        static_drift: 0.0,
        native_stretch: 0.0,
        follow_stretch: 0.0,
        rigid_drift: 0.0,
    });
    let base_groups = per_vertex_groups(base);
    let height = model_height(&model).max(1.0);
    let joints = body_joints(&model, base, &base_groups, height);
    // Limb identification: which limb each body label moves, and which
    // region of the figure each vertex sits in. A vertex may only follow a
    // label of its own limb (or the torso), so a robe hem by the hands
    // follows the hips, not the swinging arm.
    let frame = FigureFrame::of(&model);

    let mut start = base.vertex_count;
    for (index, part) in parts.iter().enumerate().skip(1) {
        let range = start..start + part.vertex_count;
        start += part.vertex_count;
        // The skeleton grows as parts bind natively: a kit hero's first
        // part may be only the sleeves, so the hands, feet and chest bound
        // before this part are anchors too.
        let bound = 0..range.start;
        let labels = anchor_labels(&model);
        let limbs = identify_limbs(&model);
        let label_limbs: Vec<Option<Limb>> = labels
            .iter()
            .map(|g| g.map(|g| limbs.get(g as usize).copied().unwrap_or(Limb::Torso)))
            .collect();

        // Each part vertex's nearest labelled body vertex at rest.
        let anchors: Vec<Option<(usize, f32)>> = range
            .clone()
            .map(|v| {
                let region = frame.region(position(&model, v));
                nearest_labelled(&model, v, bound.clone(), &labels, |b| {
                    label_limbs[b].is_some_and(|limb| region.accepts(limb))
                })
                .or_else(|| nearest_labelled(&model, v, bound.clone(), &labels, |_| true))
            })
            .collect();
        let contacts: Vec<(usize, usize, f32)> = range
            .clone()
            .zip(&anchors)
            .filter_map(|(v, anchor)| {
                anchor
                    .filter(|(_, d)| *d <= CONTACT * height)
                    .map(|(a, d)| (v, a, d))
            })
            .collect();

        // Candidate bindings, each built on the model as assembled so far.
        let mut native = model.clone();
        add_native_groups(&mut native, part, range.start);
        let mut follow = model.clone();
        add_follow_groups(&mut follow, range.clone(), &anchors, &labels);
        // Rigid: everything follows the label of the closest touch (or the
        // closest anchor when nothing touches).
        let closest = anchors
            .iter()
            .flatten()
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .copied();
        let mut rigid = model.clone();
        add_follow_groups(
            &mut rigid,
            range.clone(),
            &vec![closest; part.vertex_count],
            &labels,
        );
        let edges = part_edges(&model, range.clone(), height);

        let labelled = labelled_count(part);
        let native_tear = if labelled > 0 {
            (contact_drift(&native, &joints, clips) - contact_drift(&model, &joints, clips))
                .max(0.0)
                / height
        } else {
            f32::INFINITY
        };
        let (native_drift, native_stretch) = if labelled > 0 {
            measure(&native, &contacts, &edges, clips, height)
        } else {
            (f32::INFINITY, f32::INFINITY)
        };
        let (follow_drift, follow_stretch) = measure(&follow, &contacts, &edges, clips, height);
        let (rigid_drift, _) = measure(&rigid, &contacts, &[], clips, height);
        let static_drift = contact_drift(&model, &contacts, clips) / height;

        // Each candidate scores how far it pulls off the body plus how much
        // it warps itself. Regional follow unless rigid scores better.
        let follow_score = follow_drift + follow_stretch;
        // A part built on a single label of its own (a staff, a blade) is
        // one solid object: it follows rigidly.
        let own_labels = part
            .vertex_groups
            .as_ref()
            .map_or(0, |g| g.iter().filter(|v| !v.is_empty()).count());
        // A pair (both hands, both boots) can never move as one piece.
        let regions: Vec<Limb> = range
            .clone()
            .map(|v| frame.region(position(&model, v)))
            .collect();
        let paired = |a: Limb, b: Limb| regions.contains(&a) && regions.contains(&b);
        let bilateral =
            paired(Limb::ArmLeft, Limb::ArmRight) || paired(Limb::LegLeft, Limb::LegRight);
        let rigid_better = !bilateral
            && ((own_labels == 1 && rigid_drift <= DRIFT_LIMIT)
                || rigid_drift + SCORE_MARGIN < follow_score);
        let (follower, best_score) = if rigid_better {
            (Binding::Rigid, rigid_drift)
        } else {
            (Binding::Follow, follow_score)
        };
        // Native labels are what the OSRS client does, so they win unless
        // they tear the body's joints or score clearly worse.
        let keep_native = labelled * 2 >= part.vertex_count
            && native_tear <= TEAR_LIMIT
            && native_drift + native_stretch <= best_score + NATIVE_BIAS;
        let binding = if keep_native {
            model = native;
            Binding::Native
        } else {
            model = if rigid_better { rigid } else { follow };
            follower
        };
        reports.push(PartReport {
            index,
            model_id: part.id,
            vertices: part.vertex_count,
            labelled,
            contacts: contacts.len(),
            binding,
            native_tear,
            native_drift,
            follow_drift,
            static_drift,
            native_stretch,
            follow_stretch,
            rigid_drift,
        });
    }
    (model, reports)
}

/// The limb each skeleton label moves, from the rest pose: one entry per
/// label in `model.vertex_groups` (followers are ignored).
pub fn identify_limbs(model: &Model) -> Vec<Limb> {
    let Some(groups) = model.vertex_groups.as_ref() else {
        return Vec::new();
    };
    let frame = FigureFrame::of(model);
    groups
        .iter()
        .map(|vertices| {
            if vertices.is_empty() {
                return Limb::Empty;
            }
            let mut lo = [i32::MAX; 3];
            let mut hi = [i32::MIN; 3];
            let mut sum = [0f64; 3];
            for &v in vertices {
                let p = position(model, v as usize);
                for axis in 0..3 {
                    lo[axis] = lo[axis].min(p[axis]);
                    hi[axis] = hi[axis].max(p[axis]);
                    sum[axis] += p[axis] as f64;
                }
            }
            let n = vertices.len() as f64;
            let c = [sum[0] / n, sum[1] / n, sum[2] / n].map(|v| v as f32);
            let span = (hi[1] - lo[1]) as f32 / frame.height;
            frame.classify(c, span)
        })
        .collect()
}

/// The rest-pose box of a figure, for naming limbs and regions.
struct FigureFrame {
    top: f32,
    height: f32,
    half_width: f32,
    half_depth: f32,
    centre_x: f32,
    centre_z: f32,
}

impl FigureFrame {
    fn of(model: &Model) -> Self {
        let (min, max) = bounds(model);
        Self {
            top: min[1] as f32,
            height: (max[1] - min[1]).max(1) as f32,
            half_width: (max[0] - min[0]).max(1) as f32 * 0.5,
            half_depth: (max[2] - min[2]).max(1) as f32 * 0.5,
            centre_x: (min[0] + max[0]) as f32 * 0.5,
            centre_z: (min[2] + max[2]) as f32 * 0.5,
        }
    }

    /// The region a single rest-pose point sits in.
    fn region(&self, p: [i32; 3]) -> Limb {
        self.classify(p.map(|v| v as f32), 0.0)
    }

    /// Name the limb at centroid `c` whose vertical extent is `span`
    /// (fraction of height).
    fn classify(&self, c: [f32; 3], span: f32) -> Limb {
        {
            // Cache Y grows downward: 0 at the top of the model, 1 at the feet.
            let down = (c[1] - self.top) / self.height;
            let side = (c[0] - self.centre_x) / self.half_width;
            let back = (c[2] - self.centre_z) / self.half_depth;
            if span > 0.7 {
                Limb::Root
            } else if down < 0.22 {
                Limb::Head
            } else if side.abs() > 0.5 && down < 0.85 {
                // Far out to the side: arms, even long ones that hang below
                // the hips (apes, brutes). Legs sit under the body.
                if side < 0.0 {
                    Limb::ArmLeft
                } else {
                    Limb::ArmRight
                }
            } else if down > 0.62 {
                if side < 0.0 {
                    Limb::LegLeft
                } else {
                    Limb::LegRight
                }
            } else if side.abs() > 0.38 {
                if side < 0.0 {
                    Limb::ArmLeft
                } else {
                    Limb::ArmRight
                }
            } else if back.abs() > 0.55 {
                Limb::Tail
            } else {
                Limb::Torso
            }
        }
    }
}

impl Limb {
    /// Whether a vertex in this region may follow a label that moves
    /// `limb`: its own limb, or the core of the body.
    fn accepts(self, limb: Limb) -> bool {
        self == limb || matches!(limb, Limb::Torso | Limb::Root)
    }
}

fn position(model: &Model, v: usize) -> [i32; 3] {
    [model.vertex_x[v], model.vertex_y[v], model.vertex_z[v]]
}

fn bounds(model: &Model) -> ([i32; 3], [i32; 3]) {
    let mut lo = [i32::MAX; 3];
    let mut hi = [i32::MIN; 3];
    for v in 0..model.vertex_x.len() {
        let p = position(model, v);
        for axis in 0..3 {
            lo[axis] = lo[axis].min(p[axis]);
            hi[axis] = hi[axis].max(p[axis]);
        }
    }
    if model.vertex_x.is_empty() {
        return ([0; 3], [0; 3]);
    }
    (lo, hi)
}

fn model_height(model: &Model) -> f32 {
    let (lo, hi) = bounds(model);
    (hi[1] - lo[1]) as f32
}

/// The label of every vertex of `model` (None when unlabelled).
fn per_vertex_groups(model: &Model) -> Vec<Option<u8>> {
    let mut out = vec![None; model.vertex_count];
    if let Some(groups) = model.vertex_groups.as_ref() {
        for (group, vertices) in groups.iter().enumerate() {
            for &v in vertices {
                if let Some(slot) = out.get_mut(v as usize) {
                    *slot = Some(group as u8);
                }
            }
        }
    }
    out
}

/// The label every vertex moves with: its own, or the one it follows
/// (so a glove can anchor to a sleeve that is itself a follower).
fn anchor_labels(model: &Model) -> Vec<Option<u8>> {
    let mut out = per_vertex_groups(model);
    if let Some(groups) = model.follow_groups.as_ref() {
        for (group, vertices) in groups.iter().enumerate() {
            for &v in vertices {
                if let Some(slot @ None) = out.get_mut(v as usize) {
                    *slot = Some(group as u8);
                }
            }
        }
    }
    out
}

fn labelled_count(model: &Model) -> usize {
    per_vertex_groups(model)
        .iter()
        .filter(|g| g.is_some())
        .count()
}

fn nearest_labelled(
    model: &Model,
    v: usize,
    base: std::ops::Range<usize>,
    base_groups: &[Option<u8>],
    allowed: impl Fn(usize) -> bool,
) -> Option<(usize, f32)> {
    let p = position(model, v);
    base.filter(|&b| base_groups.get(b).is_some_and(Option::is_some) && allowed(b))
        .map(|b| {
            let q = position(model, b);
            let d = (((p[0] - q[0]) as f64).powi(2)
                + ((p[1] - q[1]) as f64).powi(2)
                + ((p[2] - q[2]) as f64).powi(2))
            .sqrt() as f32;
            (b, d)
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
}

fn add_native_groups(model: &mut Model, part: &Model, offset: usize) {
    let Some(groups) = part.vertex_groups.as_ref() else {
        return;
    };
    let merged = model.vertex_groups.get_or_insert_with(Vec::new);
    if merged.len() < groups.len() {
        merged.resize(groups.len(), Vec::new());
    }
    for (index, vertices) in groups.iter().enumerate() {
        merged[index].extend(vertices.iter().map(|&v| v + offset as u32));
    }
}

fn add_follow_groups(
    model: &mut Model,
    range: std::ops::Range<usize>,
    anchors: &[Option<(usize, f32)>],
    base_groups: &[Option<u8>],
) {
    let follow = model.follow_groups.get_or_insert_with(Vec::new);
    for (v, anchor) in range.zip(anchors) {
        let Some(group) = anchor.and_then(|(a, _)| base_groups[a]) else {
            continue;
        };
        if follow.len() <= group as usize {
            follow.resize(group as usize + 1, Vec::new());
        }
        follow[group as usize].push(v as u32);
    }
}

/// Sample times spread over each clip's loop.
fn sample_times(clip: &AnimationClip) -> Vec<f32> {
    if clip.frames.is_empty() {
        return Vec::new();
    }
    let loop_ms = clip.loop_ms();
    (0..SAMPLES)
        .map(|i| loop_ms * i as f32 / SAMPLES as f32)
        .collect()
}

/// The body's own joints: touching pairs of base vertices in different
/// labels, an even sample of at most [`JOINT_PAIRS`].
fn body_joints(
    model: &Model,
    base: &Model,
    base_groups: &[Option<u8>],
    height: f32,
) -> Vec<(usize, usize, f32)> {
    let mut pairs = Vec::new();
    let n = base.vertex_count;
    for a in 0..n {
        let Some(ga) = base_groups[a] else { continue };
        let pa = position(model, a);
        for (b, gb) in base_groups.iter().enumerate().take(n).skip(a + 1) {
            let Some(gb) = *gb else { continue };
            if ga == gb {
                continue;
            }
            let pb = position(model, b);
            let d = (((pa[0] - pb[0]) as f64).powi(2)
                + ((pa[1] - pb[1]) as f64).powi(2)
                + ((pa[2] - pb[2]) as f64).powi(2))
            .sqrt() as f32;
            if d <= CONTACT * height * 0.5 {
                pairs.push((a, b, d));
            }
        }
    }
    if pairs.len() > JOINT_PAIRS {
        let step = pairs.len() as f32 / JOINT_PAIRS as f32;
        pairs = (0..JOINT_PAIRS)
            .map(|i| pairs[(i as f32 * step) as usize])
            .collect();
    }
    pairs
}

/// Mean change in distance between each touching part vertex and the body
/// vertex it touches at rest, across the clips.
fn contact_drift(model: &Model, contacts: &[(usize, usize, f32)], clips: &[&AnimationClip]) -> f32 {
    if contacts.is_empty() {
        return 0.0;
    }
    let mut total = 0f64;
    let mut count = 0usize;
    for clip in clips {
        for t in sample_times(clip) {
            let p = model.animated_positions(clip, t);
            for &(v, a, rest) in contacts {
                total += (dist(p[v], p[a]) - rest).abs() as f64;
                count += 1;
            }
        }
    }
    if count == 0 {
        0.0
    } else {
        (total / count as f64) as f32
    }
}

/// The part's own mesh edges (vertex pairs from its faces, both inside
/// `range`), an even sample of at most [`EDGE_SAMPLES`], with rest lengths.
fn part_edges(
    model: &Model,
    range: std::ops::Range<usize>,
    height: f32,
) -> Vec<(usize, usize, f32)> {
    let inside = |v: u32| range.contains(&(v as usize));
    let mut edges: Vec<(usize, usize)> = Vec::new();
    for face in &model.faces {
        if !face.iter().all(|&v| inside(v)) {
            continue;
        }
        for (a, b) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            let (a, b) = (a.min(b) as usize, a.max(b) as usize);
            if a != b {
                edges.push((a, b));
            }
        }
    }
    edges.sort_unstable();
    edges.dedup();
    if edges.len() > EDGE_SAMPLES {
        let step = edges.len() as f32 / EDGE_SAMPLES as f32;
        edges = (0..EDGE_SAMPLES)
            .map(|i| edges[(i as f32 * step) as usize])
            .collect();
    }
    let rest = |v: usize| position(model, v).map(|c| c as f32);
    edges
        .into_iter()
        .map(|(a, b)| (a, b, dist(rest(a), rest(b))))
        .filter(|&(_, _, d)| d > height * 0.001)
        .collect()
}

/// Contact drift and edge stretch (both fractions of `height`) of a
/// candidate across the clips, from one animation pass.
fn measure(
    model: &Model,
    contacts: &[(usize, usize, f32)],
    edges: &[(usize, usize, f32)],
    clips: &[&AnimationClip],
    height: f32,
) -> (f32, f32) {
    let (mut drift, mut stretch) = (0f64, 0f64);
    let (mut drift_n, mut stretch_n) = (0usize, 0usize);
    for clip in clips {
        for t in sample_times(clip) {
            let p = model.animated_positions(clip, t);
            for &(v, a, rest) in contacts {
                drift += (dist(p[v], p[a]) - rest).abs() as f64;
                drift_n += 1;
            }
            for &(a, b, rest) in edges {
                stretch += (dist(p[a], p[b]) - rest).abs() as f64;
                stretch_n += 1;
            }
        }
    }
    let mean = |sum: f64, n: usize| {
        if n == 0 {
            0.0
        } else {
            (sum / n as f64) as f32 / height
        }
    };
    (mean(drift, drift_n), mean(stretch, stretch_n))
}

fn dist(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sequence::{Frame, Framemap, Sequence};

    /// A box of 8 vertices around `centre`, all in `label` (None: unlabelled).
    fn block(id: u32, centre: [i32; 3], half: [i32; 3], label: Option<u8>) -> Model {
        let mut model = Model::merge(&[]);
        model.id = id;
        for i in 0..8 {
            let sx = if i & 1 == 0 { -1 } else { 1 };
            let sy = if i & 2 == 0 { -1 } else { 1 };
            let sz = if i & 4 == 0 { -1 } else { 1 };
            model.vertex_x.push(centre[0] + sx * half[0]);
            model.vertex_y.push(centre[1] + sy * half[1]);
            model.vertex_z.push(centre[2] + sz * half[2]);
        }
        model.vertex_count = 8;
        model.faces = vec![[0, 1, 2]];
        model.face_count = 1;
        model.face_colors = vec![0];
        model.face_normals = vec![[0.0, 1.0, 0.0]];
        if let Some(label) = label {
            let mut groups = vec![Vec::new(); label as usize + 1];
            groups[label as usize] = (0..8).collect();
            model.vertex_groups = Some(groups);
        }
        model
    }

    /// A two-frame clip: frame 0 is the rest pose, frame 1 translates
    /// `label` by `dx` (framemap: origin then translate on that label).
    fn shove(label: u8, dx: i32) -> AnimationClip {
        let framemap = Framemap {
            id: 1,
            length: 2,
            types: vec![0, 1],
            frame_maps: vec![vec![label], vec![label]],
        };
        let rest = Frame {
            id: 0,
            framemap_id: 1,
            index_frame_ids: vec![],
            translator_x: vec![],
            translator_y: vec![],
            translator_z: vec![],
            showing: false,
        };
        let moved = Frame {
            id: 1,
            framemap_id: 1,
            index_frame_ids: vec![1],
            translator_x: vec![dx],
            translator_y: vec![0],
            translator_z: vec![0],
            showing: false,
        };
        AnimationClip {
            sequence: Sequence {
                frame_ids: vec![0, 1],
                frame_lengths: vec![10, 10],
                max_loops: 99,
                ..Default::default()
            },
            frames: vec![rest, moved],
            framemaps: vec![framemap.clone(), framemap],
        }
    }

    #[test]
    fn a_part_on_the_same_skeleton_keeps_its_labels() {
        // Torso (label 1) and a head (also label 1) touching it.
        let torso = block(1, [0, 0, 0], [40, 100, 30], Some(1));
        let head = block(2, [0, -120, 0], [30, 20, 30], Some(1));
        let clip = shove(1, 60);
        let (model, reports) = assemble(&[torso, head], &[&clip]);
        assert_eq!(reports[1].binding, Binding::Native);
        assert!(reports[1].native_drift < 0.001);
        assert!(
            reports[1].static_drift > 0.02,
            "frozen, it would come apart"
        );
        // The head moves with the torso.
        let p = model.animated_positions(&clip, 200.0);
        assert!((p[8][0] - (model.vertex_x[8] as f32 + 60.0)).abs() < 1.0);
    }

    #[test]
    fn a_foreign_part_follows_the_limb_it_touches() {
        // The part touches the torso (label 1) but carries label 2, which the
        // clip throws far away: native labels would tear it off. One label
        // of its own makes it a solid object, so it follows rigidly.
        let torso = block(1, [0, 0, 0], [40, 100, 30], Some(1));
        let sword = block(2, [60, 0, 0], [20, 100, 30], Some(2));
        let clip = shove(2, 900);
        let (model, reports) = assemble(&[torso, sword], &[&clip]);
        assert_eq!(reports[1].binding, Binding::Rigid);
        assert!(reports[1].native_drift > DRIFT_LIMIT);
        assert!(reports[1].rigid_drift < 0.001);
        let follow = model.follow_groups.as_ref().expect("followers");
        assert_eq!(follow[1].len(), 8, "bound to the torso's label");
    }

    #[test]
    fn an_unlabelled_part_follows_and_does_not_move_the_pivot() {
        let torso = block(1, [0, 0, 0], [40, 100, 30], Some(1));
        let crown = block(2, [0, -110, 0], [30, 10, 30], None);
        let clip = shove(1, 60);
        let (model, reports) = assemble(&[torso.clone(), crown], &[&clip]);
        assert_eq!(reports[1].binding, Binding::Follow);
        let alone = torso.animated_positions(&clip, 200.0);
        let together = model.animated_positions(&clip, 200.0);
        for v in 0..8 {
            assert_eq!(alone[v], together[v], "the body animates as if alone");
        }
        assert!((together[8][0] - (model.vertex_x[8] as f32 + 60.0)).abs() < 1.0);
    }

    #[test]
    fn limbs_are_named_from_the_rest_pose() {
        // A box humanoid, cache Y down: head on top, arms out, legs below.
        let parts = [
            block(1, [0, -170, 0], [20, 20, 20], Some(1)),  // head
            block(2, [0, -90, 0], [35, 50, 20], Some(2)),   // torso
            block(3, [-70, -90, 0], [12, 45, 12], Some(3)), // arm L
            block(4, [70, -90, 0], [12, 45, 12], Some(4)),  // arm R
            block(5, [-20, 0, 0], [12, 40, 12], Some(5)),   // leg L
            block(6, [20, 0, 0], [12, 40, 12], Some(6)),    // leg R
        ];
        let mut model = Model::merge(&parts);
        let mut groups = vec![Vec::new(); 7];
        for i in 0..6 {
            groups[i + 1] = (i as u32 * 8..i as u32 * 8 + 8).collect();
        }
        model.vertex_groups = Some(groups);
        assert_eq!(
            identify_limbs(&model),
            vec![
                Limb::Empty,
                Limb::Head,
                Limb::Torso,
                Limb::ArmLeft,
                Limb::ArmRight,
                Limb::LegLeft,
                Limb::LegRight
            ]
        );
    }
}
