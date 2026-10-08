//! Combat and match rules that do not depend on rendering.
//!
//! Hits, damage-over-time, series length, and loadout constraints live here
//! so the feel of the game can be checked without opening a window.

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Guard {
    Open,
    Shield,
    Parry,
    Block,
}

/// A strike can be parried and interrupts healing. A dot cannot.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum HitKind {
    Strike,
    Dot,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum HitOutcome {
    Ignored,
    Parried,
    Blocked,
    Hit,
}

#[derive(Clone, Copy, Debug)]
pub struct Incoming {
    pub dmg: f32,
    pub kx: f32,
    pub ky: f32,
    pub kind: HitKind,
    pub guard: Guard,
    pub dmg_taken: f32,
    pub kb_resist: f32,
    pub dead: bool,
    pub invuln: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct HitEffect {
    pub outcome: HitOutcome,
    pub dmg: f32,
    pub kx: f32,
    pub ky: f32,
    /// Mend should stop when a real hit connects, including a partial block.
    pub interrupt_mend: bool,
}

pub fn resolve_hit(hit: Incoming) -> HitEffect {
    let none = HitEffect { outcome: HitOutcome::Ignored, dmg: 0.0, kx: 0.0, ky: 0.0, interrupt_mend: false };
    if hit.dead || hit.invuln || hit.guard == Guard::Shield {
        return none;
    }
    if hit.kind == HitKind::Strike && hit.guard == Guard::Parry {
        return HitEffect { outcome: HitOutcome::Parried, ..none };
    }

    let mut dmg = hit.dmg * hit.dmg_taken;
    let mut kx = hit.kx * hit.kb_resist;
    let mut ky = hit.ky * hit.kb_resist;
    let mut outcome = HitOutcome::Hit;
    if hit.guard == Guard::Block {
        dmg *= 0.3;
        if hit.kind == HitKind::Strike {
            kx *= 0.2;
            ky = 0.0;
            outcome = HitOutcome::Blocked;
        }
    }
    if hit.kind == HitKind::Dot {
        kx = 0.0;
        ky = 0.0;
    }
    HitEffect {
        outcome,
        dmg,
        kx,
        ky,
        interrupt_mend: hit.kind == HitKind::Strike && matches!(outcome, HitOutcome::Hit | HitOutcome::Blocked),
    }
}

/// Extra damage from a combo, capped, with the power pickup stacked on top.
pub fn combo_extra(drop: f32, combo: u32, powered: bool) -> f32 {
    let mut bonus = ((combo as f32 - 1.0) * 0.1).min(0.5);
    if powered {
        bonus += 0.5;
    }
    drop * bonus.max(0.0)
}

/// Best-of-N ends when someone reaches this many round wins.
pub fn wins_needed(best_of: u32) -> u32 {
    best_of / 2 + 1
}

pub fn series_over(wins: [u32; 2], round_no: u32, best_of: u32) -> bool {
    let need = wins_needed(best_of);
    wins[0] >= need || wins[1] >= need || round_no >= best_of
}

/// Step an ability slot without landing on the other slot's ability.
pub fn cycle_distinct(current: usize, other: usize, len: usize, dir: i32) -> usize {
    if len <= 1 {
        return current;
    }
    let mut v = current;
    for _ in 0..len {
        v = (v as i32 + dir).rem_euclid(len as i32) as usize;
        if v != other {
            return v;
        }
    }
    current
}

pub fn explosive_mult(bomber: bool) -> f32 {
    if bomber { 1.2 } else { 1.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strike(guard: Guard) -> Incoming {
        Incoming {
            dmg: 10.0,
            kx: 100.0,
            ky: -50.0,
            kind: HitKind::Strike,
            guard,
            dmg_taken: 1.0,
            kb_resist: 1.0,
            dead: false,
            invuln: false,
        }
    }

    #[test]
    fn shield_parry_and_block_follow_the_guard_rules() {
        assert_eq!(resolve_hit(strike(Guard::Shield)).outcome, HitOutcome::Ignored);
        assert_eq!(resolve_hit(strike(Guard::Parry)).outcome, HitOutcome::Parried);
        let blocked = resolve_hit(strike(Guard::Block));
        assert_eq!(blocked.outcome, HitOutcome::Blocked);
        assert!((blocked.dmg - 3.0).abs() < 0.001);
        assert!((blocked.kx - 20.0).abs() < 0.001);
        assert_eq!(blocked.ky, 0.0);
        assert!(blocked.interrupt_mend);
    }

    #[test]
    fn passives_scale_damage_and_knockback() {
        let mut hit = strike(Guard::Open);
        hit.dmg_taken = 0.85;
        hit.kb_resist = 0.55;
        let fx = resolve_hit(hit);
        assert!((fx.dmg - 8.5).abs() < 0.001);
        assert!((fx.kx - 55.0).abs() < 0.001);
        assert!((fx.ky - -27.5).abs() < 0.001);
    }

    #[test]
    fn dots_ignore_parry_and_carry_no_knockback() {
        let mut hit = strike(Guard::Parry);
        hit.kind = HitKind::Dot;
        let fx = resolve_hit(hit);
        assert_eq!(fx.outcome, HitOutcome::Hit);
        assert_eq!(fx.kx, 0.0);
        assert!(!fx.interrupt_mend);

        let mut blocked = strike(Guard::Block);
        blocked.kind = HitKind::Dot;
        let fx = resolve_hit(blocked);
        assert!((fx.dmg - 3.0).abs() < 0.001);
        assert_eq!(fx.ky, 0.0);
    }

    #[test]
    fn dead_and_invulnerable_squares_take_nothing() {
        let mut hit = strike(Guard::Open);
        hit.dead = true;
        assert_eq!(resolve_hit(hit).outcome, HitOutcome::Ignored);
        hit.dead = false;
        hit.invuln = true;
        assert_eq!(resolve_hit(hit).outcome, HitOutcome::Ignored);
    }

    #[test]
    fn combo_bonus_caps_and_power_stacks() {
        assert!((combo_extra(10.0, 1, false)).abs() < 0.001);
        assert!((combo_extra(10.0, 3, false) - 2.0).abs() < 0.001);
        assert!((combo_extra(10.0, 20, false) - 5.0).abs() < 0.001);
        assert!((combo_extra(10.0, 3, true) - 7.0).abs() < 0.001);
    }

    #[test]
    fn series_ends_early_once_a_side_clinches() {
        assert_eq!(wins_needed(1), 1);
        assert_eq!(wins_needed(3), 2);
        assert_eq!(wins_needed(5), 3);
        assert_eq!(wins_needed(10), 6);
        assert!(!series_over([1, 1], 2, 3));
        assert!(series_over([2, 0], 2, 3));
        assert!(series_over([0, 0], 1, 1));
        assert!(series_over([1, 1], 3, 3));
    }

    #[test]
    fn loadouts_cannot_pick_the_same_ability_twice() {
        assert_eq!(cycle_distinct(0, 1, 12, 1), 2);
        assert_eq!(cycle_distinct(0, 1, 12, -1), 11);
        assert_eq!(cycle_distinct(1, 2, 12, 1), 3);
    }

    #[test]
    fn bomber_explosives_hit_harder() {
        assert!((explosive_mult(true) - 1.2).abs() < 0.001);
        assert!((explosive_mult(false) - 1.0).abs() < 0.001);
    }
}
