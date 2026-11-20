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

/// The lanes table and the objectives list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectivesState {
    /// The six objective lines: two nexus and four camps.
    pub lines: usize,
    /// How many of those lines have a second line today.
    pub second_lines: usize,
    /// How many second lines the usual state shows. The budget is measured
    /// on this state; the longest list is what the scale must fit.
    pub usual_second_lines: usize,
    pub camps: Vec<CampState>,
}

impl ObjectivesState {
    /// The panel is the reference's head and rule, plus 24 for every line
    /// and second line. Nothing here is a fixed 302 or 398: add a line and
    /// the height follows.
    pub const fn height(lines: usize, second_lines: usize) -> i32 {
        158 + 24 * (lines + second_lines) as i32
    }

    pub fn usual_height(&self) -> i32 {
        Self::height(self.lines, self.usual_second_lines)
    }

    pub fn longest_height(&self) -> i32 {
        Self::height(self.lines, self.second_lines)
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetState {
    pub name: String,
    pub kind: String,
    pub health: u32,
    pub max_health: u32,
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

/// Everything the interface shows, as plain data. The component slices are
/// kept together here so the client has one thing to fill, not a dozen.
#[derive(Debug, Clone, PartialEq)]
pub struct HudState {
    pub score: ScoreState,
    pub ability: AbilityState,
    pub minimap: MinimapState,
    pub objectives: ObjectivesState,
    pub allies: Vec<AllyState>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinimapState {
    pub allies: usize,
    pub enemies: usize,
    pub camps: usize,
}
