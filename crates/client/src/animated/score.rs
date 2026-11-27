//! Every hero's match line for the scoreboard (roadmap 7.2): kills,
//! deaths, assists and creep score, kept by unit index (units are never
//! removed, so an index names one hero for the whole match).
//!
//! Credit follows the usual MOBA rules:
//! - The hero who lands the killing blow gets the kill.
//! - If a tower, minion or camp finishes a hero, the kill goes to the last
//!   enemy hero who hit them within [`ASSIST_WINDOW`] seconds; with none,
//!   nobody gets it (the death still counts).
//! - Every other enemy hero who hit the victim within the window gets an
//!   assist.
//! - Creep score is lane minions last-hit by the hero.

use std::collections::HashMap;

/// Seconds a hero's hit on an enemy hero keeps earning kill or assist
/// credit.
pub const ASSIST_WINDOW: f32 = 10.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HeroScore {
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub cs: u32,
}

/// A recent hit by one hero on another: victim, attacker, seconds left.
#[derive(Debug, Clone, Copy)]
struct Hit {
    victim: usize,
    attacker: usize,
    left: f32,
}

#[derive(Debug, Default)]
pub struct ScoreBook {
    scores: HashMap<usize, HeroScore>,
    /// Newest last.
    hits: Vec<Hit>,
}

impl ScoreBook {
    pub fn get(&self, unit: usize) -> HeroScore {
        self.scores.get(&unit).copied().unwrap_or_default()
    }

    fn entry(&mut self, unit: usize) -> &mut HeroScore {
        self.scores.entry(unit).or_default()
    }

    /// Hero `attacker` damaged enemy hero `victim`.
    pub fn hero_hit(&mut self, victim: usize, attacker: usize) {
        if victim == attacker {
            return;
        }
        self.hits
            .retain(|hit| !(hit.victim == victim && hit.attacker == attacker));
        self.hits.push(Hit {
            victim,
            attacker,
            left: ASSIST_WINDOW,
        });
    }

    pub fn tick(&mut self, dt: f32) {
        for hit in &mut self.hits {
            hit.left -= dt;
        }
        self.hits.retain(|hit| hit.left > 0.0);
    }

    /// Hero `victim` died. `killer` is the hero that landed the blow, if a
    /// hero did. Returns who got the kill.
    pub fn hero_died(&mut self, victim: usize, killer: Option<usize>) -> Option<usize> {
        self.entry(victim).deaths += 1;
        let recent: Vec<usize> = self
            .hits
            .iter()
            .rev()
            .filter(|hit| hit.victim == victim)
            .map(|hit| hit.attacker)
            .collect();
        let credited = killer.filter(|&k| k != victim).or(recent.first().copied());
        if let Some(k) = credited {
            self.entry(k).kills += 1;
        }
        for &helper in &recent {
            if Some(helper) != credited {
                self.entry(helper).assists += 1;
            }
        }
        // The slate is clean for the victim's next life.
        self.hits.retain(|hit| hit.victim != victim);
        credited
    }

    /// Hero `hero` last-hit a lane minion.
    pub fn minion_last_hit(&mut self, hero: usize) {
        self.entry(hero).cs += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_last_hitter_kills_and_the_other_hitters_assist() {
        let mut book = ScoreBook::default();
        book.hero_hit(9, 1);
        book.hero_hit(9, 2);
        book.hero_hit(9, 3);
        assert_eq!(book.hero_died(9, Some(2)), Some(2));
        assert_eq!(book.get(2).kills, 1);
        assert_eq!(book.get(1).assists, 1);
        assert_eq!(book.get(3).assists, 1);
        assert_eq!(book.get(2).assists, 0);
        assert_eq!(book.get(9).deaths, 1);
    }

    #[test]
    fn a_tower_kill_goes_to_the_last_hero_who_hit_within_the_window() {
        let mut book = ScoreBook::default();
        book.hero_hit(9, 1);
        book.tick(3.0);
        book.hero_hit(9, 2);
        assert_eq!(book.hero_died(9, None), Some(2));
        assert_eq!(book.get(2).kills, 1);
        assert_eq!(book.get(1).assists, 1);
    }

    #[test]
    fn old_hits_earn_nothing_and_a_lonely_death_gives_no_kill() {
        let mut book = ScoreBook::default();
        book.hero_hit(9, 1);
        book.tick(ASSIST_WINDOW + 0.1);
        assert_eq!(book.hero_died(9, None), None);
        assert_eq!(book.get(1), HeroScore::default());
        assert_eq!(book.get(9).deaths, 1);
    }

    #[test]
    fn the_slate_is_wiped_on_death() {
        let mut book = ScoreBook::default();
        book.hero_hit(9, 1);
        book.hero_died(9, Some(1));
        // Next life: hero 1's old hit doesn't assist a later kill.
        book.hero_hit(9, 2);
        book.hero_died(9, Some(2));
        assert_eq!(book.get(1).assists, 0);
        assert_eq!(book.get(1).kills, 1);
        assert_eq!(book.get(2).kills, 1);
    }

    #[test]
    fn creep_score_counts_per_hero() {
        let mut book = ScoreBook::default();
        book.minion_last_hit(4);
        book.minion_last_hit(4);
        book.minion_last_hit(5);
        assert_eq!(book.get(4).cs, 2);
        assert_eq!(book.get(5).cs, 1);
    }
}
