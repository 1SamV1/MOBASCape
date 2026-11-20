//! The plain data the interface shows (docs/INTERFACE_STAGE3_PLAN.md,
//! section 3). In stage 3 every value here is made up by `sample.rs`; in
//! stage 4 the client fills the same fields from the match, and nothing in
//! a component has to change.

/// One jungle camp's line in the objectives panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CampState {
    pub name: String,
    /// "Might" or "Swift".
    pub grant: String,
    /// "Alive", "Taken", or a countdown.
    pub state: String,
    /// A quieter second line, shown only when there is one.
    pub detail: Option<String>,
}

/// One lane's structure pips. Two towers, then the inhibitor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaneState {
    pub name: String,
    pub blue_towers: [bool; 2],
    pub blue_inhibitor: bool,
    pub red_towers: [bool; 2],
    pub red_inhibitor: bool,
}

/// A nexus line: shielded, open, or its health as text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NexusState {
    pub name: String,
    pub blue: bool,
    pub state: String,
}

/// The lanes table and the objectives list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectivesState {
    pub lanes: Vec<LaneState>,
    pub nexuses: Vec<NexusState>,
    pub camps: Vec<CampState>,
    /// How many of the objective lines have a second line today.
    pub second_lines: usize,
    /// How many second lines the usual state shows. The budget is measured
    /// on this state; the longest list is what the scale must fit.
    pub usual_second_lines: usize,
}

impl ObjectivesState {
    /// The panel is the reference's head and rule, plus 24 for every line
    /// and second line. Nothing here is a fixed 302 or 398: add a line and
    /// the height follows.
    pub const fn height(lines: usize, second_lines: usize) -> i32 {
        158 + 24 * (lines + second_lines) as i32
    }

    /// The objective lines are the nexuses and the camps. There is no
    /// separate count to fall out of step with the content.
    pub fn lines(&self) -> usize {
        self.nexuses.len() + self.camps.len()
    }

    pub fn usual_height(&self) -> i32 {
        Self::height(self.lines(), self.usual_second_lines)
    }

    pub fn longest_height(&self) -> i32 {
        Self::height(self.lines(), self.second_lines)
    }

    /// What the sample wants on screen: the full list when it is live.
    pub fn wanted_height(&self) -> i32 {
        self.longest_height()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllyState {
    pub name: String,
    pub health: u32,
    pub max_health: u32,
    pub resource: u32,
    pub max_resource: u32,
    pub level: u8,
    pub dead: bool,
    /// Seconds until respawn, when `dead`.
    pub respawn_seconds: Option<u32>,
}

/// One figure in the target panel's two columns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetStat {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetState {
    pub name: String,
    /// The tag at the panel's right: "Structure", "Hero", "Minion".
    pub kind: String,
    /// Which team the target belongs to; the name takes its colour.
    pub blue: bool,
    /// The orange line under the health bar. Stage 3 samples it; stage 4
    /// fills it from the selected unit.
    pub description: String,
    pub health: u32,
    pub max_health: u32,
    /// Up to eight figures, drawn two columns of four.
    pub stats: Vec<TargetStat>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KillFeedEntry {
    pub killer: String,
    pub victim: String,
    pub gold: u32,
    pub killer_blue: bool,
}

/// One item in the six-slot inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryItem {
    pub name: String,
    /// The item's colour, until item sprites are wired.
    pub colour: [u8; 3],
}

/// What kind of unit a plate belongs to. The three kinds draw at different
/// sizes: heroes carry a name, level and bar, minions a thin bar, structures
/// a wider one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlateKind {
    Hero,
    Minion,
    Structure,
}

/// One unit plate, already placed in window pixels by the client's
/// projection. `at` is the unit's head; the plate is drawn above it. The
/// interface draws a plate where it is told and never places one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitPlate {
    pub at: (i32, i32),
    pub kind: PlateKind,
    pub name: String,
    pub level: u8,
    pub health: u32,
    pub max_health: u32,
    pub blue: bool,
}

/// Everything the interface shows, as plain data. The component slices are
/// kept together here so the client has one thing to fill, not a dozen.
#[derive(Debug, Clone, PartialEq)]
pub struct HudState {
    pub score: ScoreState,
    pub ability: AbilityState,
    pub minimap: MinimapState,
    pub objectives: ObjectivesState,
    pub allies: Vec<AllyState>,
    pub plates: Vec<UnitPlate>,
    pub target: Option<TargetState>,
    pub inventory: Vec<Option<InventoryItem>>,
    pub gold: u32,
    pub chat: Vec<String>,
    pub feed: Vec<KillFeedEntry>,
    pub announcement: Option<String>,
    pub tooltip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoreState {
    pub clock: String,
    pub blue_kills: u32,
    pub red_kills: u32,
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub creep_score: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AbilityState {
    pub health: u32,
    pub max_health: u32,
    pub resource: u32,
    pub max_resource: u32,
    pub level: u8,
    pub experience: u32,
    pub cooldowns: [f32; 4],
}

/// A point in the minimap's own 254 by 252 picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapPoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StructureMark {
    pub at: MapPoint,
    pub blue: bool,
}

/// The minimap's picture and marks. Stage 3 fills this with a sample map;
/// stage 4 feeds the client's visible units and camera box.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinimapState {
    pub lanes: Vec<Vec<MapPoint>>,
    pub river: Vec<MapPoint>,
    pub bases: Vec<StructureMark>,
    pub allies: Vec<MapPoint>,
    pub enemies: Vec<MapPoint>,
    pub camps: Vec<MapPoint>,
    pub structures: Vec<StructureMark>,
    /// Left, top, width and height in the map picture.
    pub camera: (i32, i32, i32, i32),
}
