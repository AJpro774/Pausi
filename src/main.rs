use raylib::prelude::*;

// ---- Alexander's movement numbers (unchanged feel) ----
const W: i32 = 1280;
const H: i32 = 720;
const SIZE: f32 = 60.0; // smaller than the solo prototype so two fit in an arena
const TOP_SPEED: f32 = 350.0; // px per second
const ICE: f32 = 400.0; // slide after letting go
const JUMP: f32 = 200.0; // normal jump height in pixels
const MAX_CHARGE: f32 = 3.5; // full charge = 3.5x jump
const GRAVITY: f32 = 800.0; // higher = snappier, lower = floatier (was 250)
const TAP_CROUCH: f32 = 0.15;
const CROUCH: f32 = 5.0;

// ---- duel numbers ----
const MAX_HP: f32 = 100.0;
const FLOOR_H: f32 = 40.0;
const DASH_SPEED: f32 = 900.0;
const DASH_TIME: f32 = 0.2;
const SLAM_SPEED: f32 = 900.0;
const BOMB_GRAVITY: f32 = 600.0;
const BLAST_R: f32 = 120.0;
const SHIELD_TIME: f32 = 1.2;
const BOOM_TIME: f32 = 0.3;
const MELEE_TIME: f32 = 0.25; // length of the knife swing
const MELEE_CD: f32 = 0.0; // no cooldown (you still finish the swing in progress)
const MELEE_REACH: f32 = 80.0;
const LAVA_DPS: f32 = 35.0;
const MAX_SPEED: f32 = 1500.0; // knockback never launches anyone faster than this
const BLINK_DIST: f32 = 260.0;
const STRIKE_DELAY: f32 = 0.9;
const FROZEN_TIME: f32 = 1.3;
const BURN_TIME: f32 = 2.5;
const KO_TIME: f32 = 1.8; // slow-motion KO cinematic
const KO_SLOW: f32 = 0.25;

// ---- movement tech ----
const WALL_SLIDE: f32 = 140.0; // max fall speed while sliding down a wall
const WALL_KICK: f32 = 430.0; // sideways push off a wall jump

// ---- combat depth ----
const BLOCK_MAX: f32 = 1.6; // longest you can hold a guard
const PARRY_WINDOW: f32 = 0.16; // block right as a hit lands to parry it
const ULT_MAX: f32 = 100.0;
const COMBO_WINDOW: f32 = 1.8;

// ---- modes ----
const TIME_LIMIT: f32 = 60.0;
const HILL_TIME: f32 = 90.0;
const HILL_TARGET: f32 = 30.0;
const HILL_MOVE: f32 = 15.0;
const STOCKS: i32 = 3;
const RESPAWN_TIME: f32 = 1.6;

// =====================================================================
// small helpers
// =====================================================================

struct Rng(u32);
impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 % 10000) as f32 / 10000.0
    }
    fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.next()
    }
}

fn cycle(v: usize, n: usize, dir: i32) -> usize {
    (v as i32 + dir).rem_euclid(n as i32) as usize
}

fn with_alpha(c: Color, a: u8) -> Color {
    Color::new(c.r, c.g, c.b, a)
}

fn hsv(h: f32) -> Color {
    let h = h.rem_euclid(360.0) / 60.0;
    let x = 1.0 - (h % 2.0 - 1.0).abs();
    let (r, g, b) = match h as i32 {
        0 => (1.0, x, 0.0),
        1 => (x, 1.0, 0.0),
        2 => (0.0, 1.0, x),
        3 => (0.0, x, 1.0),
        4 => (x, 0.0, 1.0),
        _ => (1.0, 0.0, x),
    };
    Color::new((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, 255)
}

fn overlaps(a: &Rectangle, b: &Rectangle) -> bool {
    a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y
}

fn ease(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

// raylib wants triangles wound counter-clockwise, so fix the order for it
fn tri(d: &mut impl RaylibDraw, a: Vector2, b: Vector2, c: Vector2, col: Color) {
    let cross = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    if cross > 0.0 {
        d.draw_triangle(a, c, b, col);
    } else {
        d.draw_triangle(a, b, c, col);
    }
}

// =====================================================================
// characters, abilities, trails
// =====================================================================

#[derive(Clone, Copy, PartialEq)]
enum Eyes {
    Forward,
    Left,
    Right,
    UpWide,
    Squint,
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Dasher,
    Bomber,
    Shielder,
    Titan,
    Ghost,
    Spark,
}

const KINDS: [Kind; 6] = [Kind::Dasher, Kind::Bomber, Kind::Shielder, Kind::Titan, Kind::Ghost, Kind::Spark];

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Kind::Dasher => "DASHER",
            Kind::Bomber => "BOMBER",
            Kind::Shielder => "SHIELDER",
            Kind::Titan => "TITAN",
            Kind::Ghost => "GHOST",
            Kind::Spark => "SPARK",
        }
    }
    fn color(self) -> Color {
        match self {
            Kind::Dasher => Color::ORANGE,
            Kind::Bomber => Color::new(220, 60, 60, 255),
            Kind::Shielder => Color::new(40, 170, 160, 255),
            Kind::Titan => Color::new(110, 120, 160, 255),
            Kind::Ghost => Color::new(225, 220, 255, 255),
            Kind::Spark => Color::new(255, 230, 60, 255),
        }
    }
    // indices into ABILITIES
    fn default_loadout(self) -> [usize; 2] {
        match self {
            Kind::Dasher => [0, 1],
            Kind::Bomber => [2, 3],
            Kind::Shielder => [4, 5],
            Kind::Titan => [1, 5],
            Kind::Ghost => [6, 7],
            Kind::Spark => [8, 0],
        }
    }
    // ---- passive traits ----
    fn max_hp(self) -> f32 {
        match self {
            Kind::Titan => 130.0,
            Kind::Ghost => 85.0,
            Kind::Spark => 90.0,
            _ => MAX_HP,
        }
    }
    fn speed(self) -> f32 {
        match self {
            Kind::Dasher => 1.1,
            Kind::Titan => 0.85,
            Kind::Spark => 1.25,
            _ => 1.0,
        }
    }
    fn kb_resist(self) -> f32 {
        if self == Kind::Titan {
            0.55
        } else {
            1.0
        }
    }
    fn dmg_taken(self) -> f32 {
        if self == Kind::Shielder {
            0.85
        } else {
            1.0
        }
    }
    fn grav(self) -> f32 {
        match self {
            Kind::Ghost => 0.75,
            Kind::Titan => 1.1,
            _ => 1.0,
        }
    }
    fn air_jumps(self) -> i32 {
        match self {
            Kind::Titan => 0,
            Kind::Ghost => 2,
            _ => 1,
        }
    }
    fn melee_dmg(self) -> f32 {
        match self {
            Kind::Spark => 14.0,
            Kind::Titan => 12.0,
            _ => 10.0,
        }
    }
    fn passive(self) -> &'static str {
        match self {
            Kind::Dasher => "PASSIVE: +10% speed",
            Kind::Bomber => "PASSIVE: bombs hit 20% harder",
            Kind::Shielder => "PASSIVE: takes 15% less damage",
            Kind::Titan => "PASSIVE: 130 HP, resists knockback, slow, no double jump",
            Kind::Ghost => "PASSIVE: 85 HP, floaty, two air jumps",
            Kind::Spark => "PASSIVE: 90 HP, +25% speed, stronger knife",
        }
    }
    fn ult_name(self) -> &'static str {
        match self {
            Kind::Dasher => "OVERDRIVE",
            Kind::Bomber => "CARPET BOMB",
            Kind::Shielder => "AEGIS NOVA",
            Kind::Titan => "EARTHQUAKE",
            Kind::Ghost => "DEEP FREEZE",
            Kind::Spark => "LIGHTNING STORM",
        }
    }
    fn ult_blurb(self) -> &'static str {
        match self {
            Kind::Dasher => "6s faster, cooldowns x3, knife x1.8",
            Kind::Bomber => "bombs rain across the arena",
            Kind::Shielder => "3s invincible + blast wave",
            Kind::Titan => "foe on the ground: 25 dmg + launch",
            Kind::Ghost => "freeze them 2s from anywhere",
            Kind::Spark => "5 bolts strike around them",
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Ability {
    Dash,
    Slam,
    Bomb,
    Hop,
    Shield,
    Pulse,
    Blink,
    Frost,
    Thunder,
    Magnet,
    Mend,
    Fireball,
}

const ABILITIES: [Ability; 12] = [
    Ability::Dash,
    Ability::Slam,
    Ability::Bomb,
    Ability::Hop,
    Ability::Shield,
    Ability::Pulse,
    Ability::Blink,
    Ability::Frost,
    Ability::Thunder,
    Ability::Magnet,
    Ability::Mend,
    Ability::Fireball,
];

impl Ability {
    fn name(self) -> &'static str {
        match self {
            Ability::Dash => "DASH",
            Ability::Slam => "SLAM",
            Ability::Bomb => "BOMB",
            Ability::Hop => "HOP",
            Ability::Shield => "SHIELD",
            Ability::Pulse => "PULSE",
            Ability::Blink => "BLINK",
            Ability::Frost => "FROST",
            Ability::Thunder => "THUNDER",
            Ability::Magnet => "MAGNET",
            Ability::Mend => "MEND",
            Ability::Fireball => "FIREBALL",
        }
    }
    fn blurb(self) -> &'static str {
        match self {
            Ability::Dash => "ram forward, 15 dmg",
            Ability::Slam => "leap and crash down, 20 dmg",
            Ability::Bomb => "thrown, 25 dmg, launches you",
            Ability::Hop => "boost upward, even in the air",
            Ability::Shield => "blocks damage, reflects shots",
            Ability::Pulse => "12 dmg, big knockback",
            Ability::Blink => "teleport forward past danger",
            Ability::Frost => "freezes them for 1.3s, 5 dmg",
            Ability::Thunder => "lightning on their spot, 22 dmg",
            Ability::Magnet => "yank them toward you",
            Ability::Mend => "heal 18 HP over 2 seconds",
            Ability::Fireball => "14 dmg and burns for 2.5s",
        }
    }
    fn cooldown(self) -> f32 {
        match self {
            Ability::Dash => 2.0,
            Ability::Slam => 3.0,
            Ability::Bomb => 1.5,
            Ability::Hop => 2.5,
            Ability::Shield => 3.0,
            Ability::Pulse => 1.5,
            Ability::Blink => 2.5,
            Ability::Frost => 3.5,
            Ability::Thunder => 5.0,
            Ability::Magnet => 3.5,
            Ability::Mend => 9.0,
            Ability::Fireball => 2.0,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Trail {
    Off,
    Dots,
    Ghost,
    Sparkle,
    Smoke,
}

const TRAIL_STYLES: [Trail; 5] = [Trail::Off, Trail::Dots, Trail::Ghost, Trail::Sparkle, Trail::Smoke];

impl Trail {
    fn name(self) -> &'static str {
        match self {
            Trail::Off => "NONE",
            Trail::Dots => "DOTS",
            Trail::Ghost => "GHOST",
            Trail::Sparkle => "SPARKLE",
            Trail::Smoke => "SMOKE",
        }
    }
}

const TRAIL_COLOR_NAMES: [&str; 9] = ["BODY", "WHITE", "RED", "YELLOW", "GREEN", "CYAN", "PURPLE", "PINK", "RAINBOW"];

fn trail_color(idx: usize, t: f32, body: Color, spread: f32) -> Color {
    match idx {
        0 => body,
        1 => Color::WHITE,
        2 => Color::new(235, 50, 50, 255),
        3 => Color::new(255, 230, 40, 255),
        4 => Color::new(60, 220, 70, 255),
        5 => Color::new(0, 220, 255, 255),
        6 => Color::new(160, 80, 230, 255),
        7 => Color::new(255, 120, 190, 255),
        _ => hsv(t * 200.0 + spread),
    }
}

#[derive(Clone, Copy)]
struct Setup {
    kind: usize,
    abil: [usize; 2],
    trail: usize,
    color: usize,
    death: usize,   // index into DEATH_NAMES (10 = random)
    victory: usize, // index into VICTORY_NAMES (10 = random)
}

impl Setup {
    fn new(kind: usize) -> Self {
        Setup { kind, abil: KINDS[kind].default_loadout(), trail: 1, color: 0, death: 0, victory: 0 }
    }
}

// =====================================================================
// game modes
// =====================================================================

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Classic,
    Timed,
    Stock,
    Hill,
}

const MODES: [Mode; 4] = [Mode::Classic, Mode::Timed, Mode::Stock, Mode::Hill];

impl Mode {
    fn name(self) -> &'static str {
        match self {
            Mode::Classic => "CLASSIC",
            Mode::Timed => "TIMED",
            Mode::Stock => "STOCK",
            Mode::Hill => "KING OF THE HILL",
        }
    }
    fn blurb(self) -> &'static str {
        match self {
            Mode::Classic => "a round is won when the other square is knocked out",
            Mode::Timed => "60 second clock - KO wins, otherwise most HP wins",
            Mode::Stock => "3 lives per round, you respawn until they run out",
            Mode::Hill => "stand in the glowing zone to score, 30 points or most in 90s",
        }
    }
}

// =====================================================================
// effects: particles, rings, screen shake, hit-stop ("cinematics")
// =====================================================================

#[derive(Clone, Copy, PartialEq)]
enum Shape {
    Circle,
    Ghost,
    Star,
    Streak,
    Square,
}

struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: f32,
    max: f32,
    size: f32,
    rot: f32,
    color: Color,
    shape: Shape,
    grow: bool,
    grav: f32,
    spin: f32, // degrees per second
}

impl Particle {
    fn new(x: f32, y: f32, vx: f32, vy: f32, life: f32, size: f32, color: Color, shape: Shape) -> Self {
        Particle { x, y, vx, vy, life, max: life, size, rot: 0.0, color, shape, grow: false, grav: 0.0, spin: 0.0 }
    }
}

struct Boom {
    x: f32,
    y: f32,
    r: f32,
    t: f32,
    color: Color,
}

enum ProjKind {
    Bomb,
    Frost,
    Fire,
}

struct Proj {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    owner: usize,
    life: f32,
    kind: ProjKind,
}

struct Strike {
    x: f32,
    t: f32,
    owner: usize,
    struck: bool,
    vis: f32,
    pts: Vec<Vector2>,
}

// arena meteor: shows a warning, then falls diagonally and explodes
struct Meteor {
    tx: f32,
    warn: f32,
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    falling: bool,
}

// neon arcade: a horizontal laser beam that warns, then sweeps the whole screen
struct Laser {
    y: f32,
    t: f32,
    fired: bool,
    vis: f32,
}

#[derive(Clone, Copy, PartialEq)]
enum PickKind {
    Health,
    Power,
    Haste,
    Reset,
}

struct Pickup {
    x: f32,
    y: f32,
    kind: PickKind,
    age: f32,
}

impl PickKind {
    fn color(self) -> Color {
        match self {
            PickKind::Health => Color::new(70, 220, 90, 255),
            PickKind::Power => Color::new(235, 70, 60, 255),
            PickKind::Haste => Color::new(60, 200, 255, 255),
            PickKind::Reset => Color::new(255, 220, 70, 255),
        }
    }
    fn label(self) -> &'static str {
        match self {
            PickKind::Health => "+",
            PickKind::Power => "P",
            PickKind::Haste => ">",
            PickKind::Reset => "C",
        }
    }
}

struct Fx {
    shake: f32,   // screen shake in pixels
    hitstop: f32, // freeze-frame seconds
    punch: f32,   // quick camera zoom punch
    flash: f32,   // white screen flash 0..1
}

struct World {
    projs: Vec<Proj>,
    booms: Vec<Boom>,
    parts: Vec<Particle>,
    strikes: Vec<Strike>,
    meteors: Vec<Meteor>,
    lasers: Vec<Laser>,
    pickups: Vec<Pickup>,
    banner: Option<(String, Color, f32)>, // big ultimate / event text
    grav_scale: f32,                      // map events can make gravity lighter
    rng: Rng,
    fx: Fx,
}

impl World {
    fn new() -> Self {
        World {
            projs: Vec::new(),
            booms: Vec::new(),
            parts: Vec::new(),
            strikes: Vec::new(),
            meteors: Vec::new(),
            lasers: Vec::new(),
            pickups: Vec::new(),
            banner: None,
            grav_scale: 1.0,
            rng: Rng(2463534242),
            fx: Fx { shake: 0.0, hitstop: 0.0, punch: 0.0, flash: 0.0 },
        }
    }

    fn clear(&mut self) {
        self.projs.clear();
        self.booms.clear();
        self.parts.clear();
        self.strikes.clear();
        self.meteors.clear();
        self.lasers.clear();
        self.pickups.clear();
        self.banner = None;
        self.grav_scale = 1.0;
        self.fx = Fx { shake: 0.0, hitstop: 0.0, punch: 0.0, flash: 0.0 };
    }

    fn ring(&mut self, x: f32, y: f32, r: f32, color: Color) {
        self.booms.push(Boom { x, y, r, t: 0.0, color });
    }

    fn shake(&mut self, amount: f32) {
        self.fx.shake = self.fx.shake.max(amount);
    }

    // a burst of particles flying out in all directions
    #[allow(clippy::too_many_arguments)]
    fn burst(&mut self, x: f32, y: f32, n: usize, col: Color, speed: f32, life: f32, size: f32, shape: Shape, grav: f32) {
        if self.parts.len() > 1600 {
            return;
        }
        for _ in 0..n {
            let a = self.rng.range(0.0, 6.2832);
            let s = speed * self.rng.range(0.3, 1.0);
            let mut q = Particle::new(
                x,
                y,
                a.cos() * s,
                a.sin() * s,
                life * self.rng.range(0.6, 1.0),
                size * self.rng.range(0.6, 1.2),
                col,
                shape,
            );
            q.grav = grav;
            q.rot = self.rng.range(0.0, 90.0);
            self.parts.push(q);
        }
    }
}

fn update_particles(parts: &mut Vec<Particle>, dt: f32) {
    for q in parts.iter_mut() {
        q.vy += q.grav * dt;
        q.x += q.vx * dt;
        q.y += q.vy * dt;
        q.rot += q.spin * dt;
        q.life -= dt;
    }
    parts.retain(|q| q.life > 0.0);
}

fn update_booms(booms: &mut Vec<Boom>, dt: f32) {
    for b in booms.iter_mut() {
        b.t += dt;
    }
    booms.retain(|b| b.t < BOOM_TIME);
}

fn draw_particles(d: &mut impl RaylibDraw, parts: &[Particle]) {
    for q in parts {
        let a = (q.life / q.max).clamp(0.0, 1.0);
        let col = with_alpha(q.color, (q.color.a as f32 * a) as u8);
        match q.shape {
            Shape::Circle => {
                let r = if q.grow { q.size * (1.0 + (1.0 - a)) } else { q.size * (0.4 + 0.6 * a) };
                d.draw_circle(q.x as i32, q.y as i32, r, col);
            }
            Shape::Ghost => {
                d.draw_rectangle_pro(
                    Rectangle::new(q.x, q.y, SIZE, SIZE),
                    Vector2::new(SIZE / 2.0, SIZE / 2.0),
                    q.rot,
                    col,
                );
            }
            Shape::Square => {
                let s = q.size * (0.5 + 0.5 * a);
                d.draw_rectangle_pro(Rectangle::new(q.x, q.y, s, s), Vector2::new(s / 2.0, s / 2.0), q.rot + q.life * 300.0, col);
            }
            Shape::Streak => {
                d.draw_line_ex(
                    Vector2::new(q.x, q.y),
                    Vector2::new(q.x - q.vx * 0.06, q.y - q.vy * 0.06),
                    q.size,
                    col,
                );
            }
            Shape::Star => {
                let s = q.size * (0.5 + 0.5 * a);
                for rot in [0.0, 45.0] {
                    d.draw_rectangle_pro(Rectangle::new(q.x, q.y, s * 2.0, 3.0), Vector2::new(s, 1.5), rot, col);
                    d.draw_rectangle_pro(Rectangle::new(q.x, q.y, 3.0, s * 2.0), Vector2::new(1.5, s), rot, col);
                }
            }
        }
    }
}

// =====================================================================
// input (humans read the keyboard, the CPU builds the same struct)
// =====================================================================

#[derive(Clone, Copy)]
struct Keys {
    left: KeyboardKey,
    right: KeyboardKey,
    up: KeyboardKey,
    down: KeyboardKey,
    a1: KeyboardKey,
    a2: KeyboardKey,
    melee: KeyboardKey,
    block: KeyboardKey,
    ult: KeyboardKey,
}

#[derive(Clone, Copy, Default)]
struct Input {
    left: bool,
    right: bool,
    down: bool,
    jump: bool,
    a1: bool,
    a2: bool,
    melee: bool,
    block_down: bool,
    ult: bool,
}

fn read_input(rl: &RaylibHandle, k: &Keys) -> Input {
    Input {
        left: rl.is_key_down(k.left),
        right: rl.is_key_down(k.right),
        down: rl.is_key_down(k.down),
        jump: rl.is_key_pressed(k.up),
        a1: rl.is_key_pressed(k.a1),
        a2: rl.is_key_pressed(k.a2),
        melee: rl.is_key_pressed(k.melee),
        block_down: rl.is_key_down(k.block),
        ult: rl.is_key_pressed(k.ult),
    }
}

// =====================================================================
// CPU personalities
// =====================================================================

// every number is a per-frame chance or a distance, so a profile is just a "feel"
struct Profile {
    dist: f32,   // preferred fighting distance
    flee: f32,   // chance to back off when the foe is too close
    melee: f32,  // knife swings when in reach
    abil: f32,   // chance to fire a ready ability
    block: f32,  // chance to guard when the foe attacks
    jump: f32,   // random hopping
    ult: f32,    // chance to cash in a full ultimate
    react: f32,  // 1.0 = acts every frame, lower = slow reactions
    smart: f32,  // how often it checks an ability makes sense before using it
    wobble: f32, // erratic strafing
    range: f32,  // how far away it will use abilities
}

const CPU_NAMES: [&str; 9] = [
    "BALANCED",
    "AGGRESSOR",
    "DEFENDER",
    "SNIPER",
    "TRICKSTER",
    "BERSERKER",
    "ROOKIE",
    "PRO",
    "RANDOM",
];

const CPU_BLURBS: [&str; 9] = [
    "a bit of everything",
    "closes in fast and loves the knife",
    "guards and parries, then counters",
    "keeps its distance and fires abilities",
    "erratic, hops around, unpredictable",
    "relentless, no defence, spams everything",
    "slow reactions, easy to beat",
    "sharp spacing, smart abilities, hard to hit",
    "a different personality each round",
];

fn cpu_profile(idx: usize) -> Profile {
    let p = |dist, flee, melee, abil, block, jump, ult, react, smart, wobble, range| Profile {
        dist,
        flee,
        melee,
        abil,
        block,
        jump,
        ult,
        react,
        smart,
        wobble,
        range,
    };
    match idx {
        1 => p(70.0, 0.05, 0.25, 0.05, 0.15, 0.006, 0.08, 1.0, 0.5, 0.1, 450.0), // AGGRESSOR
        2 => p(160.0, 0.5, 0.08, 0.025, 0.9, 0.003, 0.05, 1.0, 0.9, 0.05, 450.0), // DEFENDER
        3 => p(380.0, 0.9, 0.06, 0.07, 0.4, 0.01, 0.07, 1.0, 0.9, 0.3, 800.0),   // SNIPER
        4 => p(130.0, 0.4, 0.15, 0.06, 0.3, 0.02, 0.1, 1.0, 0.4, 0.8, 520.0),    // TRICKSTER
        5 => p(50.0, 0.0, 0.35, 0.08, 0.0, 0.008, 0.15, 1.0, 0.3, 0.2, 500.0),   // BERSERKER
        6 => p(130.0, 0.1, 0.05, 0.015, 0.1, 0.004, 0.02, 0.35, 0.2, 0.4, 420.0), // ROOKIE
        7 => p(100.0, 0.5, 0.2, 0.06, 0.8, 0.005, 0.1, 1.0, 1.0, 0.15, 560.0),   // PRO
        _ => p(110.0, 0.3, 0.12, 0.03, 0.5, 0.004, 0.05, 1.0, 0.7, 0.1, 520.0),  // BALANCED
    }
}

// would using this ability right now make sense?
fn ability_ok(ab: Ability, me: &Player, foe: &Player, dist: f32, foe_attacking: bool) -> bool {
    match ab {
        Ability::Mend => me.hp < me.max_hp - 25.0,
        Ability::Shield => foe_attacking || dist < 160.0,
        Ability::Pulse => dist < 190.0,
        Ability::Dash => dist > 90.0 && dist < 420.0,
        Ability::Magnet => dist > 220.0,
        Ability::Slam => dist < 300.0,
        Ability::Blink => dist > 200.0 || foe_attacking,
        Ability::Hop => foe.y + 40.0 < me.y || foe_attacking,
        Ability::Bomb | Ability::Frost | Ability::Fireball => dist > 130.0,
        Ability::Thunder => true,
    }
}

// the CPU brain: reads the same game state a player sees and builds an Input
fn bot_input(me: &Player, foe: &Player, rng: &mut Rng, prof: &Profile, t: f32, seed: f32) -> Input {
    let mut i = Input::default();
    let (mx, my) = me.center();
    let (fx, fy) = foe.center();
    let dx = fx - mx;
    let dy = fy - my;
    let dist = dx.abs();
    let foe_attacking = foe.melee_t > 0.0 || foe.dash_t > 0.0 || foe.slam;
    let react = rng.next() < prof.react; // slow profiles skip decisions some frames

    // ---- movement: hold the preferred distance ----
    let gap = dist - prof.dist;
    let mut want = if gap > 30.0 {
        dx.signum()
    } else if gap < -30.0 && rng.next() < prof.flee {
        -dx.signum()
    } else {
        0.0
    };
    // erratic profiles sometimes strafe the wrong way
    if prof.wobble > 0.0 && (t * 1.7 + seed).sin() > 1.0 - prof.wobble * 1.5 {
        want = -want;
    }
    if want > 0.0 {
        i.right = true;
    } else if want < 0.0 {
        i.left = true;
    }

    // ---- jumping: reach the foe, climb walls, double jump ----
    if me.on_ground {
        if dy < -80.0 && rng.next() < 0.06 {
            i.jump = true;
        } else if me.vx.abs() < 5.0 && want != 0.0 && rng.next() < 0.1 {
            i.jump = true;
        } else if rng.next() < prof.jump {
            i.jump = true;
        }
    } else if me.wall_dir != 0.0 && rng.next() < 0.1 {
        i.jump = true;
    } else if me.air_jumps > 0 && dy < -120.0 && me.vy > 0.0 && rng.next() < 0.05 {
        i.jump = true;
    }

    // ---- attacking ----
    if react {
        if dist < 95.0 && dy.abs() < 80.0 && rng.next() < prof.melee {
            i.melee = true;
        }
        let loadout = me.abilities;
        let in_range = dist < prof.range;
        let wants = |ab: Ability, rng: &mut Rng| -> bool {
            in_range && (rng.next() >= prof.smart || ability_ok(ab, me, foe, dist, foe_attacking))
        };
        if me.cd1 <= 0.0 && rng.next() < prof.abil && wants(loadout[0], rng) {
            i.a1 = true;
        }
        if me.cd2 <= 0.0 && rng.next() < prof.abil && wants(loadout[1], rng) {
            i.a2 = true;
        }
        if me.ult >= ULT_MAX && dist < 650.0 && rng.next() < prof.ult {
            i.ult = true;
        }
    }

    // ---- defending ----
    if foe_attacking && dist < 170.0 && rng.next() < prof.block * if react { 1.0 } else { 0.4 } {
        i.block_down = true;
    }
    i
}

// =====================================================================
// players and world
// =====================================================================

struct Player {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    on_ground: bool,
    rot: f32,
    charge: f32,
    crouch: f32,
    tap_timer: f32,
    facing: f32,
    eyes: Eyes,
    hp: f32,
    max_hp: f32,
    kind: Kind,
    abilities: [Ability; 2],
    trail_style: usize,
    trail_color: usize,
    trail_acc: f32,
    #[allow(dead_code)]
    keys: Keys,
    cd1: f32,
    cd2: f32,
    dash_t: f32,
    dash_hit: bool,
    slam: bool,
    slam_arm: bool,
    shield_t: f32,
    hurt_t: f32,
    melee_t: f32,
    melee_cd: f32,
    melee_hit: bool,
    frozen_t: f32,
    burn_t: f32,
    regen_t: f32,
    stun_t: f32,
    // movement tech
    wall_dir: f32,
    air_jumps: i32,
    // guard / parry
    block_held: bool,
    block_prev: bool,
    block_t: f32,
    block_cd: f32,
    parry_t: f32,
    parried: bool,
    blocked: bool,
    // ultimate and buffs
    ult: f32,
    over_t: f32,
    power_t: f32,
    haste_t: f32,
    dead_t: f32,
    invuln: bool, // the champion during a victory cutscene
}

impl Player {
    fn new(setup: &Setup, x: f32, facing: f32, keys: Keys) -> Self {
        let kind = KINDS[setup.kind];
        let mut p = Player {
            x,
            y: H as f32 - FLOOR_H - SIZE,
            vx: 0.0,
            vy: 0.0,
            on_ground: true,
            rot: 0.0,
            charge: 0.0,
            crouch: 0.0,
            tap_timer: 0.0,
            facing,
            eyes: Eyes::Forward,
            hp: kind.max_hp(),
            max_hp: kind.max_hp(),
            kind,
            abilities: [ABILITIES[setup.abil[0]], ABILITIES[setup.abil[1]]],
            trail_style: setup.trail,
            trail_color: setup.color,
            trail_acc: 0.0,
            keys,
            cd1: 0.0,
            cd2: 0.0,
            dash_t: 0.0,
            dash_hit: false,
            slam: false,
            slam_arm: false,
            shield_t: 0.0,
            hurt_t: 0.0,
            melee_t: 0.0,
            melee_cd: 0.0,
            melee_hit: false,
            frozen_t: 0.0,
            burn_t: 0.0,
            regen_t: 0.0,
            stun_t: 0.0,
            wall_dir: 0.0,
            air_jumps: kind.air_jumps(),
            block_held: false,
            block_prev: false,
            block_t: 0.0,
            block_cd: 0.0,
            parry_t: 0.0,
            parried: false,
            blocked: false,
            ult: 0.0,
            over_t: 0.0,
            power_t: 0.0,
            haste_t: 0.0,
            dead_t: 0.0,
            invuln: false,
        };
        p.apply_setup(setup);
        p
    }

    fn apply_setup(&mut self, s: &Setup) {
        self.kind = KINDS[s.kind];
        self.abilities = [ABILITIES[s.abil[0]], ABILITIES[s.abil[1]]];
        self.trail_style = s.trail;
        self.trail_color = s.color;
    }

    fn rect(&self) -> Rectangle {
        Rectangle::new(self.x, self.y, SIZE, SIZE)
    }

    fn center(&self) -> (f32, f32) {
        (self.x + SIZE / 2.0, self.y + SIZE / 2.0)
    }

    fn speed_mult(&self) -> f32 {
        let mut m = self.kind.speed();
        if self.haste_t > 0.0 {
            m *= 1.4;
        }
        if self.over_t > 0.0 {
            m *= 1.5;
        }
        if self.block_held {
            m *= 0.5;
        }
        m
    }

    // damage + knockback. Shield blocks everything, a well-timed block parries,
    // a held block takes a fraction of the hit and no knockback.
    fn hurt(&mut self, dmg: f32, kx: f32, ky: f32) {
        if self.dead_t > 0.0 || self.shield_t > 0.0 || self.invuln {
            return;
        }
        if self.parry_t > 0.0 {
            self.parry_t = 0.0;
            self.parried = true;
            return;
        }
        let mut dmg = dmg * self.kind.dmg_taken();
        let mut kx = kx * self.kind.kb_resist();
        let mut ky = ky * self.kind.kb_resist();
        if self.block_held {
            dmg *= 0.3;
            kx *= 0.2;
            ky = 0.0;
            self.blocked = true;
        }
        self.hp = (self.hp - dmg).max(0.0);
        self.vx = (self.vx + kx).clamp(-MAX_SPEED, MAX_SPEED);
        if ky != 0.0 {
            self.vy = ky.clamp(-MAX_SPEED, MAX_SPEED);
            self.on_ground = false;
        }
        self.hurt_t = 0.25;
    }
}

// everything you can stand on is fully solid: you collide from any direction
struct Solid {
    r: Rectangle,
}

#[derive(Clone, Copy, PartialEq)]
enum Theme {
    Meadow,
    Skyline,
    Volcano,
    Frozen,
    Space,
    Sea,
    Sand,
    Neon,
}

const THEMES: [Theme; 8] = [
    Theme::Meadow,
    Theme::Skyline,
    Theme::Volcano,
    Theme::Frozen,
    Theme::Space,
    Theme::Sea,
    Theme::Sand,
    Theme::Neon,
];

impl Theme {
    fn name(self) -> &'static str {
        match self {
            Theme::Meadow => "SUNNY MEADOW",
            Theme::Skyline => "NIGHT SKYLINE",
            Theme::Volcano => "VOLCANO",
            Theme::Frozen => "FROZEN PEAKS",
            Theme::Space => "DEEP SPACE",
            Theme::Sea => "DEEP SEA",
            Theme::Sand => "SAND RUINS",
            Theme::Neon => "NEON ARCADE",
        }
    }
    fn blurb(self) -> &'static str {
        match self {
            Theme::Meadow => "open hills, wind gusts push everyone",
            Theme::Skyline => "rooftops and towers, meteors fall",
            Theme::Volcano => "lava surges and meteors - stay off the floor!",
            Theme::Frozen => "icy ledges, blizzards shove you around",
            Theme::Space => "zero-g bursts and falling comets",
            Theme::Sea => "floaty water and shifting currents",
            Theme::Sand => "crumbling ruins, rocks fall from the sky",
            Theme::Neon => "sweeping laser beams - watch the warnings",
        }
    }
}

struct Map {
    solids: Vec<Solid>,
    hazard: Option<Rectangle>, // lava: damages while you touch it
    zones: Vec<Rectangle>,     // king of the hill spots
    spawns: [f32; 2],
}

fn make_map(theme: Theme) -> Map {
    let floor_y = H as f32 - FLOOR_H;
    let b = |x: f32, y: f32, w: f32, h: f32| Solid { r: Rectangle::new(x, y, w, h) };
    let z = |x: f32, y: f32, w: f32, h: f32| Rectangle::new(x, y, w, h);
    let floor = b(0.0, floor_y, W as f32, FLOOR_H);
    match theme {
        Theme::Meadow => Map {
            solids: vec![
                floor,
                b(590.0, floor_y - 70.0, 100.0, 70.0), // center block
                b(0.0, 300.0, 70.0, 30.0),             // wall ledges
                b(W as f32 - 70.0, 300.0, 70.0, 30.0),
                b(140.0, 520.0, 240.0, 20.0),
                b(900.0, 520.0, 240.0, 20.0),
                b(520.0, 420.0, 240.0, 20.0),
                b(200.0, 330.0, 200.0, 20.0),
                b(880.0, 330.0, 200.0, 20.0),
            ],
            hazard: None,
            zones: vec![z(520.0, 340.0, 240.0, 80.0), z(140.0, 440.0, 240.0, 80.0), z(900.0, 440.0, 240.0, 80.0)],
            spawns: [120.0, W as f32 - 120.0 - SIZE],
        },
        Theme::Skyline => Map {
            solids: vec![
                floor,
                b(0.0, 560.0, 170.0, 120.0), // left and right buildings
                b(W as f32 - 170.0, 560.0, 170.0, 120.0),
                b(400.0, 500.0, 140.0, 180.0), // twin towers
                b(740.0, 500.0, 140.0, 180.0),
                b(560.0, 400.0, 160.0, 20.0), // floating billboards
                b(190.0, 400.0, 180.0, 20.0),
                b(910.0, 400.0, 180.0, 20.0),
                b(560.0, 270.0, 160.0, 20.0),
            ],
            hazard: None,
            zones: vec![z(560.0, 320.0, 160.0, 80.0), z(400.0, 420.0, 140.0, 80.0), z(190.0, 320.0, 180.0, 80.0)],
            spawns: [210.0, W as f32 - 210.0 - SIZE],
        },
        Theme::Volcano => Map {
            solids: vec![
                floor,
                b(0.0, 440.0, 50.0, 240.0), // rock walls
                b(W as f32 - 50.0, 440.0, 50.0, 240.0),
                b(560.0, 520.0, 160.0, 20.0), // stepping stones over the lava
                b(120.0, 560.0, 200.0, 20.0),
                b(960.0, 560.0, 200.0, 20.0),
                b(330.0, 400.0, 160.0, 20.0),
                b(790.0, 400.0, 160.0, 20.0),
                b(560.0, 280.0, 160.0, 20.0),
            ],
            hazard: Some(Rectangle::new(470.0, floor_y - 10.0, 340.0, 14.0)),
            zones: vec![z(560.0, 440.0, 160.0, 80.0), z(330.0, 320.0, 160.0, 80.0), z(790.0, 320.0, 160.0, 80.0)],
            spawns: [120.0, W as f32 - 120.0 - SIZE],
        },
        Theme::Frozen => Map {
            solids: vec![
                floor,
                b(590.0, floor_y - 70.0, 100.0, 70.0), // center ice block
                b(300.0, 560.0, 120.0, 120.0),         // ice pillars
                b(860.0, 560.0, 120.0, 120.0),
                b(120.0, 470.0, 200.0, 20.0),
                b(960.0, 470.0, 200.0, 20.0),
                b(540.0, 400.0, 200.0, 20.0),
                b(300.0, 300.0, 160.0, 20.0),
                b(820.0, 300.0, 160.0, 20.0),
            ],
            hazard: None,
            zones: vec![z(540.0, 320.0, 200.0, 80.0), z(120.0, 390.0, 200.0, 80.0), z(960.0, 390.0, 200.0, 80.0)],
            spawns: [100.0, W as f32 - 100.0 - SIZE],
        },
        Theme::Space => Map {
            solids: vec![
                floor,
                b(600.0, 600.0, 80.0, 80.0), // central station block
                b(100.0, 540.0, 180.0, 20.0),
                b(1000.0, 540.0, 180.0, 20.0),
                b(300.0, 430.0, 160.0, 20.0),
                b(820.0, 430.0, 160.0, 20.0),
                b(560.0, 340.0, 160.0, 20.0),
                b(120.0, 300.0, 140.0, 20.0),
                b(1020.0, 300.0, 140.0, 20.0),
            ],
            hazard: None,
            zones: vec![z(560.0, 260.0, 160.0, 80.0), z(300.0, 350.0, 160.0, 80.0), z(820.0, 350.0, 160.0, 80.0)],
            spawns: [120.0, W as f32 - 120.0 - SIZE],
        },
        Theme::Sea => Map {
            solids: vec![
                floor,
                b(200.0, 590.0, 160.0, 90.0), // coral mounds
                b(920.0, 590.0, 160.0, 90.0),
                b(540.0, 560.0, 200.0, 120.0), // shipwreck hull
                b(580.0, 480.0, 120.0, 20.0),  // ship's deck
                b(80.0, 430.0, 180.0, 20.0),
                b(1020.0, 430.0, 180.0, 20.0),
                b(330.0, 360.0, 160.0, 20.0),
                b(790.0, 360.0, 160.0, 20.0),
            ],
            hazard: None,
            zones: vec![z(580.0, 400.0, 120.0, 80.0), z(330.0, 280.0, 160.0, 80.0), z(790.0, 280.0, 160.0, 80.0)],
            spawns: [100.0, W as f32 - 100.0 - SIZE],
        },
        Theme::Sand => Map {
            solids: vec![
                floor,
                b(540.0, 620.0, 200.0, 60.0), // stepped pyramid
                b(580.0, 560.0, 120.0, 60.0),
                b(620.0, 500.0, 40.0, 60.0),
                b(180.0, 520.0, 40.0, 160.0), // temple pillars
                b(1060.0, 520.0, 40.0, 160.0),
                b(140.0, 500.0, 120.0, 20.0),
                b(1020.0, 500.0, 120.0, 20.0),
                b(330.0, 420.0, 200.0, 20.0),
                b(750.0, 420.0, 200.0, 20.0),
                b(540.0, 330.0, 200.0, 20.0),
            ],
            hazard: None,
            zones: vec![z(540.0, 250.0, 200.0, 80.0), z(330.0, 340.0, 200.0, 80.0), z(750.0, 340.0, 200.0, 80.0)],
            spawns: [90.0, W as f32 - 90.0 - SIZE],
        },
        Theme::Neon => Map {
            solids: vec![
                floor,
                b(600.0, 600.0, 80.0, 80.0), // center cabinet
                b(60.0, 540.0, 200.0, 20.0),
                b(1020.0, 540.0, 200.0, 20.0),
                b(300.0, 450.0, 150.0, 20.0),
                b(830.0, 450.0, 150.0, 20.0),
                b(565.0, 360.0, 150.0, 20.0),
                b(60.0, 300.0, 160.0, 20.0),
                b(1060.0, 300.0, 160.0, 20.0),
            ],
            hazard: None,
            zones: vec![z(565.0, 280.0, 150.0, 80.0), z(300.0, 370.0, 150.0, 80.0), z(830.0, 370.0, 150.0, 80.0)],
            spawns: [110.0, W as f32 - 110.0 - SIZE],
        },
    }
}

// =====================================================================
// per-round state
// =====================================================================

struct Round {
    time: f32,
    stocks: [i32; 2],
    hill: [f32; 2],
    zone_idx: usize,
    zone_t: f32,
    pickup_t: f32,
    meteor_t: f32,
    wind: f32,
    wind_warn: bool,
    zerog: bool,
    zerog_warn: bool,
    combo: [u32; 2],
    combo_t: [f32; 2],
    combo_show: [f32; 2],
}

impl Round {
    fn new() -> Self {
        Round {
            time: 0.0,
            stocks: [STOCKS, STOCKS],
            hill: [0.0, 0.0],
            zone_idx: 0,
            zone_t: HILL_MOVE,
            pickup_t: 6.0,
            meteor_t: 4.0,
            wind: 0.0,
            wind_warn: false,
            zerog: false,
            zerog_warn: false,
            combo: [0, 0],
            combo_t: [0.0, 0.0],
            combo_show: [0.0, 0.0],
        }
    }
}

// =====================================================================
// physics
// =====================================================================

// last line of defense: nobody ever leaves the arena
fn clamp_to_arena(p: &mut Player) {
    let max_x = W as f32 - SIZE;
    if p.x < 0.0 {
        p.x = 0.0;
        p.vx = p.vx.max(0.0);
    } else if p.x > max_x {
        p.x = max_x;
        p.vx = p.vx.min(0.0);
    }
    if p.y < 0.0 {
        p.y = 0.0;
        p.vy = p.vy.max(0.0);
    }
    let max_y = H as f32 - FLOOR_H - SIZE;
    if p.y > max_y {
        p.y = max_y;
        p.vy = 0.0;
        p.on_ground = true;
    }
}

// keep inside the screen walls and out of blocks (sideways)
fn push_out_x(p: &mut Player, solids: &[Solid]) {
    let max_x = W as f32 - SIZE;
    p.x = p.x.clamp(0.0, max_x);
    for s in solids {
        if !overlaps(&p.rect(), &s.r) {
            continue;
        }
        let left_x = s.r.x - SIZE; // stand to the left of the block
        let right_x = s.r.x + s.r.width; // stand to the right of it
        let from_left = if left_x < 0.0 {
            false // no room on the left side of the screen
        } else if right_x > max_x {
            true
        } else if p.vx > 0.0 {
            true
        } else if p.vx < 0.0 {
            false
        } else {
            p.x + SIZE / 2.0 < s.r.x + s.r.width / 2.0
        };
        p.x = if from_left { left_x } else { right_x };
        p.vx = 0.0;
    }
    p.x = p.x.clamp(0.0, max_x);
}

// squares are solid to each other: stand on heads, shove sideways
fn collide_players(pl: &mut [Player; 2], solids: &[Solid]) {
    let (a, b) = pl.split_at_mut(1);
    let (a, b) = (&mut a[0], &mut b[0]);
    if a.dead_t > 0.0 || b.dead_t > 0.0 || !overlaps(&a.rect(), &b.rect()) {
        return;
    }
    let ox = (a.x + SIZE).min(b.x + SIZE) - a.x.max(b.x);
    let oy = (a.y + SIZE).min(b.y + SIZE) - a.y.max(b.y);
    if oy < ox {
        // the top square rides on the bottom one; the bottom one is never
        // stopped, so jumps, hops and leaps still work with someone on your head
        let (up, low) = if a.y < b.y { (a, b) } else { (b, a) };
        up.y = low.y - SIZE;
        up.vy = low.vy;
        up.on_ground = true;
    } else {
        let push = ox / 2.0 + 0.5;
        if a.x + SIZE / 2.0 < b.x + SIZE / 2.0 {
            a.x -= push;
            b.x += push;
        } else {
            a.x += push;
            b.x -= push;
        }
        let avg = (a.vx + b.vx) / 2.0;
        a.vx = avg - (a.vx - avg) * 0.2;
        b.vx = avg - (b.vx - avg) * 0.2;
        push_out_x(a, solids);
        push_out_x(b, solids);
    }
}

fn step_once(p: &mut Player, solids: &[Solid], dt: f32, gs: f32) {
    p.x += p.vx * dt;
    push_out_x(p, solids);

    if p.dash_t <= 0.0 {
        p.vy += GRAVITY * p.kind.grav() * gs * dt;
    }
    let prev_bottom = p.y + SIZE;
    p.y += p.vy * dt;
    p.on_ground = false;
    for s in solids {
        if !overlaps(&p.rect(), &s.r) {
            continue;
        }
        if p.vy >= 0.0 && prev_bottom <= s.r.y + 1.0 {
            p.y = s.r.y - SIZE;
            p.vy = 0.0;
            p.on_ground = true;
        } else if p.vy < 0.0 {
            p.y = s.r.y + s.r.height; // bonk head
            p.vy = 0.0;
        }
    }
}

// which side (if any) is a wall within reach: -1 left, 1 right, 0 none
fn wall_side(p: &Player, solids: &[Solid]) -> f32 {
    if p.on_ground {
        return 0.0;
    }
    if p.x <= 1.0 {
        return -1.0;
    }
    if p.x >= W as f32 - SIZE - 1.0 {
        return 1.0;
    }
    for dir in [-1.0_f32, 1.0] {
        let probe = Rectangle::new(p.x + dir * 3.0, p.y + 6.0, SIZE, SIZE - 12.0);
        if solids.iter().any(|s| overlaps(&probe, &s.r)) {
            return dir;
        }
    }
    0.0
}

// move + collide; fast movers are split into small steps so nobody tunnels
fn step(p: &mut Player, solids: &[Solid], dt: f32, gs: f32) {
    let moves = (p.vx.abs().max(p.vy.abs()) * dt / 20.0).ceil().clamp(1.0, 8.0) as i32;
    let sub = dt / moves as f32;
    for _ in 0..moves {
        step_once(p, solids, sub, gs);
    }
    clamp_to_arena(p);
    p.wall_dir = wall_side(p, solids);
}

fn launch(p: &mut Player, power: f32) {
    p.vy = -(2.0 * GRAVITY * JUMP * power).sqrt();
    p.on_ground = false;
    p.charge = 0.0;
    p.crouch = 0.0;
}

// =====================================================================
// abilities
// =====================================================================

fn use_ability(ab: Ability, me: &mut Player, foe: &mut Player, my_idx: usize, solids: &[Solid], w: &mut World) {
    let (cx, cy) = me.center();
    match ab {
        Ability::Dash => {
            me.dash_t = DASH_TIME;
            me.dash_hit = false;
            w.burst(cx, cy, 10, Color::new(255, 200, 100, 255), 300.0, 0.3, 3.0, Shape::Streak, 0.0);
            w.shake(4.0);
        }
        Ability::Slam => {
            if me.on_ground {
                // leap up first, then crash down at the top of the leap
                me.vy = -(2.0 * GRAVITY * JUMP * 0.8).sqrt();
                me.on_ground = false;
                me.slam_arm = true;
                w.ring(cx, me.y + SIZE, 60.0, Color::new(255, 220, 120, 255));
                w.burst(cx, me.y + SIZE, 10, Color::new(200, 190, 170, 200), 160.0, 0.5, 8.0, Shape::Circle, -40.0);
            } else {
                me.slam = true;
            }
        }
        Ability::Bomb => {
            w.projs.push(Proj {
                x: cx,
                y: cy,
                vx: me.facing * 550.0 + me.vx * 0.5,
                vy: -300.0,
                owner: my_idx,
                life: 3.0,
                kind: ProjKind::Bomb,
            });
            w.burst(cx + me.facing * 30.0, cy, 6, Color::new(255, 190, 60, 255), 180.0, 0.25, 4.0, Shape::Circle, 0.0);
        }
        Ability::Hop => {
            me.vy = -(2.0 * GRAVITY * JUMP * 1.2).sqrt();
            me.on_ground = false;
            w.ring(cx, me.y + SIZE, 70.0, Color::WHITE);
            w.burst(cx, me.y + SIZE, 14, Color::new(230, 230, 230, 200), 220.0, 0.5, 7.0, Shape::Circle, 60.0);
        }
        Ability::Shield => {
            me.shield_t = SHIELD_TIME;
            w.ring(cx, cy, 90.0, Color::new(120, 220, 255, 255));
            w.burst(cx, cy, 16, Color::new(150, 230, 255, 255), 260.0, 0.4, 6.0, Shape::Star, 0.0);
        }
        Ability::Pulse => {
            let reach = 170.0;
            let rx = if me.facing > 0.0 { me.x + SIZE } else { me.x - reach };
            let zone = Rectangle::new(rx, me.y, reach, SIZE);
            let fx = if me.facing > 0.0 { me.x + SIZE + 40.0 } else { me.x - 40.0 };
            if overlaps(&zone, &foe.rect()) {
                foe.hurt(12.0, me.facing * 700.0, -250.0);
            }
            w.ring(fx, cy, 85.0, Color::new(255, 255, 200, 255));
            for k in 0..8 {
                let sp = 250.0 + k as f32 * 40.0;
                let q = Particle::new(cx + me.facing * 35.0, cy + (k as f32 - 3.5) * 8.0, me.facing * sp, 0.0, 0.3, 3.0, Color::WHITE, Shape::Streak);
                w.parts.push(q);
            }
            w.shake(6.0);
        }
        Ability::Blink => {
            let mut nx = me.x;
            for _ in 0..(BLINK_DIST / 10.0) as i32 {
                let tx = nx + me.facing * 10.0;
                let r = Rectangle::new(tx, me.y, SIZE, SIZE);
                if tx < 0.0 || tx > W as f32 - SIZE || solids.iter().any(|s| overlaps(&r, &s.r)) || overlaps(&r, &foe.rect()) {
                    break;
                }
                nx = tx;
            }
            let violet = Color::new(190, 120, 255, 255);
            // streak between start and end, flashes at both ends
            for k in 0..14 {
                let f = k as f32 / 13.0;
                let px = cx + (nx - me.x) * f;
                let q = Particle::new(px, cy + w.rng.range(-20.0, 20.0), 0.0, w.rng.range(-20.0, 20.0), 0.4, 5.0, violet, Shape::Star);
                w.parts.push(q);
            }
            w.ring(cx, cy, 70.0, violet);
            w.burst(cx, cy, 14, violet, 260.0, 0.4, 5.0, Shape::Square, 0.0);
            me.x = nx;
            w.ring(nx + SIZE / 2.0, cy, 70.0, Color::WHITE);
            w.burst(nx + SIZE / 2.0, cy, 14, Color::WHITE, 260.0, 0.4, 5.0, Shape::Star, 0.0);
            me.vy = me.vy.min(0.0);
        }
        Ability::Frost => {
            w.projs.push(Proj { x: cx, y: cy, vx: me.facing * 700.0, vy: 0.0, owner: my_idx, life: 1.2, kind: ProjKind::Frost });
            w.ring(cx + me.facing * 30.0, cy, 50.0, Color::new(150, 230, 255, 255));
        }
        Ability::Thunder => {
            let (fx, _) = foe.center();
            w.strikes.push(Strike { x: fx, t: STRIKE_DELAY, owner: my_idx, struck: false, vis: 0.0, pts: Vec::new() });
            w.burst(cx, cy, 8, Color::new(255, 240, 120, 255), 200.0, 0.4, 5.0, Shape::Star, 0.0);
        }
        Ability::Magnet => {
            let (fx, fy) = foe.center();
            let dist = ((fx - cx).powi(2) + (fy - cy).powi(2)).sqrt();
            if dist < 650.0 {
                let dir: f32 = if fx > cx { -1.0 } else { 1.0 };
                foe.vx = (dir * 1100.0).clamp(-MAX_SPEED, MAX_SPEED);
                foe.vy = -140.0;
                foe.on_ground = false;
                for k in 0..10 {
                    let f = k as f32 / 10.0;
                    let q = Particle::new(fx + (cx - fx) * f, fy + (cy - fy) * f, (cx - fx) * 1.5, (cy - fy) * 1.5, 0.3, 3.0, Color::new(255, 120, 200, 255), Shape::Streak);
                    w.parts.push(q);
                }
            }
            w.ring(cx, cy, 130.0, Color::new(255, 120, 200, 255));
            w.shake(5.0);
        }
        Ability::Mend => {
            me.regen_t = 2.0;
            w.ring(cx, cy, 80.0, Color::new(90, 255, 120, 255));
            w.burst(cx, cy, 14, Color::new(120, 255, 140, 255), 140.0, 0.8, 7.0, Shape::Star, -80.0);
        }
        Ability::Fireball => {
            w.projs.push(Proj { x: cx, y: cy, vx: me.facing * 650.0, vy: 0.0, owner: my_idx, life: 1.5, kind: ProjKind::Fire });
            w.burst(cx + me.facing * 30.0, cy, 8, Color::new(255, 150, 40, 255), 220.0, 0.3, 6.0, Shape::Circle, 0.0);
            w.shake(3.0);
        }
    }
}

// each character's ultimate: a big cinematic super move
fn use_ult(me: &mut Player, foe: &mut Player, my_idx: usize, w: &mut World) {
    let (cx, cy) = me.center();
    let col = me.kind.color();
    w.fx.hitstop = w.fx.hitstop.max(0.25);
    w.fx.flash = w.fx.flash.max(0.6);
    w.fx.punch = w.fx.punch.max(0.1);
    w.shake(16.0);
    w.ring(cx, cy, 300.0, Color::WHITE);
    w.ring(cx, cy, 200.0, col);
    w.burst(cx, cy, 30, col, 600.0, 0.7, 8.0, Shape::Star, 0.0);
    w.banner = Some((me.kind.ult_name().to_string(), col, 1.5));
    match me.kind {
        Kind::Dasher => {
            me.over_t = 6.0;
            me.cd1 = 0.0;
            me.cd2 = 0.0;
        }
        Kind::Bomber => {
            for k in 0..8 {
                w.projs.push(Proj {
                    x: 90.0 + k as f32 * 150.0,
                    y: -40.0 - k as f32 * 70.0,
                    vx: 0.0,
                    vy: 260.0,
                    owner: my_idx,
                    life: 5.0,
                    kind: ProjKind::Bomb,
                });
            }
        }
        Kind::Shielder => {
            me.shield_t = 3.0;
            let (fx, fy) = foe.center();
            let dist = ((fx - cx).powi(2) + (fy - cy).powi(2)).sqrt();
            if dist < 450.0 {
                let dir = if fx >= cx { 1.0 } else { -1.0 };
                foe.hurt(18.0, dir * 900.0, -300.0);
            }
            w.ring(cx, cy, 450.0, Color::new(150, 230, 255, 255));
        }
        Kind::Titan => {
            w.shake(30.0);
            for k in 0..14 {
                let x = 40.0 + k as f32 * 95.0;
                w.burst(x, H as f32 - FLOOR_H, 4, Color::new(120, 100, 80, 255), 380.0, 0.9, 12.0, Shape::Square, 900.0);
            }
            w.ring(cx, me.y + SIZE, 600.0, Color::new(255, 190, 80, 255));
            if foe.on_ground && foe.dead_t <= 0.0 {
                foe.hurt(25.0, 0.0, -650.0);
                foe.stun_t = 0.6;
            }
        }
        Kind::Ghost => {
            let (fx, fy) = foe.center();
            if foe.shield_t <= 0.0 && foe.dead_t <= 0.0 {
                foe.frozen_t = 2.0;
                foe.vx = 0.0;
                foe.hp = (foe.hp - 10.0).max(0.0);
                foe.hurt_t = 0.15;
            }
            w.ring(fx, fy, 260.0, Color::new(170, 235, 255, 255));
            w.burst(fx, fy, 30, Color::new(190, 240, 255, 255), 500.0, 0.8, 8.0, Shape::Star, 200.0);
        }
        Kind::Spark => {
            let (fx, _) = foe.center();
            for (k, off) in [-240.0_f32, -120.0, 0.0, 120.0, 240.0].iter().enumerate() {
                w.strikes.push(Strike {
                    x: (fx + off).clamp(40.0, W as f32 - 40.0),
                    t: 0.8 + k as f32 * 0.22,
                    owner: my_idx,
                    struck: false,
                    vis: 0.0,
                    pts: Vec::new(),
                });
            }
        }
    }
}

// runs AFTER collisions are resolved, so landing on the other square counts too
fn slam_land(me: &mut Player, foe: &mut Player, w: &mut World) {
    if !(me.slam && me.on_ground) {
        return;
    }
    me.slam = false;
    let (mx, _) = me.center();
    let (fx, _) = foe.center();
    let base = me.y + SIZE;
    w.ring(mx, base, 250.0, Color::new(255, 190, 80, 255));
    w.ring(mx, base, 150.0, Color::WHITE);
    w.burst(mx, base, 26, Color::new(150, 120, 90, 255), 520.0, 0.8, 12.0, Shape::Square, 900.0);
    w.burst(mx, base, 14, Color::new(210, 200, 185, 180), 300.0, 0.9, 10.0, Shape::Circle, -30.0);
    for k in 0..16 {
        let dir = if k % 2 == 0 { 1.0 } else { -1.0 };
        let q = Particle::new(mx, base - 4.0, dir * (200.0 + k as f32 * 35.0), 0.0, 0.35, 3.0, Color::WHITE, Shape::Streak);
        w.parts.push(q);
    }
    w.shake(18.0);
    w.fx.punch = w.fx.punch.max(0.07);
    w.fx.flash = w.fx.flash.max(0.2);
    // hits if they're near and on the ground (or you landed right on them)
    let landed_on_foe = (foe.y - base).abs() < 2.0 && (fx - mx).abs() < SIZE;
    if (fx - mx).abs() < 250.0 && (landed_on_foe || foe.y + SIZE >= base - 40.0) {
        let dir = if (fx - mx).abs() < 1.0 { me.facing } else { (fx - mx).signum() };
        foe.hurt(20.0, dir * 500.0, -350.0);
    }
}

fn explode(players: &mut [Player; 2], owner: usize, cx: f32, cy: f32, w: &mut World) {
    w.ring(cx, cy, BLAST_R, Color::new(255, 150, 40, 255));
    w.ring(cx, cy, BLAST_R * 0.6, Color::new(255, 255, 220, 255));
    w.burst(cx, cy, 30, Color::new(255, 170, 40, 255), 520.0, 0.6, 9.0, Shape::Circle, 0.0);
    w.burst(cx, cy, 16, Color::new(255, 240, 160, 255), 420.0, 0.4, 6.0, Shape::Star, 0.0);
    w.burst(cx, cy, 12, Color::new(70, 60, 60, 200), 160.0, 1.0, 14.0, Shape::Circle, -60.0);
    w.burst(cx, cy, 14, Color::new(90, 70, 60, 255), 450.0, 0.8, 9.0, Shape::Square, 700.0);
    w.shake(12.0);
    w.fx.punch = w.fx.punch.max(0.04);
    w.fx.flash = w.fx.flash.max(0.15);
    let mult = if players[owner].kind == Kind::Bomber { 1.2 } else { 1.0 };
    let reach = BLAST_R + SIZE / 2.0;
    for j in 0..2 {
        let p = &mut players[j];
        let (px, py) = p.center();
        let (dx, dy) = (px - cx, py - cy);
        let d = (dx * dx + dy * dy).sqrt();
        if d > reach {
            continue;
        }
        let (nx, ny) = if d < 1.0 { (0.0, -1.0) } else { (dx / d, dy / d) };
        let f = 1.0 - (d / reach) * 0.5;
        if j == owner {
            // blast jump: knockback only, no self damage
            p.vx += nx * 500.0 * f;
            p.vy = ny * 600.0 * f - 100.0;
            p.on_ground = false;
        } else {
            p.hurt(25.0 * f * mult, nx * 600.0 * f, ny * 600.0 * f - 200.0);
        }
    }
}

// what happens when a projectile ends (hit something or ran out)
fn proj_impact(b: &Proj, hit_foe: bool, players: &mut [Player; 2], w: &mut World) {
    let foe = 1 - b.owner;
    match b.kind {
        ProjKind::Bomb => explode(players, b.owner, b.x, b.y, w),
        ProjKind::Frost => {
            let ice = Color::new(170, 235, 255, 255);
            if hit_foe {
                let p = &mut players[foe];
                p.hp = (p.hp - 5.0).max(0.0);
                p.frozen_t = FROZEN_TIME;
                p.vx = 0.0;
                p.hurt_t = 0.15;
            }
            w.ring(b.x, b.y, 80.0, ice);
            w.burst(b.x, b.y, 22, ice, 360.0, 0.6, 8.0, Shape::Star, 200.0);
            w.burst(b.x, b.y, 10, Color::WHITE, 260.0, 0.5, 8.0, Shape::Square, 400.0);
            w.shake(6.0);
        }
        ProjKind::Fire => {
            if hit_foe {
                let dir = if b.vx >= 0.0 { 1.0 } else { -1.0 };
                let p = &mut players[foe];
                if p.shield_t <= 0.0 {
                    p.hurt(14.0, dir * 300.0, -150.0);
                    p.burn_t = BURN_TIME;
                }
            }
            w.ring(b.x, b.y, 70.0, Color::new(255, 140, 40, 255));
            w.burst(b.x, b.y, 22, Color::new(255, 150, 40, 255), 340.0, 0.5, 9.0, Shape::Circle, -80.0);
            w.burst(b.x, b.y, 10, Color::new(255, 230, 120, 255), 300.0, 0.4, 5.0, Shape::Star, 0.0);
            w.shake(7.0);
        }
    }
}

fn update_projs(players: &mut [Player; 2], solids: &[Solid], w: &mut World, dt: f32) {
    let projs = std::mem::take(&mut w.projs);
    let mut keep = Vec::new();
    for mut b in projs {
        if let ProjKind::Bomb = b.kind {
            b.vy += BOMB_GRAVITY * dt;
        }
        b.x += b.vx * dt;
        b.y += b.vy * dt;
        b.life -= dt;

        // projectile trails
        match b.kind {
            ProjKind::Bomb => {
                if w.rng.next() < 0.5 {
                    let q = Particle::new(b.x, b.y, w.rng.range(-30.0, 30.0), w.rng.range(-40.0, -10.0), 0.4, 4.0, Color::new(255, 170, 60, 255), Shape::Circle);
                    w.parts.push(q);
                }
            }
            ProjKind::Fire => {
                let mut q = Particle::new(b.x, b.y + w.rng.range(-6.0, 6.0), -b.vx * 0.1, w.rng.range(-30.0, 10.0), 0.4, 9.0, Color::new(255, 130 + (w.rng.next() * 100.0) as u8, 30, 230), Shape::Circle);
                q.grav = -60.0;
                w.parts.push(q);
            }
            ProjKind::Frost => {
                if w.rng.next() < 0.7 {
                    let q = Particle::new(b.x, b.y + w.rng.range(-8.0, 8.0), -b.vx * 0.05, w.rng.range(-20.0, 20.0), 0.35, 5.0, Color::new(200, 245, 255, 255), Shape::Star);
                    w.parts.push(q);
                }
            }
        }

        let mut end = b.life <= 0.0 || b.x < 0.0 || b.x > W as f32 || b.y > H as f32 || b.y < -400.0;
        let br = Rectangle::new(b.x - 8.0, b.y - 8.0, 16.0, 16.0);
        if b.y > 0.0 && solids.iter().any(|s| overlaps(&br, &s.r)) {
            end = true;
        }
        let foe = 1 - b.owner;
        let mut hit = false;
        if players[foe].dead_t <= 0.0 && overlaps(&br, &players[foe].rect()) {
            if players[foe].shield_t > 0.0 {
                // reflected back at the thrower
                b.vx = -b.vx;
                b.vy = if let ProjKind::Bomb = b.kind { -b.vy.abs() } else { b.vy };
                b.owner = foe;
                b.life = 3.0;
                end = false;
                w.burst(b.x, b.y, 10, Color::new(150, 230, 255, 255), 300.0, 0.3, 5.0, Shape::Star, 0.0);
                w.ring(b.x, b.y, 40.0, Color::new(150, 230, 255, 255));
                w.shake(5.0);
            } else {
                hit = true;
                end = true;
            }
        }
        if end {
            proj_impact(&b, hit, players, w);
        } else {
            keep.push(b);
        }
    }
    w.projs = keep;
}

fn update_strikes(players: &mut [Player; 2], w: &mut World, dt: f32) {
    let ground = H as f32 - FLOOR_H;
    let mut strikes = std::mem::take(&mut w.strikes);
    for s in strikes.iter_mut() {
        if !s.struck {
            s.t -= dt;
            if s.t <= 0.0 {
                s.struck = true;
                s.vis = 0.35;
                let mut y = 0.0;
                let mut x = s.x;
                while y < ground {
                    s.pts.push(Vector2::new(x, y));
                    y += w.rng.range(30.0, 60.0);
                    x = s.x + w.rng.range(-28.0, 28.0);
                }
                s.pts.push(Vector2::new(s.x, ground));
                let column = Rectangle::new(s.x - 35.0, 0.0, 70.0, ground);
                for j in 0..2 {
                    if j != s.owner && overlaps(&column, &players[j].rect()) {
                        players[j].hurt(22.0, 0.0, -320.0);
                    }
                }
                w.fx.flash = w.fx.flash.max(0.55);
                w.shake(20.0);
                w.fx.punch = w.fx.punch.max(0.06);
                w.ring(s.x, ground, 140.0, Color::new(255, 245, 150, 255));
                w.burst(s.x, ground, 24, Color::new(255, 245, 150, 255), 460.0, 0.6, 8.0, Shape::Star, 400.0);
                w.burst(s.x, ground, 12, Color::new(120, 110, 100, 255), 300.0, 0.7, 8.0, Shape::Square, 800.0);
            }
        } else {
            s.vis -= dt;
        }
    }
    strikes.retain(|s| !(s.struck && s.vis <= 0.0));
    w.strikes = strikes;
}

// =====================================================================
// map events: wind, meteors, pickups
// =====================================================================

fn update_meteors(players: &mut [Player; 2], solids: &[Solid], w: &mut World, dt: f32) {
    let ground = H as f32 - FLOOR_H;
    let meteors = std::mem::take(&mut w.meteors);
    let mut keep = Vec::new();
    for mut m in meteors {
        if !m.falling {
            m.warn -= dt;
            if m.warn <= 0.0 {
                m.falling = true;
                m.x = m.tx - 300.0;
                m.y = -60.0;
                m.vx = 272.0;
                m.vy = 672.0;
            }
            keep.push(m);
            continue;
        }
        m.x += m.vx * dt;
        m.y += m.vy * dt;
        let mut q = Particle::new(m.x, m.y, w.rng.range(-40.0, 40.0), w.rng.range(-60.0, 0.0), 0.5, 10.0, Color::new(255, 140 + (w.rng.next() * 90.0) as u8, 30, 230), Shape::Circle);
        q.grav = -50.0;
        w.parts.push(q);
        let r = Rectangle::new(m.x - 14.0, m.y - 14.0, 28.0, 28.0);
        let hit = m.y >= ground || (m.y > 0.0 && solids.iter().any(|s| overlaps(&r, &s.r)));
        if hit {
            let (ix, iy) = (m.x, m.y.min(ground));
            w.ring(ix, iy, 120.0, Color::new(255, 150, 50, 255));
            w.ring(ix, iy, 70.0, Color::WHITE);
            w.burst(ix, iy, 26, Color::new(255, 160, 40, 255), 480.0, 0.6, 9.0, Shape::Circle, 100.0);
            w.burst(ix, iy, 12, Color::new(90, 70, 60, 255), 420.0, 0.8, 9.0, Shape::Square, 800.0);
            w.shake(14.0);
            w.fx.flash = w.fx.flash.max(0.15);
            for p in players.iter_mut() {
                let (px, py) = p.center();
                let d = ((px - ix).powi(2) + (py - iy).powi(2)).sqrt();
                if d < 130.0 {
                    let dir = if px >= ix { 1.0 } else { -1.0 };
                    p.hurt(15.0, dir * 450.0, -300.0);
                }
            }
        } else {
            keep.push(m);
        }
    }
    w.meteors = keep;
}

fn update_lasers(players: &mut [Player; 2], w: &mut World, dt: f32) {
    let lasers = std::mem::take(&mut w.lasers);
    let mut keep = Vec::new();
    for mut l in lasers {
        if !l.fired {
            l.t -= dt;
            if l.t <= 0.0 {
                l.fired = true;
                l.vis = 0.4;
                let beam = Rectangle::new(0.0, l.y - 22.0, W as f32, 44.0);
                for p in players.iter_mut() {
                    if overlaps(&beam, &p.rect()) {
                        p.hurt(18.0, 0.0, -300.0);
                    }
                }
                w.fx.flash = w.fx.flash.max(0.3);
                w.shake(14.0);
                for k in 0..22 {
                    let x = k as f32 * 60.0;
                    let q = Particle::new(x, l.y, w.rng.range(-80.0, 80.0), w.rng.range(-200.0, 200.0), 0.5, 4.0, Color::new(255, 80, 220, 255), Shape::Star);
                    w.parts.push(q);
                }
            }
        } else {
            l.vis -= dt;
        }
        if !(l.fired && l.vis <= 0.0) {
            keep.push(l);
        }
    }
    w.lasers = keep;
}

// gusting wind / blizzard / currents: (cycle length, warning starts, gust starts, drift speed)
fn wind_params(theme: Theme) -> Option<(f32, f32, f32, f32)> {
    match theme {
        Theme::Meadow => Some((14.0, 8.0, 10.0, 140.0)),
        Theme::Frozen => Some((11.0, 6.0, 8.0, 210.0)),
        Theme::Sea => Some((10.0, 5.0, 7.0, 170.0)),
        _ => None,
    }
}

fn update_events(theme: Theme, rd: &mut Round, players: &mut [Player; 2], solids: &[Solid], w: &mut World, dt: f32) {
    // default: normal gravity; events below may change it
    w.grav_scale = if theme == Theme::Sea { 0.65 } else { 1.0 };

    if let Some((cycle_len, warn_at, gust_at, speed)) = wind_params(theme) {
        let cyc = rd.time % cycle_len;
        let n = (rd.time / cycle_len) as i32;
        let dir: f32 = if n % 2 == 0 { 1.0 } else { -1.0 };
        rd.wind = if cyc >= gust_at { dir } else { 0.0 };
        rd.wind_warn = cyc >= warn_at && cyc < gust_at;
        if rd.wind != 0.0 {
            for p in players.iter_mut() {
                if p.dead_t <= 0.0 {
                    p.x += rd.wind * speed * dt;
                    push_out_x(p, solids);
                }
            }
            let streak = match theme {
                Theme::Sea => Color::new(200, 240, 255, 130),
                Theme::Frozen => Color::new(255, 255, 255, 190),
                _ => Color::new(255, 255, 255, 150),
            };
            for _ in 0..2 {
                let sx = if rd.wind > 0.0 { -10.0 } else { W as f32 + 10.0 };
                let y = w.rng.range(0.0, H as f32 - FLOOR_H);
                let vx = rd.wind * w.rng.range(700.0, 1000.0);
                let q = Particle::new(sx, y, vx, w.rng.range(-30.0, 30.0), 2.0, 3.0, streak, Shape::Streak);
                w.parts.push(q);
            }
        }
    }

    match theme {
        Theme::Skyline | Theme::Volcano | Theme::Sand | Theme::Space => {
            rd.meteor_t -= dt;
            if rd.meteor_t <= 0.0 {
                rd.meteor_t = match theme {
                    Theme::Skyline => w.rng.range(4.0, 6.5),
                    Theme::Volcano => w.rng.range(3.5, 5.5),
                    Theme::Sand => w.rng.range(3.0, 4.5),
                    _ => w.rng.range(5.5, 8.0),
                };
                let tx = w.rng.range(100.0, W as f32 - 100.0);
                w.meteors.push(Meteor { tx, warn: 1.0, x: 0.0, y: 0.0, vx: 0.0, vy: 0.0, falling: false });
            }
        }
        Theme::Neon => {
            rd.meteor_t -= dt;
            if rd.meteor_t <= 0.0 {
                rd.meteor_t = w.rng.range(3.5, 5.5);
                let y = w.rng.range(140.0, H as f32 - FLOOR_H - 50.0);
                w.lasers.push(Laser { y, t: 1.1, fired: false, vis: 0.0 });
            }
        }
        _ => {}
    }

    if theme == Theme::Space {
        // periodic zero-g: everything floats
        let cyc = rd.time % 16.0;
        rd.zerog = cyc >= 10.0;
        rd.zerog_warn = (8.0..10.0).contains(&cyc);
        if rd.zerog {
            w.grav_scale = 0.3;
        }
    }

    update_meteors(players, solids, w, dt);
    update_lasers(players, w, dt);
}

fn update_pickups(players: &mut [Player; 2], solids: &[Solid], rd: &mut Round, w: &mut World, dt: f32) {
    rd.pickup_t -= dt;
    if rd.pickup_t <= 0.0 && w.pickups.len() < 2 {
        // try a few random spots on top of blocks; only accept one that is
        // completely out in the open (not inside or touching any wall/block)
        for _ in 0..30 {
            let si = (w.rng.next() * solids.len() as f32) as usize % solids.len();
            let r = solids[si].r;
            if r.width < 70.0 {
                continue;
            }
            let x = r.x + w.rng.range(30.0, r.width - 30.0);
            let y = r.y - 32.0;
            let area = Rectangle::new(x - 28.0, y - 28.0, 56.0, 56.0 - 2.0); // sits just above the surface
            let blocked = solids.iter().any(|s| overlaps(&area, &s.r));
            let too_close = w.pickups.iter().any(|p| (p.x - x).abs() < 160.0);
            if blocked || too_close || x < 40.0 || x > W as f32 - 40.0 {
                continue;
            }
            rd.pickup_t = w.rng.range(8.0, 11.0);
            let kind = match (w.rng.next() * 4.0) as i32 {
                0 => PickKind::Health,
                1 => PickKind::Power,
                2 => PickKind::Haste,
                _ => PickKind::Reset,
            };
            w.ring(x, y, 40.0, kind.color());
            w.pickups.push(Pickup { x, y, kind, age: 0.0 });
            break;
        }
    }
    let pickups = std::mem::take(&mut w.pickups);
    let mut keep = Vec::new();
    for mut pk in pickups {
        pk.age += dt;
        let area = Rectangle::new(pk.x - 24.0, pk.y - 24.0, 48.0, 48.0);
        let mut taken = false;
        for p in players.iter_mut() {
            if p.dead_t <= 0.0 && !taken && overlaps(&area, &p.rect()) {
                taken = true;
                match pk.kind {
                    PickKind::Health => p.hp = (p.hp + 25.0).min(p.max_hp),
                    PickKind::Power => p.power_t = 8.0,
                    PickKind::Haste => p.haste_t = 8.0,
                    PickKind::Reset => {
                        p.cd1 = 0.0;
                        p.cd2 = 0.0;
                        p.ult = (p.ult + 25.0).min(ULT_MAX);
                    }
                }
            }
        }
        if taken {
            w.ring(pk.x, pk.y, 90.0, pk.kind.color());
            w.burst(pk.x, pk.y, 18, pk.kind.color(), 320.0, 0.6, 6.0, Shape::Star, 0.0);
        } else {
            keep.push(pk);
        }
    }
    w.pickups = keep;
}

// =====================================================================
// player update
// =====================================================================

fn update_player(inp: &Input, me: &mut Player, foe: &mut Player, my_idx: usize, solids: &[Solid], w: &mut World, dt: f32) {
    let cd_tick = if me.over_t > 0.0 { dt * 3.0 } else { dt };
    me.cd1 = (me.cd1 - cd_tick).max(0.0);
    me.cd2 = (me.cd2 - cd_tick).max(0.0);
    me.shield_t = (me.shield_t - dt).max(0.0);
    me.hurt_t = (me.hurt_t - dt).max(0.0);
    me.frozen_t = (me.frozen_t - dt).max(0.0);
    me.stun_t = (me.stun_t - dt).max(0.0);
    me.power_t = (me.power_t - dt).max(0.0);
    me.haste_t = (me.haste_t - dt).max(0.0);
    me.over_t = (me.over_t - dt).max(0.0);
    me.parry_t = (me.parry_t - dt).max(0.0);
    me.block_cd = (me.block_cd - dt).max(0.0);
    me.ult = (me.ult + 1.5 * dt).min(ULT_MAX); // slow passive charge
    let can_act = me.frozen_t <= 0.0 && me.stun_t <= 0.0;
    let (cx, cy) = me.center();

    if me.on_ground {
        me.air_jumps = me.kind.air_jumps();
    }

    // ---- guard: hold to block, tap right as a hit lands to parry ----
    let press = inp.block_down && !me.block_prev;
    me.block_prev = inp.block_down;
    if can_act && me.block_cd <= 0.0 && inp.block_down {
        if press {
            me.parry_t = PARRY_WINDOW;
        }
        me.block_held = true;
        me.block_t += dt;
        if me.block_t > BLOCK_MAX {
            me.block_held = false;
            me.block_cd = 1.2; // guard break: can't block for a moment
            me.block_t = 0.0;
        }
    } else {
        me.block_held = false;
        me.block_t = (me.block_t - dt * 2.0).max(0.0);
    }
    let acting = can_act && !me.block_held;

    // ---- burning, healing, buff and status particles ----
    if me.burn_t > 0.0 {
        me.burn_t -= dt;
        me.hp = (me.hp - 4.0 * dt).max(0.0);
        if w.rng.next() < 0.7 {
            let mut q = Particle::new(cx + w.rng.range(-22.0, 22.0), cy + w.rng.range(-10.0, 25.0), w.rng.range(-20.0, 20.0), -50.0, 0.5, 8.0, Color::new(255, 140 + (w.rng.next() * 100.0) as u8, 30, 230), Shape::Circle);
            q.grav = -80.0;
            w.parts.push(q);
        }
    }
    if me.regen_t > 0.0 {
        me.regen_t -= dt;
        me.hp = (me.hp + 9.0 * dt).min(me.max_hp);
        if w.rng.next() < 0.5 {
            let q = Particle::new(cx + w.rng.range(-25.0, 25.0), cy + 10.0, 0.0, -70.0, 0.7, 6.0, Color::new(110, 255, 140, 255), Shape::Star);
            w.parts.push(q);
        }
    }
    if me.frozen_t > 0.0 && w.rng.next() < 0.3 {
        let q = Particle::new(cx + w.rng.range(-25.0, 25.0), cy + w.rng.range(-25.0, 25.0), 0.0, 30.0, 0.5, 4.0, Color::new(200, 240, 255, 255), Shape::Star);
        w.parts.push(q);
    }
    if me.over_t > 0.0 && w.rng.next() < 0.6 {
        let mut q = Particle::new(cx, cy, 0.0, 0.0, 0.25, SIZE, Color::new(255, 150, 40, 90), Shape::Ghost);
        q.rot = me.rot;
        w.parts.push(q);
    }
    if me.power_t > 0.0 && w.rng.next() < 0.3 {
        let q = Particle::new(cx + w.rng.range(-25.0, 25.0), cy + 20.0, 0.0, -60.0, 0.5, 5.0, Color::new(255, 80, 60, 255), Shape::Star);
        w.parts.push(q);
    }

    // ---- wall slide ----
    let mut push = 0.0;
    if can_act {
        if inp.right {
            push += 1.0;
        }
        if inp.left {
            push -= 1.0;
        }
    }
    if push != 0.0 {
        me.facing = push;
    }
    if !me.on_ground && me.wall_dir != 0.0 && push == me.wall_dir && me.vy > 0.0 {
        me.vy = me.vy.min(WALL_SLIDE);
        if w.rng.next() < 0.4 {
            let q = Particle::new(cx + me.wall_dir * 28.0, cy + 20.0, 0.0, -30.0, 0.3, 4.0, Color::new(220, 220, 220, 200), Shape::Circle);
            w.parts.push(q);
        }
    }

    // ---- momentum: roll and slide ----
    let top = TOP_SPEED * me.speed_mult();
    let accel = top / 0.25;
    if me.dash_t > 0.0 {
        me.dash_t -= dt;
        me.vx = me.facing * DASH_SPEED;
        me.vy = 0.0;
    } else if push != 0.0 && me.vx * push < top {
        me.vx += push * accel * dt;
        if me.vx * push > top {
            me.vx = push * top;
        }
    } else {
        // no input, or knocked faster than top speed: slide down
        let slow = (ICE * dt).min(me.vx.abs());
        me.vx -= me.vx.signum() * slow;
    }
    me.rot += me.vx * dt / (SIZE / 2.0) * 57.3;
    if me.vx == 0.0 {
        let flat = (me.rot / 90.0).round() * 90.0;
        me.rot += (flat - me.rot) * (10.0 * dt).min(1.0);
    }

    // ---- crouch, charge, jump ----
    if acting && inp.down && me.on_ground {
        me.charge = (me.charge + dt).min(1.0);
        me.crouch = CROUCH;
    } else if me.tap_timer <= 0.0 {
        me.charge = 0.0;
        me.crouch = 0.0;
    }
    if acting && inp.jump {
        if me.on_ground && me.tap_timer <= 0.0 {
            if me.charge > 0.0 {
                let power = 1.0 + me.charge * (MAX_CHARGE - 1.0);
                launch(me, power);
            } else {
                me.tap_timer = TAP_CROUCH;
                me.crouch = CROUCH;
            }
        } else if !me.on_ground && me.wall_dir != 0.0 {
            // wall jump: kick off the wall, refunds your air jumps
            let kick = -me.wall_dir;
            me.vy = -(2.0 * GRAVITY * JUMP * 0.95).sqrt();
            me.vx = kick * WALL_KICK;
            me.facing = kick;
            me.air_jumps = me.kind.air_jumps();
            w.burst(cx - kick * 30.0, cy, 8, Color::new(230, 230, 230, 220), 200.0, 0.35, 5.0, Shape::Circle, 100.0);
        } else if !me.on_ground && me.air_jumps > 0 && me.tap_timer <= 0.0 {
            // double jump
            me.air_jumps -= 1;
            me.vy = -(2.0 * GRAVITY * JUMP * 0.85).sqrt();
            w.ring(cx, me.y + SIZE, 55.0, Color::WHITE);
            w.burst(cx, me.y + SIZE, 8, Color::new(230, 230, 230, 200), 180.0, 0.35, 5.0, Shape::Circle, 100.0);
        }
    }
    if me.tap_timer > 0.0 {
        me.tap_timer -= dt;
        if me.tap_timer <= 0.0 {
            launch(me, 1.0);
        }
    }

    // ---- loadout abilities and ultimate ----
    let loadout = me.abilities;
    if acting && inp.a1 && me.cd1 <= 0.0 {
        use_ability(loadout[0], me, foe, my_idx, solids, w);
        me.cd1 = loadout[0].cooldown();
    }
    if acting && inp.a2 && me.cd2 <= 0.0 {
        use_ability(loadout[1], me, foe, my_idx, solids, w);
        me.cd2 = loadout[1].cooldown();
    }
    if acting && inp.ult && me.ult >= ULT_MAX {
        me.ult = 0.0;
        use_ult(me, foe, my_idx, w);
    }
    if me.slam_arm && me.vy >= 0.0 {
        me.slam_arm = false;
        me.slam = true;
    }
    if me.slam {
        me.vy = me.vy.max(SLAM_SPEED);
        if w.rng.next() < 0.8 {
            let q = Particle::new(cx + w.rng.range(-20.0, 20.0), cy - 30.0, 0.0, -200.0, 0.25, 3.0, Color::WHITE, Shape::Streak);
            w.parts.push(q);
        }
    }

    // ---- melee: knife swing ----
    me.melee_cd = (me.melee_cd - dt).max(0.0);
    me.melee_t = (me.melee_t - dt).max(0.0);
    if acting && inp.melee && me.melee_cd <= 0.0 && me.melee_t <= 0.0 {
        me.melee_t = MELEE_TIME;
        me.melee_cd = MELEE_CD;
        me.melee_hit = false;
    }

    step(me, solids, dt, w.grav_scale);

    if me.melee_t > 0.0 && !me.melee_hit {
        let progress = 1.0 - me.melee_t / MELEE_TIME;
        if (0.2..0.85).contains(&progress) {
            let rx = if me.facing > 0.0 { me.x + SIZE } else { me.x - MELEE_REACH };
            let zone = Rectangle::new(rx, me.y - 10.0, MELEE_REACH, SIZE + 20.0);
            if overlaps(&zone, &foe.rect()) {
                let dmg = me.kind.melee_dmg() * if me.over_t > 0.0 { 1.8 } else { 1.0 };
                foe.hurt(dmg, me.facing * 350.0, -180.0);
                me.melee_hit = true;
                let (fx, fy) = foe.center();
                w.burst(fx, fy, 8, Color::new(220, 240, 255, 255), 320.0, 0.25, 3.0, Shape::Streak, 0.0);
            }
        }
    }

    // ---- dash results ----
    if me.dash_t > 0.0 && !me.dash_hit && overlaps(&me.rect(), &foe.rect()) {
        foe.hurt(15.0, me.facing * 600.0, -250.0);
        me.dash_hit = true;
        me.dash_t = 0.0; // dash ends on impact so you don't keep shoving them
        me.vx *= 0.3;
        let (fx, fy) = foe.center();
        w.ring(fx, fy, 90.0, Color::new(255, 200, 100, 255));
        w.shake(8.0);
    }

    // ---- eyes ----
    me.eyes = if me.crouch > 0.0 || me.hurt_t > 0.0 || me.frozen_t > 0.0 || me.stun_t > 0.0 {
        Eyes::Squint
    } else if !me.on_ground && me.vy < 0.0 {
        Eyes::UpWide
    } else if push > 0.0 {
        Eyes::Right
    } else if push < 0.0 {
        Eyes::Left
    } else {
        Eyes::Forward
    };
}

// =====================================================================
// trails
// =====================================================================

fn emit_trail(p: &mut Player, w: &mut World, t: f32, dt: f32) {
    let style = TRAIL_STYLES[p.trail_style];
    if style == Trail::Off || p.dead_t > 0.0 {
        return;
    }
    let speed = (p.vx * p.vx + p.vy * p.vy).sqrt();
    if speed < 40.0 {
        p.trail_acc = 0.0;
        return;
    }
    p.trail_acc += dt;
    let every = if style == Trail::Ghost { 0.04 } else { 0.02 };
    while p.trail_acc >= every {
        p.trail_acc -= every;
        if w.parts.len() > 1600 {
            return;
        }
        let (cx, cy) = p.center();
        let col = trail_color(p.trail_color, t, p.kind.color(), w.rng.range(0.0, 40.0));
        let r = &mut w.rng;
        let q = match style {
            Trail::Dots => {
                let size = r.range(5.0, 10.0);
                Particle::new(cx + r.range(-12.0, 12.0), cy + r.range(-12.0, 12.0), r.range(-25.0, 25.0), r.range(-25.0, 25.0), 0.5, size, col, Shape::Circle)
            }
            Trail::Ghost => {
                let mut q = Particle::new(cx, cy, 0.0, 0.0, 0.3, SIZE, with_alpha(col, 120), Shape::Ghost);
                q.rot = p.rot;
                q
            }
            Trail::Sparkle => {
                let size = r.range(4.0, 9.0);
                Particle::new(cx + r.range(-20.0, 20.0), cy + r.range(-20.0, 20.0), r.range(-30.0, 30.0), r.range(-60.0, -10.0), 0.7, size, col, Shape::Star)
            }
            _ => {
                let size = r.range(8.0, 14.0);
                let mut q = Particle::new(cx + r.range(-10.0, 10.0), cy + r.range(-5.0, 15.0), r.range(-15.0, 15.0), r.range(-40.0, -15.0), 0.9, size, with_alpha(col, 140), Shape::Circle);
                q.grow = true;
                q
            }
        };
        w.parts.push(q);
    }
}

// =====================================================================
// drawing: players
// =====================================================================

// pixel-style knife, drawn from rectangles. (px, py) is the handle end,
// theta is degrees clockwise from straight up, flip = 1 or -1 (facing).
fn draw_knife(d: &mut impl RaylibDraw, px: f32, py: f32, theta: f32, flip: f32, scale: f32) {
    let steel = Color::new(130, 180, 200, 255);
    let wood = Color::new(140, 85, 40, 255);
    let dark_wood = Color::new(90, 55, 25, 255);
    // (x0, y0, x1, y1) in knife space: handle at y=0, blade toward negative y
    let parts = [
        (-6.0, -24.0, 6.0, 2.0, Color::BLACK),
        (-4.0, -22.0, 4.0, 0.0, wood),
        (-12.0, -30.0, 12.0, -22.0, Color::BLACK),
        (-10.0, -28.0, 10.0, -24.0, dark_wood),
        (-8.0, -80.0, 8.0, -28.0, Color::BLACK),
        (-5.0, -92.0, 5.0, -80.0, Color::BLACK),
        (-5.0, -78.0, 5.0, -30.0, steel),
        (1.0, -78.0, 5.0, -30.0, Color::WHITE),
        (-3.0, -90.0, 3.0, -78.0, steel),
    ];
    let (s, c) = theta.to_radians().sin_cos();
    for (x0, y0, x1, y1, col) in parts {
        let (w, h) = ((x1 - x0) * scale, (y1 - y0) * scale);
        let lx = (x0 + x1) / 2.0 * flip * scale;
        let ly = (y0 + y1) / 2.0 * scale;
        let cx = px + lx * c - ly * s;
        let cy = py + lx * s + ly * c;
        d.draw_rectangle_pro(Rectangle::new(cx, cy, w, h), Vector2::new(w / 2.0, h / 2.0), theta, col);
    }
}

fn draw_player(d: &mut impl RaylibDraw, p: &Player, t: f32) {
    let w = SIZE;
    let h = SIZE - p.crouch;
    let cx = p.x + SIZE / 2.0;
    let cy = p.y + SIZE - h / 2.0;
    let body = if p.hurt_t > 0.0 { Color::WHITE } else { p.kind.color() };

    // buff auras behind the body
    if p.power_t > 0.0 {
        d.draw_circle(cx as i32, cy as i32, SIZE * 0.85, Color::new(255, 70, 60, 60));
    }
    if p.haste_t > 0.0 {
        d.draw_circle(cx as i32, cy as i32, SIZE * 0.85, Color::new(60, 200, 255, 60));
    }
    if p.over_t > 0.0 {
        d.draw_circle(cx as i32, cy as i32, SIZE * 1.0, Color::new(255, 150, 40, 90));
    }
    if p.ult >= ULT_MAX {
        let pulse = 0.5 + 0.5 * (t * 8.0).sin();
        d.draw_circle_lines(cx as i32, cy as i32, SIZE * 0.8 + 6.0 * pulse, Color::new(255, 230, 80, 220));
    }

    d.draw_rectangle_pro(Rectangle::new(cx, cy, w, h), Vector2::new(w / 2.0, h / 2.0), p.rot, body);

    let (look_x, look_y, ew, eh) = match p.eyes {
        Eyes::Forward => (0.0, 0.0, 2.0, 2.0),
        Eyes::Left => (-2.0, 0.0, 2.0, 2.0),
        Eyes::Right => (2.0, 0.0, 2.0, 2.0),
        Eyes::UpWide => (0.0, -1.5, 3.0, 3.0),
        Eyes::Squint => (0.0, 0.0, 3.0, 1.0),
    };
    let eye_col = if p.hurt_t > 0.0 { Color::BLACK } else if p.kind == Kind::Ghost { Color::new(60, 50, 90, 255) } else { Color::WHITE };
    let big = SIZE / 15.0;
    let (s, c) = p.rot.to_radians().sin_cos();
    for side in [-1.0_f32, 1.0] {
        let ox = (side * 3.0 + look_x) * big;
        let oy = (-2.0 + look_y) * big - p.crouch * 0.3;
        let ex = cx + ox * c - oy * s;
        let ey = cy + ox * s + oy * c;
        d.draw_rectangle_pro(
            Rectangle::new(ex, ey, ew * big, eh * big),
            Vector2::new(ew * big / 2.0, eh * big / 2.0),
            p.rot,
            eye_col,
        );
    }

    if p.frozen_t > 0.0 {
        d.draw_rectangle_pro(Rectangle::new(cx, cy, w + 8.0, h + 8.0), Vector2::new(w / 2.0 + 4.0, h / 2.0 + 4.0), p.rot, Color::new(170, 230, 255, 160));
        d.draw_rectangle_pro(Rectangle::new(cx - 10.0, cy - 12.0, 8.0, 26.0), Vector2::new(4.0, 13.0), p.rot + 20.0, Color::new(255, 255, 255, 190));
    }
    if p.stun_t > 0.0 {
        // little stars circling the head
        for k in 0..3 {
            let a = t * 9.0 + k as f32 * 2.09;
            d.draw_circle((cx + a.cos() * 26.0) as i32, (p.y - 10.0 + a.sin() * 6.0) as i32, 5.0, Color::new(255, 230, 60, 255));
        }
    }
    if p.block_held {
        // guard arc in front of you; flashes white during the parry window
        let gx = cx + p.facing * SIZE * 0.62;
        let col = if p.parry_t > 0.0 { Color::WHITE } else { Color::new(200, 230, 255, 220) };
        d.draw_circle(gx as i32, cy as i32, 40.0, Color::new(200, 230, 255, 50));
        d.draw_circle_lines(gx as i32, cy as i32, 40.0, col);
        d.draw_circle_lines(gx as i32, cy as i32, 39.0, col);
    }
    if p.melee_t > 0.0 {
        let progress = 1.0 - p.melee_t / MELEE_TIME;
        let angle = (-60.0 + 150.0 * progress) * p.facing; // raised -> slashed down
        draw_knife(d, cx + p.facing * SIZE * 0.35, cy + SIZE * 0.1, angle, p.facing, 1.0);
        // slash arc
        if progress > 0.2 && progress < 0.9 {
            let a = (1.0 - progress) * 200.0;
            d.draw_circle_lines((cx + p.facing * SIZE * 0.35) as i32, (cy + SIZE * 0.1) as i32, 70.0, Color::new(255, 255, 255, a as u8));
        }
    }
    if p.shield_t > 0.0 {
        d.draw_circle(cx as i32, cy as i32, SIZE * 0.95, Color::new(120, 220, 255, 70));
        d.draw_circle_lines(cx as i32, cy as i32, SIZE * 0.95, Color::new(120, 220, 255, 255));
        // orbiting sparks
        for k in 0..6 {
            let a = p.shield_t * 6.0 + k as f32 * 1.047;
            d.draw_circle((cx + a.cos() * SIZE * 0.95) as i32, (cy + a.sin() * SIZE * 0.95) as i32, 4.0, Color::WHITE);
        }
    }
    if p.dash_t > 0.0 {
        for i in 1..4 {
            let tx = cx - p.facing * i as f32 * 25.0;
            d.draw_rectangle(
                (tx - w / 2.0) as i32,
                (cy - h / 2.0) as i32,
                w as i32,
                h as i32,
                Color::new(255, 165, 0, (110 - i * 30) as u8),
            );
        }
    }
}

fn draw_strikes(d: &mut impl RaylibDraw, strikes: &[Strike], t: f32) {
    let ground = H as f32 - FLOOR_H;
    for s in strikes {
        if !s.struck {
            // warning column that gets brighter until the bolt lands
            let prog = (1.0 - s.t / STRIKE_DELAY).clamp(0.0, 1.0);
            let pulse = 0.5 + 0.5 * (t * 25.0).sin();
            let alpha = (25.0 + 110.0 * prog + 40.0 * pulse) as u8;
            d.draw_rectangle((s.x - 35.0) as i32, 0, 70, ground as i32, Color::new(255, 240, 120, alpha));
            d.draw_rectangle((s.x - 2.0) as i32, 0, 4, ground as i32, Color::new(255, 255, 255, (120.0 + 120.0 * prog) as u8));
            d.draw_circle(s.x as i32, ground as i32 - 2, 20.0 + 25.0 * prog, Color::new(255, 240, 120, 120));
            text(d, "!", s.x as i32 - 8, 24, 44, Color::new(255, 70, 60, 255));
        } else {
            let a = (s.vis / 0.35).clamp(0.0, 1.0);
            for i in 1..s.pts.len() {
                d.draw_line_ex(s.pts[i - 1], s.pts[i], 22.0 * a + 2.0, Color::new(170, 210, 255, (110.0 * a) as u8));
                d.draw_line_ex(s.pts[i - 1], s.pts[i], 8.0 * a + 1.0, Color::new(255, 255, 255, (255.0 * a) as u8));
            }
        }
    }
}

// =====================================================================
// drawing: text and HUD
// =====================================================================

// readable on any background: colored text with a thick black outline
fn text(d: &mut impl RaylibDraw, s: &str, x: i32, y: i32, size: i32, color: Color) {
    for (ox, oy) in [(-2, 0), (2, 0), (0, -2), (0, 2), (-2, -2), (2, 2), (-2, 2), (2, -2)] {
        d.draw_text(s, x + ox, y + oy, size, Color::BLACK);
    }
    d.draw_text(s, x, y, size, color);
}

fn text_width(s: &str, size: i32) -> i32 {
    (s.len() as f32 * size as f32 * 0.55) as i32
}

fn center_text(d: &mut impl RaylibDraw, s: &str, y: i32, size: i32, color: Color) {
    text(d, s, W / 2 - text_width(s, size) / 2, y, size, color);
}

fn draw_hud(d: &mut impl RaylibDraw, players: &[Player; 2], mode: Mode, rd: &Round, t: f32, tags: &[Option<&str>; 2]) {
    let labels = [("F", "G"), ("COMMA", "PERIOD")];
    let ult_keys = ["V", "R-CTRL"];
    for (i, p) in players.iter().enumerate() {
        let bar_w = 420.0;
        let x = if i == 0 { 20.0 } else { W as f32 - 20.0 - bar_w };
        d.draw_rectangle(x as i32 - 3, 17, bar_w as i32 + 6, 30, Color::DARKGRAY);
        let fill = bar_w * (p.hp / p.max_hp).clamp(0.0, 1.0);
        let fx = if i == 0 { x } else { x + bar_w - fill };
        d.draw_rectangle(fx as i32, 20, fill as i32, 24, p.kind.color());
        d.draw_text(&format!("{:.0}", p.hp.ceil()), x as i32 + 6, 22, 20, Color::WHITE);
        let name = match tags[i] {
            Some(tag) => format!("P{} {} [CPU {}]", i + 1, p.kind.name(), tag),
            None => format!("P{} {}", i + 1, p.kind.name()),
        };
        text(d, &name, x as i32, 52, if tags[i].is_some() { 22 } else { 28 }, Color::WHITE);

        // stock dots
        if mode == Mode::Stock {
            for k in 0..STOCKS {
                let col = if k < rd.stocks[i] { Color::new(255, 90, 90, 255) } else { Color::new(60, 60, 60, 255) };
                let dx = if i == 0 { x + 300.0 + k as f32 * 24.0 } else { x + 330.0 - k as f32 * 24.0 };
                d.draw_circle(dx as i32, 66, 9.0, col);
            }
        }

        let cds = [
            (p.cd1, p.abilities[0].cooldown(), format!("{}: {}", labels[i].0, p.abilities[0].name())),
            (p.cd2, p.abilities[1].cooldown(), format!("{}: {}", labels[i].1, p.abilities[1].name())),
        ];
        for (k, (cd, max, label)) in cds.iter().enumerate() {
            let px = x + k as f32 * 215.0;
            d.draw_rectangle(px as i32 - 2, 82, 205, 32, Color::BLACK);
            let ready = 1.0 - cd / max;
            let col = if *cd <= 0.0 { Color::new(40, 160, 40, 255) } else { Color::GRAY };
            d.draw_rectangle(px as i32, 84, (201.0 * ready) as i32, 28, col);
            text(d, label, px as i32 + 8, 86, 22, Color::WHITE);
        }

        // ultimate meter
        d.draw_rectangle(x as i32 - 2, 118, 424, 18, Color::BLACK);
        let full = p.ult >= ULT_MAX;
        let ucol = if full { hsv(t * 300.0) } else { Color::new(230, 190, 40, 255) };
        d.draw_rectangle(x as i32, 120, (420.0 * p.ult / ULT_MAX) as i32, 14, ucol);
        let label = if full {
            format!("{}: {} READY!", ult_keys[i], p.kind.ult_name())
        } else {
            format!("ULTIMATE {:.0}%", p.ult)
        };
        text(d, &label, x as i32 + 6, 118, 16, Color::WHITE);

        // active buffs and combo
        let mut by = 142;
        if p.power_t > 0.0 {
            text(d, &format!("POWER {:.0}s", p.power_t.ceil()), x as i32, by, 20, Color::new(255, 90, 80, 255));
            by += 22;
        }
        if p.haste_t > 0.0 {
            text(d, &format!("HASTE {:.0}s", p.haste_t.ceil()), x as i32, by, 20, Color::new(80, 210, 255, 255));
            by += 22;
        }
        if rd.combo_show[i] > 0.0 && rd.combo[i] >= 2 {
            text(d, &format!("x{} COMBO!", rd.combo[i]), x as i32, by, 30, Color::YELLOW);
        }
    }
}

// =====================================================================
// drawing: scenery and themes
// =====================================================================

fn draw_scenery(d: &mut impl RaylibDraw, theme: Theme, t: f32) {
    let ground = H as f32 - FLOOR_H;
    match theme {
        Theme::Meadow => {
            d.draw_rectangle_gradient_v(0, 0, W, H, Color::new(90, 160, 235, 255), Color::new(205, 232, 255, 255));
            // sun with glow
            for (r, a) in [(130.0, 25u8), (95.0, 45), (62.0, 255)] {
                let col = if a == 255 { Color::new(255, 232, 110, 255) } else { Color::new(255, 240, 150, a) };
                d.draw_circle(1070, 120, r, col);
            }
            // drifting clouds
            for i in 0..5 {
                let x = ((i as f32 * 320.0 + t * 14.0) % (W as f32 + 300.0)) - 150.0;
                let y = 70.0 + ((i * 53) % 150) as f32;
                let white = Color::new(255, 255, 255, 230);
                d.draw_circle(x as i32, y as i32, 30.0, white);
                d.draw_circle((x + 34.0) as i32, (y - 12.0) as i32, 38.0, white);
                d.draw_circle((x + 72.0) as i32, y as i32, 28.0, white);
                d.draw_rectangle(x as i32, y as i32, 72, 28, white);
            }
            // rolling hills, far then near
            d.draw_ellipse(180, (ground + 40.0) as i32, 420.0, 230.0, Color::new(120, 185, 130, 255));
            d.draw_ellipse(700, (ground + 60.0) as i32, 520.0, 190.0, Color::new(105, 175, 120, 255));
            d.draw_ellipse(1150, (ground + 40.0) as i32, 380.0, 220.0, Color::new(120, 185, 130, 255));
            d.draw_ellipse(430, (ground + 30.0) as i32, 380.0, 110.0, Color::new(70, 150, 80, 255));
            d.draw_ellipse(960, (ground + 30.0) as i32, 400.0, 130.0, Color::new(65, 145, 75, 255));
            // trees
            for tx in [95.0, 310.0, 1010.0, 1195.0] {
                d.draw_rectangle(tx as i32 - 8, (ground - 70.0) as i32, 16, 70, Color::new(110, 70, 40, 255));
                d.draw_circle(tx as i32, (ground - 95.0) as i32, 42.0, Color::new(40, 120, 50, 255));
                d.draw_circle(tx as i32 - 22, (ground - 70.0) as i32, 30.0, Color::new(50, 135, 60, 255));
                d.draw_circle(tx as i32 + 22, (ground - 72.0) as i32, 30.0, Color::new(50, 135, 60, 255));
            }
            // flowers
            for i in 0..30 {
                let fx = (i * 43 + 11) as i32;
                let col = match i % 3 {
                    0 => Color::new(235, 60, 70, 255),
                    1 => Color::new(255, 220, 60, 255),
                    _ => Color::new(255, 130, 200, 255),
                };
                d.draw_rectangle(fx, ground as i32 - 10, 2, 10, Color::new(50, 130, 50, 255));
                d.draw_circle(fx + 1, ground as i32 - 12, 4.0, col);
            }
        }
        Theme::Skyline => {
            d.draw_rectangle_gradient_v(0, 0, W, H, Color::new(8, 10, 38, 255), Color::new(75, 40, 115, 255));
            // twinkling stars
            for i in 0..80 {
                let sx = (i * 977) % W;
                let sy = (i * 389) % (H - 260);
                let tw = 150.0 + 100.0 * (t * 2.0 + i as f32).sin();
                let sz = if i % 5 == 0 { 3 } else { 2 };
                d.draw_rectangle(sx, sy, sz, sz, Color::new(255, 255, 230, tw as u8));
            }
            // moon
            d.draw_circle(190, 130, 85.0, Color::new(240, 240, 210, 25));
            d.draw_circle(190, 130, 56.0, Color::new(240, 240, 210, 255));
            for (cx, cy, r) in [(172, 115, 11.0), (210, 140, 8.0), (185, 150, 6.0), (205, 105, 5.0)] {
                d.draw_circle(cx, cy, r, Color::new(205, 205, 175, 255));
            }
            // two layers of skyline silhouettes with lit windows
            for layer in 0..2 {
                let base = if layer == 0 { Color::new(35, 32, 85, 255) } else { Color::new(22, 22, 60, 255) };
                let count = 13;
                for i in 0..count {
                    let bw = 80 + ((i * 37 + layer * 11) % 50);
                    let bh = 130 + ((i * 53 + layer * 71) % 190) + layer * 40;
                    let bx = i * 100 - 20 + layer * 45;
                    let by = ground as i32 - bh;
                    d.draw_rectangle(bx, by, bw, bh, base);
                    for wy in 0..(bh / 26) {
                        for wx in 0..(bw / 22) {
                            let lit = (wx * 7 + wy * 13 + i * 5 + layer) % 3 != 0;
                            if lit {
                                let blink = if (wx + wy + i) % 11 == 0 && (t as i32 + i) % 4 == 0 { 40 } else { 200 };
                                d.draw_rectangle(bx + 8 + wx * 22, by + 10 + wy * 26, 10, 14, Color::new(255, 220, 110, blink));
                            }
                        }
                    }
                }
            }
        }
        Theme::Volcano => {
            d.draw_rectangle_gradient_v(0, 0, W, H, Color::new(40, 6, 10, 255), Color::new(235, 105, 30, 255));
            let v = |x: f32, y: f32| Vector2::new(x, y);
            // side mountains
            tri(d, v(-60.0, ground), v(300.0, ground), v(110.0, 360.0), Color::new(70, 30, 30, 255));
            tri(d, v(980.0, ground), v(1340.0, ground), v(1170.0, 330.0), Color::new(70, 30, 30, 255));
            // the volcano
            tri(d, v(130.0, ground), v(1150.0, ground), v(640.0, 190.0), Color::new(42, 24, 30, 255));
            d.draw_ellipse(640, 195, 78.0, 18.0, Color::new(255, 150, 40, 255));
            d.draw_ellipse(640, 197, 56.0, 11.0, Color::new(255, 220, 90, 255));
            for (x1, y1, x2, y2) in [(610.0, 205.0, 520.0, 520.0), (650.0, 205.0, 740.0, 470.0), (635.0, 205.0, 640.0, 600.0)] {
                d.draw_line_ex(v(x1, y1), v(x2, y2), 5.0, Color::new(255, 120, 30, 220));
            }
            // smoke plume
            for i in 0..7 {
                let sx = 640.0 + (t * 0.5 + i as f32).sin() * 25.0 + i as f32 * 14.0;
                let sy = 165.0 - i as f32 * 32.0;
                d.draw_circle(sx as i32, sy as i32, 30.0 + i as f32 * 8.0, Color::new(60, 50, 55, (170 - i * 20) as u8));
            }
            // rising embers
            for i in 0..45 {
                let ex = (i * 131) as f32 + (t * 0.7 + i as f32).sin() * 30.0;
                let ey = H as f32 - ((t * 40.0 + (i * 57) as f32) % H as f32);
                d.draw_circle((ex.rem_euclid(W as f32)) as i32, ey as i32, 2.0, Color::new(255, 170, 50, 210));
            }
        }
        Theme::Frozen => {
            d.draw_rectangle_gradient_v(0, 0, W, H, Color::new(12, 22, 70, 255), Color::new(150, 200, 235, 255));
            for i in 0..40 {
                let tw = 120.0 + 100.0 * (t * 2.0 + i as f32).sin();
                d.draw_rectangle((i * 541) % W, (i * 263) % 250, 2, 2, Color::new(255, 255, 255, tw as u8));
            }
            // aurora curtains
            for i in 0..58 {
                let x = i * 22;
                for band in 0..2 {
                    let bf = band as f32;
                    let y = 70.0 + bf * 45.0 + 28.0 * (t * 0.6 + i as f32 * 0.22 + bf).sin();
                    let h = 70.0 + 30.0 * (t * 0.4 + i as f32 * 0.15).sin();
                    let c = if band == 0 { Color::new(70, 255, 160, 255) } else { Color::new(160, 100, 255, 255) };
                    d.draw_rectangle_gradient_v(x, y as i32, 24, h as i32, with_alpha(c, 0), with_alpha(c, 70));
                }
            }
            let v = |x: f32, y: f32| Vector2::new(x, y);
            let snow = Color::new(240, 248, 255, 255);
            tri(d, v(-120.0, ground), v(420.0, ground), v(150.0, 250.0), Color::new(140, 170, 210, 255));
            tri(d, v(150.0, 250.0), v(95.0, 340.0), v(205.0, 340.0), snow);
            tri(d, v(300.0, ground), v(900.0, ground), v(600.0, 190.0), Color::new(120, 155, 200, 255));
            tri(d, v(600.0, 190.0), v(535.0, 290.0), v(665.0, 290.0), snow);
            tri(d, v(760.0, ground), v(1400.0, ground), v(1090.0, 230.0), Color::new(140, 170, 210, 255));
            tri(d, v(1090.0, 230.0), v(1030.0, 320.0), v(1150.0, 320.0), snow);
            d.draw_ellipse(300, (ground + 50.0) as i32, 420.0, 90.0, Color::new(225, 238, 250, 255));
            d.draw_ellipse(980, (ground + 50.0) as i32, 460.0, 100.0, Color::new(225, 238, 250, 255));
            for px in [70.0_f32, 230.0, 1050.0, 1215.0] {
                let green = Color::new(25, 80, 70, 255);
                tri(d, v(px, ground - 130.0), v(px - 38.0, ground - 75.0), v(px + 38.0, ground - 75.0), green);
                tri(d, v(px, ground - 100.0), v(px - 48.0, ground - 35.0), v(px + 48.0, ground - 35.0), green);
                tri(d, v(px, ground - 65.0), v(px - 58.0, ground), v(px + 58.0, ground), green);
                tri(d, v(px, ground - 130.0), v(px - 14.0, ground - 112.0), v(px + 14.0, ground - 112.0), snow);
            }
            // falling snow
            for i in 0..90 {
                let fx = ((i * 97) as f32 + (t * 0.8 + i as f32).sin() * 30.0).rem_euclid(W as f32);
                let fy = (t * 70.0 + (i * 41) as f32) % H as f32;
                d.draw_circle(fx as i32, fy as i32, 1.5 + (i % 3) as f32, Color::new(255, 255, 255, 220));
            }
        }
        Theme::Space => {
            d.draw_rectangle_gradient_v(0, 0, W, H, Color::new(2, 2, 16, 255), Color::new(45, 12, 70, 255));
            for (cx, cy, r, col) in [
                (300, 220, 230.0, Color::new(120, 40, 170, 26)),
                (900, 160, 260.0, Color::new(200, 60, 120, 20)),
                (1050, 470, 220.0, Color::new(60, 90, 200, 24)),
                (200, 520, 200.0, Color::new(60, 160, 170, 18)),
            ] {
                d.draw_circle(cx, cy, r, col);
                d.draw_circle(cx, cy, r * 0.65, col);
            }
            for i in 0..150 {
                let tw = 140.0 + 100.0 * (t * 2.0 + i as f32).sin();
                let sz = if i % 11 == 0 { 3 } else if i % 4 == 0 { 2 } else { 1 };
                d.draw_rectangle((i * 677) % W, (i * 353) % (H - 40), sz, sz, Color::new(255, 255, 255, tw as u8));
            }
            // ringed planet
            d.draw_circle(930, 170, 115.0, Color::new(205, 125, 75, 255));
            for k in 0..5 {
                let y = 170.0 + (k as f32 - 2.0) * 34.0;
                let rh = (115.0f32 * 115.0 - (y - 170.0) * (y - 170.0)).max(0.0).sqrt();
                let band = if k % 2 == 0 { Color::new(225, 160, 100, 255) } else { Color::new(180, 100, 60, 255) };
                d.draw_ellipse(930, y as i32, rh, 9.0, band);
            }
            for (rh, rv) in [(200.0, 38.0), (185.0, 34.0), (170.0, 30.0)] {
                d.draw_ellipse_lines(930, 175, rh, rv, Color::new(230, 200, 160, 190));
            }
            // small moon
            d.draw_circle(170, 120, 38.0, Color::new(205, 205, 215, 255));
            d.draw_circle(158, 110, 8.0, Color::new(170, 170, 185, 255));
            d.draw_circle(182, 132, 6.0, Color::new(170, 170, 185, 255));
            // shooting star
            let sx = (t * 350.0) % (W as f32 + 500.0) - 250.0;
            let sy = 60.0 + (t * 90.0) % 260.0;
            d.draw_line_ex(Vector2::new(sx, sy), Vector2::new(sx - 120.0, sy - 40.0), 3.0, Color::new(255, 255, 255, 200));
            // station silhouette along the bottom
            for i in 0..14 {
                let bw = 60 + (i * 31) % 40;
                let bh = 40 + (i * 37) % 70;
                let x = i * 95 - 10;
                d.draw_rectangle(x, ground as i32 - bh, bw, bh, Color::new(25, 28, 52, 255));
                if ((t * 2.0 + i as f32) % 2.0) < 1.0 {
                    d.draw_rectangle(x + bw / 2, ground as i32 - bh - 8, 4, 8, Color::new(255, 60, 60, 255));
                }
            }
        }
        Theme::Sea => {
            d.draw_rectangle_gradient_v(0, 0, W, H, Color::new(30, 125, 175, 255), Color::new(4, 18, 48, 255));
            let v = |x: f32, y: f32| Vector2::new(x, y);
            for i in 0..7 {
                let x = 60.0 + i as f32 * 190.0 + (t * 0.3 + i as f32).sin() * 40.0;
                tri(d, v(x, 0.0), v(x + 60.0, 0.0), v(x + 170.0, ground), Color::new(200, 240, 255, 16));
            }
            d.draw_ellipse(200, (ground + 20.0) as i32, 280.0, 140.0, Color::new(10, 50, 80, 255));
            d.draw_ellipse(930, (ground + 20.0) as i32, 320.0, 150.0, Color::new(10, 50, 80, 255));
            // fish swimming by
            for i in 0..6 {
                let x = (t * (50.0 + i as f32 * 8.0) + i as f32 * 230.0) % (W as f32 + 240.0) - 120.0;
                let y = 130.0 + i as f32 * 68.0 + (t * 1.5 + i as f32).sin() * 12.0;
                let col = match i % 3 {
                    0 => Color::new(255, 170, 60, 230),
                    1 => Color::new(255, 220, 90, 230),
                    _ => Color::new(110, 190, 255, 230),
                };
                d.draw_ellipse(x as i32, y as i32, 18.0, 9.0, col);
                tri(d, v(x - 14.0, y), v(x - 32.0, y - 9.0), v(x - 32.0, y + 9.0), col);
                d.draw_circle(x as i32 + 9, y as i32 - 2, 2.0, Color::BLACK);
            }
            // jellyfish
            for i in 0..3 {
                let jx = 250.0 + i as f32 * 380.0;
                let jy = 200.0 + (t + i as f32).sin() * 40.0;
                d.draw_circle(jx as i32, jy as i32, 22.0, Color::new(255, 150, 220, 120));
                for k in -2..=2 {
                    let kx = jx + k as f32 * 7.0;
                    d.draw_line_ex(v(kx, jy + 10.0), v(kx + (t * 2.0 + k as f32).sin() * 6.0, jy + 52.0), 2.0, Color::new(255, 170, 230, 120));
                }
            }
            // bubbles
            for i in 0..40 {
                let bx = ((i * 83) as f32 + (t + i as f32).sin() * 15.0).rem_euclid(W as f32);
                let by = H as f32 - ((t * 35.0 + (i * 47) as f32) % H as f32);
                d.draw_circle_lines(bx as i32, by as i32, 2.0 + (i % 4) as f32, Color::new(220, 245, 255, 170));
            }
            // swaying seaweed
            for x in (0..W).step_by(60) {
                for s in 0..5 {
                    let sx = x as f32 + (t * 1.4 + x as f32 * 0.04 + s as f32 * 0.6).sin() * 5.0 * s as f32;
                    d.draw_rectangle(sx as i32, ground as i32 - 18 * (s + 1), 8, 18, Color::new(40, 150, 90, 255));
                }
            }
        }
        Theme::Sand => {
            d.draw_rectangle_gradient_v(0, 0, W, H, Color::new(255, 160, 80, 255), Color::new(250, 225, 165, 255));
            d.draw_circle(640, 250, 150.0, Color::new(255, 230, 170, 50));
            d.draw_circle(640, 250, 105.0, Color::new(255, 240, 200, 120));
            d.draw_circle(640, 250, 70.0, Color::new(255, 250, 230, 255));
            let v = |x: f32, y: f32| Vector2::new(x, y);
            let lit = Color::new(214, 165, 95, 255);
            let shade = Color::new(180, 132, 72, 255);
            tri(d, v(80.0, ground), v(520.0, ground), v(300.0, 300.0), lit);
            tri(d, v(300.0, 300.0), v(520.0, ground), v(300.0, ground), shade);
            tri(d, v(780.0, ground), v(1260.0, ground), v(1020.0, 350.0), lit);
            tri(d, v(1020.0, 350.0), v(1260.0, ground), v(1020.0, ground), shade);
            d.draw_ellipse(160, (ground + 30.0) as i32, 420.0, 100.0, Color::new(236, 192, 122, 255));
            d.draw_ellipse(700, (ground + 40.0) as i32, 560.0, 110.0, Color::new(226, 180, 108, 255));
            d.draw_ellipse(1180, (ground + 30.0) as i32, 420.0, 105.0, Color::new(236, 192, 122, 255));
            d.draw_ellipse(420, (ground + 36.0) as i32, 380.0, 70.0, Color::new(214, 168, 98, 255));
            d.draw_ellipse(950, (ground + 36.0) as i32, 380.0, 70.0, Color::new(214, 168, 98, 255));
            for cx in [60.0_f32, 390.0, 1220.0] {
                let green = Color::new(50, 140, 70, 255);
                d.draw_rectangle(cx as i32 - 7, ground as i32 - 70, 14, 70, green);
                d.draw_rectangle(cx as i32 - 24, ground as i32 - 48, 17, 8, green);
                d.draw_rectangle(cx as i32 - 24, ground as i32 - 60, 8, 20, green);
                d.draw_rectangle(cx as i32 + 7, ground as i32 - 38, 17, 8, green);
                d.draw_rectangle(cx as i32 + 16, ground as i32 - 52, 8, 22, green);
            }
            for i in 0..4 {
                let bx = (t * 70.0 + i as f32 * 300.0) % (W as f32 + 100.0) - 50.0;
                let by = 100.0 + i as f32 * 30.0 + (t * 3.0 + i as f32).sin() * 8.0;
                d.draw_line_ex(v(bx - 10.0, by), v(bx, by + 5.0), 2.0, Color::new(70, 50, 40, 255));
                d.draw_line_ex(v(bx, by + 5.0), v(bx + 10.0, by), 2.0, Color::new(70, 50, 40, 255));
            }
            for i in 0..25 {
                let x = (t * 180.0 + i as f32 * 53.0) % W as f32;
                let y = ground - 10.0 - ((i * 13) % 160) as f32;
                d.draw_line(x as i32, y as i32, x as i32 + 14, y as i32 + 1, Color::new(255, 235, 190, 150));
            }
        }
        Theme::Neon => {
            d.draw_rectangle_gradient_v(0, 0, W, H, Color::new(14, 0, 34, 255), Color::new(105, 0, 105, 255));
            for i in 0..60 {
                let tw = 120.0 + 100.0 * (t * 2.0 + i as f32).sin();
                d.draw_rectangle((i * 613) % W, (i * 197) % 260, 2, 2, Color::new(255, 255, 255, tw as u8));
            }
            // retro sun with scanline stripes
            d.draw_circle_gradient(640, 320, 170.0, Color::new(255, 230, 90, 255), Color::new(255, 40, 150, 255));
            for k in 0..7 {
                let y = 330.0 + k as f32 * 24.0;
                let half = (170.0f32 * 170.0 - (y - 320.0) * (y - 320.0)).max(0.0).sqrt();
                d.draw_rectangle((640.0 - half) as i32, y as i32, (half * 2.0) as i32, 3 + k * 2, Color::new(50, 0, 70, 255));
            }
            // neon skyline
            for i in 0..12 {
                let bh = 100 + (i * 47) % 150;
                let x = i * 108 - 5;
                let neon = if i % 2 == 0 { Color::new(0, 240, 255, 255) } else { Color::new(255, 60, 200, 255) };
                d.draw_rectangle(x, 430 - bh, 90, bh, Color::new(18, 4, 40, 255));
                d.draw_rectangle_lines(x, 430 - bh, 90, bh, neon);
            }
            // perspective grid floor
            d.draw_rectangle_gradient_v(0, 430, W, H - 430, Color::new(30, 0, 60, 255), Color::new(90, 0, 120, 255));
            for i in -14..=14 {
                d.draw_line(640 + i * 28, 430, 640 + i * 190, H, Color::new(0, 240, 255, 110));
            }
            for k in 0..9 {
                let f = ((k as f32 + (t * 0.5) % 1.0) / 9.0).powi(2);
                let y = 430.0 + f * (H as f32 - 430.0);
                d.draw_line(0, y as i32, W, y as i32, Color::new(255, 60, 200, 100));
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_world(d: &mut impl RaylibDraw, theme: Theme, map: &Map, hazard: Option<Rectangle>, zone: Option<Rectangle>, w: &World, t: f32) {
    for s in &map.solids {
        let r = s.r;
        let (x, y, wd, h) = (r.x as i32, r.y as i32, r.width as i32, r.height as i32);
        match theme {
            Theme::Meadow => {
                d.draw_rectangle(x, y, wd, h, Color::new(115, 80, 50, 255));
                d.draw_rectangle(x, y, wd, 9, Color::new(85, 175, 65, 255));
            }
            Theme::Skyline => {
                d.draw_rectangle(x, y, wd, h, Color::new(28, 32, 62, 255));
                d.draw_rectangle(x, y, wd, 5, Color::new(100, 110, 170, 255));
                if h >= 100 && wd < W - 10 {
                    for wy in 0..((h - 14) / 28) {
                        for wx in 0..((wd - 10) / 24) {
                            if (wx * 7 + wy * 13 + x / 10) % 3 != 0 {
                                d.draw_rectangle(x + 10 + wx * 24, y + 16 + wy * 28, 12, 16, Color::new(255, 215, 100, 255));
                            }
                        }
                    }
                }
            }
            Theme::Volcano => {
                d.draw_rectangle(x, y, wd, h, Color::new(52, 36, 38, 255));
                d.draw_rectangle(x, y, wd, 5, Color::new(215, 90, 25, 255));
            }
            Theme::Frozen => {
                d.draw_rectangle(x, y, wd, h, Color::new(150, 195, 225, 255));
                d.draw_rectangle(x, y, wd, 8, Color::new(240, 250, 255, 255));
                d.draw_rectangle(x + 4, y + 10, wd - 8, 2, Color::new(200, 230, 250, 255));
            }
            Theme::Space => {
                d.draw_rectangle(x, y, wd, h, Color::new(48, 52, 80, 255));
                d.draw_rectangle(x, y, wd, 4, Color::new(110, 200, 255, 255));
                d.draw_rectangle_lines(x, y, wd, h, Color::new(110, 200, 255, 90));
            }
            Theme::Sea => {
                d.draw_rectangle(x, y, wd, h, Color::new(55, 85, 100, 255));
                d.draw_rectangle(x, y, wd, 6, Color::new(90, 175, 150, 255));
                for k in 0..(wd / 36) {
                    d.draw_circle(x + 18 + k * 36, y + 3, 3.0, Color::new(240, 140, 160, 255));
                }
            }
            Theme::Sand => {
                d.draw_rectangle(x, y, wd, h, Color::new(186, 146, 88, 255));
                d.draw_rectangle(x, y, wd, 8, Color::new(232, 205, 135, 255));
                let mut ly = y + 20;
                while ly < y + h {
                    d.draw_rectangle(x, ly, wd, 2, Color::new(160, 120, 70, 255));
                    ly += 20;
                }
            }
            Theme::Neon => {
                d.draw_rectangle(x, y, wd, h, Color::new(22, 8, 48, 255));
                d.draw_rectangle(x, y, wd, 4, Color::new(255, 60, 200, 255));
                d.draw_rectangle_lines(x, y, wd, h, Color::new(0, 255, 255, 160));
            }
        }
    }
    if let Some(hz) = hazard {
        let pulse = 0.5 + 0.5 * (t * 4.0).sin();
        d.draw_rectangle((hz.x - 6.0) as i32, hz.y as i32 - 6, hz.width as i32 + 12, hz.height as i32 + 10, Color::new(255, (90.0 + 60.0 * pulse) as u8, 20, 255));
        d.draw_rectangle(hz.x as i32, hz.y as i32 - 3, hz.width as i32, 8, Color::new(255, 220, 90, 230));
        for i in 0..8 {
            let bx = hz.x + 20.0 + i as f32 * 40.0;
            let by = hz.y - 6.0 - ((t * 20.0 + i as f32 * 9.0) % 14.0);
            d.draw_circle(bx as i32, by as i32, 3.0, Color::new(255, 200, 60, 220));
        }
    }
    // king of the hill zone
    if let Some(z) = zone {
        let pulse = 0.5 + 0.5 * (t * 4.0).sin();
        d.draw_rectangle(z.x as i32, z.y as i32, z.width as i32, z.height as i32, Color::new(255, 220, 60, (50.0 + 40.0 * pulse) as u8));
        d.draw_rectangle_lines(z.x as i32, z.y as i32, z.width as i32, z.height as i32, Color::new(255, 230, 90, 255));
        text(d, "HILL", (z.x + z.width / 2.0) as i32 - 30, z.y as i32 + 6, 26, Color::new(255, 240, 120, 255));
    }
    // meteor warnings and falling meteors
    for m in &w.meteors {
        if !m.falling {
            let ground = H as f32 - FLOOR_H;
            let prog = (1.0 - m.warn).clamp(0.0, 1.0);
            let pulse = 0.5 + 0.5 * (t * 20.0).sin();
            d.draw_line_ex(Vector2::new(m.tx - 300.0, -60.0), Vector2::new(m.tx, ground), 2.0, Color::new(255, 80, 60, (60.0 + 100.0 * prog) as u8));
            d.draw_circle(m.tx as i32, ground as i32 - 2, 18.0 + 22.0 * prog, Color::new(255, 70, 50, (80.0 + 80.0 * pulse) as u8));
            text(d, "!", m.tx as i32 - 7, ground as i32 - 70, 40, Color::new(255, 90, 60, 255));
        } else {
            d.draw_circle(m.x as i32, m.y as i32, 26.0, Color::new(255, 120, 30, 110));
            d.draw_circle(m.x as i32, m.y as i32, 16.0, Color::new(90, 60, 50, 255));
            d.draw_circle(m.x as i32 - 4, m.y as i32 - 4, 7.0, Color::new(255, 170, 60, 255));
        }
    }
    // laser beams: red warning line first, then the beam
    for l in &w.lasers {
        if !l.fired {
            let prog = (1.0 - l.t / 1.1).clamp(0.0, 1.0);
            let pulse = 0.5 + 0.5 * (t * 20.0).sin();
            d.draw_rectangle(0, (l.y - 22.0 * prog) as i32, W, (44.0 * prog) as i32 + 1, Color::new(255, 60, 120, (30.0 + 50.0 * pulse) as u8));
            d.draw_rectangle(0, l.y as i32 - 2, W, 4, Color::new(255, 60, 80, (80.0 + 150.0 * prog) as u8));
            text(d, "!", 14, l.y as i32 - 22, 44, Color::new(255, 80, 80, 255));
            text(d, "!", W - 34, l.y as i32 - 22, 44, Color::new(255, 80, 80, 255));
        } else {
            let a = (l.vis / 0.4).clamp(0.0, 1.0);
            d.draw_rectangle(0, (l.y - 26.0 * a) as i32, W, (52.0 * a) as i32 + 1, Color::new(255, 80, 220, (130.0 * a) as u8));
            d.draw_rectangle(0, (l.y - 9.0 * a) as i32, W, (18.0 * a) as i32 + 1, Color::new(255, 255, 255, (255.0 * a) as u8));
        }
    }
    draw_strikes(d, &w.strikes, t);
    // pickups bob in place
    for pk in &w.pickups {
        let bob = (pk.age * 4.0).sin() * 5.0;
        let c = pk.kind.color();
        let (px, py) = (pk.x as i32, (pk.y + bob) as i32);
        d.draw_circle(px, py, 30.0, with_alpha(c, 70));
        d.draw_circle(px, py, 20.0, c);
        d.draw_circle_lines(px, py, 20.0, Color::BLACK);
        text(d, pk.kind.label(), px - 8, py - 13, 28, Color::WHITE);
    }
    for b in &w.projs {
        match b.kind {
            ProjKind::Bomb => {
                d.draw_circle(b.x as i32, b.y as i32, 9.0, Color::BLACK);
                d.draw_circle(b.x as i32, b.y as i32, 4.0, Color::RED);
            }
            ProjKind::Fire => {
                d.draw_circle(b.x as i32, b.y as i32, 16.0, Color::new(255, 120, 30, 120));
                d.draw_circle(b.x as i32, b.y as i32, 10.0, Color::new(255, 170, 40, 255));
                d.draw_circle(b.x as i32, b.y as i32, 5.0, Color::new(255, 245, 170, 255));
            }
            ProjKind::Frost => {
                d.draw_circle(b.x as i32, b.y as i32, 15.0, Color::new(150, 230, 255, 110));
                d.draw_rectangle_pro(Rectangle::new(b.x, b.y, 18.0, 18.0), Vector2::new(9.0, 9.0), 45.0 + t * 200.0, Color::new(200, 245, 255, 255));
            }
        }
    }
    for b in &w.booms {
        let p = (b.t / BOOM_TIME).min(1.0);
        let r = b.r * (0.4 + 0.6 * p);
        let fade = (255.0 * (1.0 - p)) as u8;
        d.draw_circle(b.x as i32, b.y as i32, r, with_alpha(b.color, fade / 4));
        d.draw_circle_lines(b.x as i32, b.y as i32, r, with_alpha(b.color, fade));
        d.draw_circle_lines(b.x as i32, b.y as i32, r * 0.85, with_alpha(Color::WHITE, fade / 2));
    }
}

// =====================================================================
// death animations: 10 + random (the loser picks how they go out)
// =====================================================================

const DEATH_NAMES: [&str; 11] = [
    "SHATTER",
    "EXPLODE",
    "DISINTEGRATE",
    "ASCEND",
    "LIGHTNING",
    "SPIN OFF",
    "CONFETTI POP",
    "FREEZE & CRACK",
    "BLACK HOLE",
    "GLITCH",
    "RANDOM",
];

const DEATH_BLURBS: [&str; 11] = [
    "bursts into spinning shards",
    "blows up in a fireball",
    "turns to dust and blows away",
    "a ghost floats up to the sky",
    "vaporized by a lightning bolt",
    "spins off into the distance",
    "pops like a balloon",
    "freezes solid, then cracks",
    "sucked into a black hole",
    "RGB glitch and static",
    "a surprise every time",
];

#[allow(clippy::too_many_arguments)]
fn put(w: &mut World, x: f32, y: f32, vx: f32, vy: f32, life: f32, size: f32, col: Color, shape: Shape, grav: f32, spin: f32) {
    if w.parts.len() > 2000 {
        return;
    }
    let mut q = Particle::new(x, y, vx, vy, life, size, col, shape);
    q.grav = grav;
    q.spin = spin;
    w.parts.push(q);
}

fn play_death(choice: usize, x: f32, y: f32, col: Color, w: &mut World) {
    let idx = if choice >= 10 { (w.rng.next() * 10.0) as usize % 10 } else { choice };
    w.ring(x, y, 220.0, Color::WHITE);
    w.fx.flash = w.fx.flash.max(0.3);
    w.shake(16.0);
    match idx {
        0 => {
            // SHATTER
            w.burst(x, y, 36, col, 600.0, 1.1, 14.0, Shape::Square, 500.0);
            w.burst(x, y, 20, Color::WHITE, 700.0, 0.6, 7.0, Shape::Star, 0.0);
            w.burst(x, y, 20, Color::WHITE, 800.0, 0.5, 3.0, Shape::Streak, 0.0);
            w.ring(x, y, 170.0, col);
        }
        1 => {
            // EXPLODE
            w.ring(x, y, 290.0, Color::new(255, 150, 40, 255));
            w.ring(x, y, 170.0, Color::new(255, 255, 200, 255));
            w.burst(x, y, 44, Color::new(255, 170, 40, 255), 650.0, 0.7, 10.0, Shape::Circle, 0.0);
            w.burst(x, y, 16, Color::new(255, 240, 160, 255), 500.0, 0.5, 6.0, Shape::Star, 0.0);
            w.burst(x, y, 18, col, 500.0, 0.9, 10.0, Shape::Square, 800.0);
            for _ in 0..14 {
                let (px, py) = (x + w.rng.range(-30.0, 30.0), y + w.rng.range(-30.0, 30.0));
                let (vx, vy) = (w.rng.range(-60.0, 60.0), w.rng.range(-120.0, -30.0));
                let size = w.rng.range(14.0, 24.0);
                let mut q = Particle::new(px, py, vx, vy, 1.2, size, Color::new(60, 55, 55, 170), Shape::Circle);
                q.grow = true;
                w.parts.push(q);
            }
            w.shake(28.0);
            w.fx.flash = w.fx.flash.max(0.6);
        }
        2 => {
            // DISINTEGRATE: dust blown off to the right
            for _ in 0..80 {
                let (px, py) = (x + w.rng.range(-30.0, 30.0), y + w.rng.range(-30.0, 30.0));
                let (vx, vy) = (w.rng.range(40.0, 170.0), w.rng.range(-90.0, -10.0));
                let (life, size, spin) = (w.rng.range(1.4, 2.4), w.rng.range(4.0, 9.0), w.rng.range(-200.0, 200.0));
                put(w, px, py, vx, vy, life, size, with_alpha(col, 230), Shape::Square, -15.0, spin);
            }
        }
        3 => {
            // ASCEND: a pale ghost with a halo drifts upward
            put(w, x, y, 0.0, -150.0, 1.6, SIZE, Color::new(235, 235, 255, 170), Shape::Ghost, 0.0, 0.0);
            for k in 0..12 {
                let a = k as f32 * 0.5236;
                put(w, x + a.cos() * 34.0, y - 48.0 + a.sin() * 10.0, 0.0, -150.0, 1.6, 6.0, Color::new(255, 240, 160, 255), Shape::Star, 0.0, 0.0);
            }
            w.burst(x, y, 20, Color::new(255, 240, 160, 255), 160.0, 1.4, 6.0, Shape::Star, -120.0);
        }
        4 => {
            // LIGHTNING: a bolt from the sky, ash and sparks
            let mut pts = Vec::new();
            let mut yy = 0.0;
            let mut xx = x;
            while yy < y {
                pts.push(Vector2::new(xx, yy));
                yy += w.rng.range(30.0, 60.0);
                xx = x + w.rng.range(-28.0, 28.0);
            }
            pts.push(Vector2::new(x, y));
            w.strikes.push(Strike { x, t: 0.0, owner: 0, struck: true, vis: 0.4, pts });
            put(w, x, y, 0.0, 0.0, 0.25, SIZE, Color::WHITE, Shape::Ghost, 0.0, 0.0);
            w.burst(x, y, 40, Color::new(50, 50, 50, 255), 400.0, 1.2, 7.0, Shape::Square, 600.0);
            w.burst(x, y, 24, Color::new(255, 245, 150, 255), 600.0, 0.5, 5.0, Shape::Star, 300.0);
            w.fx.flash = 1.0;
            w.shake(30.0);
        }
        5 => {
            // SPIN OFF: spins away diagonally into the distance
            put(w, x, y, 750.0, -650.0, 1.3, SIZE, col, Shape::Ghost, 200.0, 1100.0);
            for _ in 0..14 {
                let f = w.rng.range(0.3, 1.0);
                put(w, x, y, 750.0 * f, -650.0 * f, 0.9, 6.0, Color::new(255, 240, 160, 255), Shape::Star, 200.0, 0.0);
            }
            w.ring(x, y, 130.0, col);
        }
        6 => {
            // CONFETTI POP
            for _ in 0..70 {
                let a = w.rng.range(0.0, 6.2832);
                let sp = w.rng.range(150.0, 600.0);
                let (life, size, spin) = (w.rng.range(1.0, 1.8), w.rng.range(7.0, 13.0), w.rng.range(-400.0, 400.0));
                let c = hsv(w.rng.next() * 360.0);
                put(w, x, y, a.cos() * sp, a.sin() * sp - 200.0, life, size, c, Shape::Square, 500.0, spin);
            }
            w.burst(x, y, 14, Color::WHITE, 350.0, 0.4, 6.0, Shape::Star, 0.0);
            w.ring(x, y, 150.0, Color::new(255, 150, 220, 255));
        }
        7 => {
            // FREEZE & CRACK
            let ice = Color::new(170, 230, 255, 210);
            put(w, x, y, 0.0, 0.0, 0.7, SIZE, ice, Shape::Ghost, 0.0, 0.0);
            w.burst(x, y, 10, Color::WHITE, 30.0, 0.6, 10.0, Shape::Star, 0.0);
            w.burst(x, y, 40, Color::new(200, 240, 255, 255), 500.0, 1.0, 10.0, Shape::Square, 900.0);
            w.ring(x, y, 190.0, ice);
        }
        8 => {
            // BLACK HOLE: everything is pulled into the middle
            for k in 0..48 {
                let a = k as f32 * 0.1309;
                let r = w.rng.range(150.0, 260.0);
                let sp = r / 0.5;
                let c = if k % 2 == 0 { Color::new(150, 60, 255, 255) } else { Color::new(30, 0, 50, 255) };
                let size = w.rng.range(5.0, 10.0);
                put(w, x + a.cos() * r, y + a.sin() * r, -a.cos() * sp, -a.sin() * sp, 0.5, size, c, Shape::Circle, 0.0, 0.0);
            }
            put(w, x, y, 0.0, 0.0, 0.7, 60.0, Color::new(10, 0, 20, 255), Shape::Circle, 0.0, 0.0);
            w.ring(x, y, 280.0, Color::new(150, 60, 255, 255));
        }
        _ => {
            // GLITCH: RGB split, static and scan streaks
            for (dx, c, vx) in [
                (-14.0, Color::new(255, 40, 40, 170), -260.0),
                (0.0, Color::new(40, 255, 60, 150), 0.0),
                (14.0, Color::new(60, 80, 255, 170), 260.0),
            ] {
                put(w, x + dx, y, vx, 0.0, 0.55, SIZE, c, Shape::Ghost, 0.0, 0.0);
            }
            for k in 0..16 {
                let dir = if k % 2 == 0 { 1.0 } else { -1.0 };
                let (px, py, sp) = (x + w.rng.range(-80.0, 80.0), y + w.rng.range(-50.0, 50.0), w.rng.range(500.0, 1100.0));
                let c = if k % 3 == 0 { Color::new(0, 255, 255, 255) } else { Color::new(255, 0, 200, 255) };
                put(w, px, py, dir * sp, 0.0, 0.3, 3.0, c, Shape::Streak, 0.0, 0.0);
            }
            for _ in 0..30 {
                let (vx, vy, life) = (w.rng.range(-300.0, 300.0), w.rng.range(-300.0, 300.0), w.rng.range(0.3, 0.5));
                let c = hsv((w.rng.next() * 3.0).floor() * 120.0);
                put(w, x, y, vx, vy, life, 8.0, c, Shape::Square, 0.0, 0.0);
            }
        }
    }
}

// =====================================================================
// victory cutscenes: 10 + random (the winner picks how they celebrate)
// =====================================================================

const VICTORY_NAMES: [&str; 11] = [
    "KNIFE STAB FINALE",
    "VICTORY DANCE",
    "FIREWORKS",
    "LIGHTNING ASCENSION",
    "PHOENIX RISE",
    "DOMINATION STOMP",
    "SPOTLIGHT BOW",
    "BLINK STORM",
    "GIANT SLASH",
    "METEOR SHOWER",
    "RANDOM",
];

const VICTORY_BLURBS: [&str; 11] = [
    "knife into the ground, then ALL abilities fire",
    "spins, hops and showers confetti",
    "fireworks burst across the sky",
    "rises into the storm as lightning crashes",
    "flames and fiery wings",
    "three earth-shaking stomps",
    "spotlight, golden sparkles and a bow",
    "teleports all over in a storm of slashes",
    "a screen-wide knife slash cuts the world",
    "meteors rain down around the champion",
    "a surprise every time",
];

// how far the camera zooms in, and how long each cutscene lasts
const VIC_ZOOM: [f32; 10] = [1.35, 1.4, 1.0, 1.4, 1.5, 1.5, 1.7, 1.0, 1.15, 1.0];
const VIC_DUR: [f32; 10] = [4.8, 3.4, 3.6, 3.6, 3.4, 3.6, 3.4, 3.0, 3.2, 3.6];

struct Vic {
    idx: usize,
    who: usize,
    t: f32,
    dur: f32,
    cycle: i32,
}

fn at(prev: f32, now: f32, mark: f32) -> bool {
    prev < mark && now >= mark
}

fn victory_start(choice: usize, who: usize, players: &mut [Player; 2], map: &Map, w: &mut World) -> Vic {
    let idx = if choice >= 10 { (w.rng.next() * 10.0) as usize % 10 } else { choice };
    let ground = H as f32 - FLOOR_H;
    let p = &mut players[who];
    p.x = map.spawns[who];
    p.y = ground - SIZE;
    p.vx = 0.0;
    p.vy = 0.0;
    p.rot = 0.0;
    p.on_ground = true;
    p.facing = if who == 0 { 1.0 } else { -1.0 };
    p.invuln = true;
    p.shield_t = 0.0;
    p.frozen_t = 0.0;
    p.stun_t = 0.0;
    p.burn_t = 0.0;
    p.hurt_t = 0.0;
    p.dash_t = 0.0;
    p.slam = false;
    p.slam_arm = false;
    p.melee_t = 0.0;
    p.block_held = false;
    p.over_t = 0.0;
    p.crouch = 0.0;
    w.projs.clear();
    w.strikes.clear();
    w.meteors.clear();
    w.lasers.clear();
    w.pickups.clear();
    w.grav_scale = 1.0;
    w.fx.flash = 0.8;
    let (cx, cy) = p.center();
    w.ring(cx, cy, 260.0, Color::WHITE);
    w.burst(cx, cy, 20, p.kind.color(), 400.0, 0.6, 7.0, Shape::Star, 0.0);
    Vic { idx, who, t: 0.0, dur: VIC_DUR[idx], cycle: 0 }
}

// put the champion back on the ground when a cutscene ends
fn victory_finish(who: usize, players: &mut [Player; 2], map: &Map) {
    let p = &mut players[who];
    p.x = map.spawns[who];
    p.y = H as f32 - FLOOR_H - SIZE;
    p.vx = 0.0;
    p.vy = 0.0;
    p.rot = 0.0;
    p.on_ground = true;
}

// advance one frame of a cutscene; returns true when it is finished
fn victory_update(v: &mut Vic, players: &mut [Player; 2], map: &Map, w: &mut World, dt: f32) -> bool {
    let prev = v.t;
    v.t += dt;
    let t = v.t;
    let ground = H as f32 - FLOOR_H;
    let floor_y = ground - SIZE;
    let who = v.who;
    {
        let (a, b) = players.split_at_mut(1);
        let (win, lose) = if who == 0 { (&mut a[0], &mut b[0]) } else { (&mut b[0], &mut a[0]) };
        let (cx, cy) = win.center();
        let gold = Color::new(255, 220, 90, 255);
        let orange = Color::new(255, 150, 40, 255);
        match v.idx {
            0 => {
                // KNIFE STAB FINALE
                step(win, &map.solids, dt, 1.0);
                let kx = cx + 130.0;
                if at(prev, t, 0.9) {
                    w.ring(kx, ground, 320.0, Color::WHITE);
                    w.ring(kx, ground, 210.0, orange);
                    w.burst(kx, ground, 32, Color::new(110, 90, 70, 255), 650.0, 0.9, 12.0, Shape::Square, 900.0);
                    w.burst(kx, ground, 20, Color::WHITE, 600.0, 0.6, 7.0, Shape::Star, 0.0);
                    w.shake(32.0);
                    w.fx.flash = 0.7;
                    w.fx.punch = 0.08;
                }
                if at(prev, t, 1.8) {
                    // every ability at once
                    let (sx, sy) = (win.x, win.y);
                    for (k, ab) in ABILITIES.iter().enumerate() {
                        use_ability(*ab, win, lose, who, &map.solids, w);
                        w.ring(cx, cy, 80.0 + k as f32 * 30.0, hsv(k as f32 * 30.0));
                    }
                    win.x = sx;
                    win.y = sy;
                    win.vx = 0.0;
                    win.vy = 0.0;
                    win.dash_t = 0.0;
                    win.slam = false;
                    win.slam_arm = false;
                    win.shield_t = 0.0;
                    win.regen_t = 0.0;
                    w.banner = Some(("ALL ABILITIES!".to_string(), gold, 1.8));
                    w.shake(36.0);
                    w.fx.flash = 1.0;
                    w.burst(cx, cy, 60, Color::WHITE, 900.0, 1.0, 8.0, Shape::Star, 0.0);
                }
                if t > 1.8 && w.rng.next() < 0.5 {
                    let (px, py) = (cx + w.rng.range(-120.0, 120.0), cy + w.rng.range(-100.0, 60.0));
                    put(w, px, py, 0.0, -80.0, 0.7, 6.0, hsv(t * 200.0), Shape::Star, 0.0, 0.0);
                }
            }
            1 => {
                // VICTORY DANCE
                win.rot += 540.0 * dt;
                if win.on_ground && (t * 2.0) as i32 != (prev * 2.0) as i32 {
                    win.vy = -(2.0 * GRAVITY * JUMP * 0.5).sqrt();
                    win.on_ground = false;
                    w.burst(cx, cy + 30.0, 8, gold, 250.0, 0.5, 6.0, Shape::Star, 0.0);
                }
                for _ in 0..3 {
                    let (px, vx, vy) = (w.rng.range(0.0, W as f32), w.rng.range(-40.0, 40.0), w.rng.range(150.0, 300.0));
                    let (size, spin) = (w.rng.range(8.0, 14.0), w.rng.range(-300.0, 300.0));
                    let c = hsv(w.rng.next() * 360.0);
                    put(w, px, -10.0, vx, vy, 2.4, size, c, Shape::Square, 150.0, spin);
                }
                step(win, &map.solids, dt, 1.0);
            }
            2 => {
                // FIREWORKS
                if at(prev, t, 0.3) {
                    win.vy = -(2.0 * GRAVITY * JUMP * 0.7).sqrt();
                    win.on_ground = false;
                }
                if (t * 4.0) as i32 != (prev * 4.0) as i32 {
                    let (fx, fy) = (w.rng.range(150.0, W as f32 - 150.0), w.rng.range(70.0, 320.0));
                    let c = hsv(w.rng.next() * 360.0);
                    w.ring(fx, fy, 120.0, c);
                    w.burst(fx, fy, 45, c, 420.0, 1.1, 6.0, Shape::Star, 150.0);
                    w.burst(fx, fy, 20, Color::WHITE, 300.0, 0.8, 3.0, Shape::Streak, 100.0);
                    w.shake(6.0);
                }
                step(win, &map.solids, dt, 1.0);
            }
            3 => {
                // LIGHTNING ASCENSION
                win.vy = 0.0;
                win.on_ground = false;
                win.rot += 90.0 * dt;
                if win.y > 120.0 {
                    win.y -= 110.0 * dt;
                }
                if (t * 2.5) as i32 != (prev * 2.5) as i32 {
                    let mut sx = w.rng.range(80.0, W as f32 - 80.0);
                    if (sx - cx).abs() < 110.0 {
                        sx += if sx >= cx { 160.0 } else { -160.0 };
                    }
                    let sx = sx.clamp(40.0, W as f32 - 40.0);
                    w.strikes.push(Strike { x: sx, t: 0.45, owner: who, struck: false, vis: 0.0, pts: Vec::new() });
                }
                let (px, py) = (cx + w.rng.range(-40.0, 40.0), cy + w.rng.range(0.0, 60.0));
                put(w, px, py, 0.0, 60.0, 0.6, 5.0, gold, Shape::Star, 0.0, 0.0);
                if at(prev, t, v.dur - 0.5) {
                    w.fx.flash = 1.0;
                    w.ring(cx, cy, 420.0, Color::WHITE);
                    w.shake(30.0);
                }
            }
            4 => {
                // PHOENIX RISE
                win.y = floor_y - 90.0 * ease(t / 0.9);
                win.vy = 0.0;
                win.on_ground = false;
                for _ in 0..4 {
                    let (px, py) = (cx + w.rng.range(-50.0, 50.0), cy + w.rng.range(-20.0, 40.0));
                    let (vy, size) = (-w.rng.range(80.0, 220.0), w.rng.range(6.0, 12.0));
                    let c = Color::new(255, 110 + (w.rng.next() * 120.0) as u8, 30, 230);
                    put(w, px, py, 0.0, vy, 0.8, size, c, Shape::Circle, -100.0, 0.0);
                }
                if at(prev, t, 0.9) {
                    w.ring(cx, ground, 340.0, orange);
                    for k in 0..11 {
                        let fx = (cx + (k as f32 - 5.0) * 70.0).clamp(20.0, W as f32 - 20.0);
                        w.burst(fx, ground, 6, orange, 260.0, 0.9, 10.0, Shape::Circle, -420.0);
                    }
                    w.shake(20.0);
                    w.fx.flash = 0.5;
                }
            }
            5 => {
                // DOMINATION STOMP: three huge stomps
                let tt = (t - 0.2).max(0.0);
                let cyc = ((tt / 0.8) as i32).min(3);
                if t >= 0.2 && cyc < 3 {
                    let phase = (tt % 0.8) / 0.8;
                    win.y = floor_y - 170.0 * (std::f32::consts::PI * phase).sin().max(0.0).powf(0.7);
                    win.on_ground = false;
                } else {
                    win.y = floor_y;
                    win.on_ground = true;
                }
                win.vy = 0.0;
                if cyc != v.cycle {
                    v.cycle = cyc;
                    if cyc >= 1 {
                        let k = cyc as f32;
                        w.ring(cx, ground, 200.0 + 70.0 * k, orange);
                        w.ring(cx, ground, 130.0, Color::WHITE);
                        w.burst(cx, ground, 14 + cyc as usize * 4, Color::new(120, 95, 70, 255), 500.0, 0.8, 11.0, Shape::Square, 900.0);
                        for j in 0..14 {
                            let dir = if j % 2 == 0 { 1.0 } else { -1.0 };
                            let sp = w.rng.range(250.0, 900.0);
                            put(w, cx, ground - 4.0, dir * sp, 0.0, 0.4, 3.0, Color::WHITE, Shape::Streak, 0.0, 0.0);
                        }
                        w.shake(14.0 + 7.0 * k);
                        w.fx.flash = 0.3;
                        w.fx.punch = 0.07;
                    }
                }
            }
            6 => {
                // SPOTLIGHT BOW
                if at(prev, t, 0.3) {
                    win.vy = -(2.0 * GRAVITY * JUMP * 0.45).sqrt();
                    win.on_ground = false;
                }
                win.rot = if t > 1.0 && t < 1.9 { 28.0 * (std::f32::consts::PI * (t - 1.0) / 0.9).sin() * win.facing } else { 0.0 };
                if w.rng.next() < 0.6 {
                    let (px, vx, vy) = (cx + w.rng.range(-110.0, 110.0), w.rng.range(-20.0, 20.0), w.rng.range(120.0, 220.0));
                    put(w, px, cy - 260.0, vx, vy, 1.6, 7.0, gold, Shape::Star, 0.0, 0.0);
                }
                step(win, &map.solids, dt, 1.0);
            }
            7 => {
                // BLINK STORM
                let n = (t / 0.3) as i32;
                let pn = (prev / 0.3) as i32;
                if n != pn && (1..=8).contains(&n) {
                    let violet = Color::new(190, 120, 255, 255);
                    put(w, cx, cy, 0.0, 0.0, 0.5, SIZE, with_alpha(violet, 160), Shape::Ghost, 0.0, 0.0);
                    w.ring(cx, cy, 90.0, violet);
                    if n == 8 {
                        win.x = map.spawns[who];
                        win.y = floor_y;
                    } else {
                        for _ in 0..10 {
                            let si = (w.rng.next() * map.solids.len() as f32) as usize % map.solids.len();
                            let r = map.solids[si].r;
                            if r.width >= SIZE + 20.0 && r.y - SIZE >= 0.0 {
                                win.x = r.x + w.rng.range(10.0, r.width - SIZE - 10.0);
                                win.y = r.y - SIZE;
                                break;
                            }
                        }
                    }
                    win.vx = 0.0;
                    win.vy = 0.0;
                    win.facing = if n % 2 == 0 { 1.0 } else { -1.0 };
                    let (nx, ny) = win.center();
                    w.ring(nx, ny, 100.0, Color::WHITE);
                    w.ring(nx, ny, 150.0, violet);
                    w.burst(nx, ny, 12, Color::WHITE, 380.0, 0.4, 5.0, Shape::Star, 0.0);
                    w.burst(nx, ny, 8, Color::new(220, 240, 255, 255), 700.0, 0.25, 3.0, Shape::Streak, 0.0);
                    w.shake(8.0);
                }
                step(win, &map.solids, dt, 1.0);
            }
            8 => {
                // GIANT SLASH
                step(win, &map.solids, dt, 1.0);
                if at(prev, t, 0.6) {
                    w.shake(10.0);
                }
                if at(prev, t, 0.95) {
                    w.fx.flash = 1.0;
                    w.shake(38.0);
                    w.fx.punch = 0.08;
                    for k in 0..40 {
                        let f = k as f32 / 39.0;
                        let (px, py) = (f * W as f32, 40.0 + f * (H as f32 - 80.0));
                        let (vx, vy) = (w.rng.range(-80.0, 80.0), w.rng.range(-200.0, -40.0));
                        put(w, px, py, vx, vy, 0.8, 6.0, Color::new(180, 240, 255, 255), Shape::Star, 300.0, 0.0);
                    }
                }
            }
            _ => {
                // METEOR SHOWER
                step(win, &map.solids, dt, 1.0);
                if t > 0.3 && t < 2.6 && (t / 0.17) as i32 != (prev / 0.17) as i32 {
                    let tx = w.rng.range(40.0, W as f32 - 40.0);
                    w.meteors.push(Meteor { tx, warn: 0.03, x: 0.0, y: 0.0, vx: 0.0, vy: 0.0, falling: false });
                }
                if at(prev, t, v.dur - 0.4) {
                    w.ring(cx, cy, 420.0, orange);
                    w.ring(cx, cy, 280.0, Color::WHITE);
                    w.fx.flash = 0.9;
                    w.shake(30.0);
                }
            }
        }
    }
    update_particles(&mut w.parts, dt);
    update_booms(&mut w.booms, dt);
    update_strikes(players, w, dt);
    update_projs(players, &map.solids, w, dt);
    update_meteors(players, &map.solids, w, dt);
    v.t >= v.dur
}

// extra world-space visuals for each cutscene (drawn over the players)
fn victory_draw(d: &mut impl RaylibDraw, v: &Vic, win: &Player, now: f32) {
    let ground = H as f32 - FLOOR_H;
    let (cx, cy) = win.center();
    let t = v.t;
    let vv = |x: f32, y: f32| Vector2::new(x, y);
    match v.idx {
        0 => {
            // giant knife falls and sticks in the ground next to the champion
            let scale = 2.4;
            let len = 92.0 * scale;
            let kx = cx + 130.0;
            let y0 = -len - 40.0;
            let y1 = ground + 22.0 - len;
            let hy = if t < 0.9 { y0 + (y1 - y0) * ease(t / 0.9) } else { y1 };
            if t < 0.9 {
                for k in 0..4 {
                    let lx = kx - 20.0 + k as f32 * 13.0;
                    d.draw_line_ex(vv(lx, hy - 140.0), vv(lx, hy), 3.0, Color::new(255, 255, 255, 120));
                }
            }
            draw_knife(d, kx, hy, 180.0, 1.0, scale);
            if t >= 0.9 {
                let a = (1.0 - (t - 0.9) / 1.2).clamp(0.0, 1.0);
                d.draw_circle_lines(kx as i32, ground as i32, 30.0 + 160.0 * (1.0 - a), Color::new(255, 255, 255, (200.0 * a) as u8));
            }
        }
        3 => {
            // light beam and halo
            d.draw_rectangle((cx - 45.0) as i32, 0, 90, cy as i32, Color::new(255, 255, 220, 40));
            for (k, r) in [50.0, 70.0, 90.0].iter().enumerate() {
                let pulse = (now * 6.0 + k as f32).sin() * 5.0;
                d.draw_circle_lines(cx as i32, cy as i32, r + pulse, Color::new(255, 230, 120, 200));
            }
        }
        4 => {
            // wings of flame
            let flap = (now * 8.0).sin();
            for side in [-1.0_f32, 1.0] {
                for (len, ang, col) in [
                    (170.0, -40.0 + 20.0 * flap, Color::new(255, 110, 30, 200)),
                    (135.0, -10.0 + 15.0 * flap, Color::new(255, 170, 40, 200)),
                    (100.0, 22.0 + 10.0 * flap, Color::new(255, 225, 100, 200)),
                ] {
                    let a = (ang as f32).to_radians();
                    let base = vv(cx + side * 16.0, cy);
                    let tip = vv(cx + side * (20.0 + len * a.cos()), cy + len * a.sin());
                    let low = vv(cx + side * 16.0, cy + 34.0);
                    tri(d, base, tip, low, col);
                }
            }
        }
        6 => {
            // dark room with a spotlight on the champion
            let a = (t / 0.4).min(1.0);
            d.draw_ring(vv(cx, cy), 120.0, 1800.0, 0.0, 360.0, 64, Color::new(0, 0, 0, (205.0 * a) as u8));
            let beam = Color::new(255, 240, 160, (34.0 * a) as u8);
            tri(d, vv(cx - 30.0, -20.0), vv(cx - 150.0, ground), vv(cx, ground), beam);
            tri(d, vv(cx + 30.0, -20.0), vv(cx, ground), vv(cx + 150.0, ground), beam);
            tri(d, vv(cx - 30.0, -20.0), vv(cx + 30.0, -20.0), vv(cx, ground), beam);
        }
        8 => {
            // the giant knife sweeps, then a bright cut line hangs across the screen
            if t >= 0.3 && t < 1.1 {
                let f = ease((t - 0.3) / 0.65);
                draw_knife(d, cx - 60.0, cy - 40.0, -80.0 + 180.0 * f, 1.0, 7.0);
            }
            if t >= 0.95 {
                let a = (1.0 - (t - 0.95) / 1.4).clamp(0.0, 1.0);
                let th = 50.0 * a + 4.0;
                d.draw_line_ex(vv(0.0, 40.0), vv(W as f32, H as f32 - 40.0), th * 1.8, Color::new(120, 230, 255, (120.0 * a) as u8));
                d.draw_line_ex(vv(0.0, 40.0), vv(W as f32, H as f32 - 40.0), th, Color::new(255, 255, 255, (255.0 * a) as u8));
            }
        }
        _ => {}
    }
}

fn camera_for(zoom: f32, focus: (f32, f32), weight: f32, shake: (f32, f32)) -> Camera2D {
    let zoom = zoom.max(1.0);
    let tx = W as f32 / 2.0 + (focus.0 - W as f32 / 2.0) * weight;
    let ty = H as f32 / 2.0 + (focus.1 - H as f32 / 2.0) * weight;
    let half_w = W as f32 / (2.0 * zoom);
    let half_h = H as f32 / (2.0 * zoom);
    Camera2D {
        offset: Vector2::new(W as f32 / 2.0 + shake.0, H as f32 / 2.0 + shake.1),
        target: Vector2::new(tx.clamp(half_w, W as f32 - half_w), ty.clamp(half_h, H as f32 - half_h)),
        rotation: 0.0,
        zoom,
    }
}

// =====================================================================
// main
// =====================================================================

const ROWS: usize = 7;
const ROW_NAMES: [&str; ROWS] = ["CHARACTER", "ABILITY 1", "ABILITY 2", "TRAIL", "TRAIL COLOR", "DEATH ANIM", "VICTORY"];

fn respawn(p: &mut Player, setup: &Setup, x: f32, facing: f32, keys: Keys, w: &mut World) {
    let ult = p.ult;
    *p = Player::new(setup, x, facing, keys);
    p.ult = ult * 0.5;
    p.shield_t = 1.5; // spawn protection
    w.ring(x + SIZE / 2.0, p.y + SIZE / 2.0, 120.0, Color::WHITE);
    w.burst(x + SIZE / 2.0, p.y + SIZE / 2.0, 16, p.kind.color(), 300.0, 0.5, 6.0, Shape::Star, 0.0);
}

fn main() {
    let (mut rl, thread) = raylib::init().size(W, H).title("Pausi").build();
    rl.set_target_fps(60);

    let keys = [
        Keys {
            left: KeyboardKey::KEY_A,
            right: KeyboardKey::KEY_D,
            up: KeyboardKey::KEY_W,
            down: KeyboardKey::KEY_S,
            a1: KeyboardKey::KEY_F,
            a2: KeyboardKey::KEY_G,
            melee: KeyboardKey::KEY_E,
            block: KeyboardKey::KEY_Q,
            ult: KeyboardKey::KEY_V,
        },
        Keys {
            left: KeyboardKey::KEY_LEFT,
            right: KeyboardKey::KEY_RIGHT,
            up: KeyboardKey::KEY_UP,
            down: KeyboardKey::KEY_DOWN,
            a1: KeyboardKey::KEY_COMMA,
            a2: KeyboardKey::KEY_PERIOD,
            melee: KeyboardKey::KEY_SLASH,
            block: KeyboardKey::KEY_RIGHT_SHIFT,
            ult: KeyboardKey::KEY_RIGHT_CONTROL,
        },
    ];
    let ready_keys = [KeyboardKey::KEY_F, KeyboardKey::KEY_L]; // select-screen ready

    let mut w = World::new();
    let mut theme_idx = 0usize;
    let mut map = make_map(THEMES[theme_idx]);

    let mut selecting = true;
    let mut setups = [Setup::new(0), Setup::new(1)];
    let mut row = [0usize; 2];
    let mut ready = [false, false];
    let mut previews = [
        Player::new(&setups[0], 0.0, 1.0, keys[0]),
        Player::new(&setups[1], 0.0, 1.0, keys[1]),
    ];
    let mut players = [
        Player::new(&setups[0], map.spawns[0], 1.0, keys[0]),
        Player::new(&setups[1], map.spawns[1], -1.0, keys[1]),
    ];
    let mut result: Option<String> = None;

    // ---- match (series) state ----
    let round_options = [1u32, 3, 5, 10, 15];
    let mut rounds_idx = 1usize; // default: 3 rounds
    let mut mode_idx = 0usize;
    let mut cpu_mode = 0usize; // 0 = two humans, 1 = P2 is a CPU, 2 = CPU vs CPU (spectate)
    let mut idle_t = 0.0f32; // lets CPU vs CPU matches roll on by themselves
    let mut cpu_prof = [0usize, 7usize]; // chosen CPU personality per player (8 = random each round)
    let mut cpu_now = [0usize, 7usize]; // the personality actually in use this round
    let mut start_theme = 0usize;
    let mut round_no = 1u32;
    let mut wins = [0u32; 2];
    let mut series_over = false;
    let mut rd = Round::new();

    // ---- cinematic state ----
    let mut ko_timer = 0.0f32;
    let mut ko_focus = (W as f32 / 2.0, H as f32 / 2.0);
    let mut vic: Option<Vic> = None; // the winner's victory cutscene, while it plays
    let mut pending_vic: Option<usize> = None; // winner waiting for the KO to finish
    let mut vic_done = false;

    while !rl.window_should_close() {
        let dt = rl.get_frame_time().min(0.05);
        let t = rl.get_time() as f32;
        let mode = MODES[mode_idx];
        let cpu = [cpu_mode == 2, cpu_mode >= 1]; // which players are bots

        // screen effects calm down in real time
        w.fx.shake *= 0.86f32.powf(dt * 60.0);
        if w.fx.shake < 0.3 {
            w.fx.shake = 0.0;
        }
        w.fx.punch *= 0.88f32.powf(dt * 60.0);
        w.fx.flash = (w.fx.flash - dt * 2.2).max(0.0);
        if let Some((_, _, left)) = w.banner.as_mut() {
            *left -= dt;
        }
        if matches!(w.banner, Some((_, _, left)) if left <= 0.0) {
            w.banner = None;
        }
        for i in 0..2 {
            rd.combo_show[i] = (rd.combo_show[i] - dt).max(0.0);
        }

        if selecting {
            // ---- loadout menus ----
            let mut preview_death: Option<(usize, usize)> = None;
            for i in 0..2 {
                let k = keys[i];
                if !ready[i] {
                    if rl.is_key_pressed(k.up) {
                        row[i] = cycle(row[i], ROWS, -1);
                    }
                    if rl.is_key_pressed(k.down) {
                        row[i] = cycle(row[i], ROWS, 1);
                    }
                    let dir = rl.is_key_pressed(k.right) as i32 - rl.is_key_pressed(k.left) as i32;
                    if dir != 0 {
                        let s = &mut setups[i];
                        match row[i] {
                            0 => {
                                s.kind = cycle(s.kind, KINDS.len(), dir);
                                s.abil = KINDS[s.kind].default_loadout();
                            }
                            1 => s.abil[0] = cycle(s.abil[0], ABILITIES.len(), dir),
                            2 => s.abil[1] = cycle(s.abil[1], ABILITIES.len(), dir),
                            3 => s.trail = cycle(s.trail, TRAIL_STYLES.len(), dir),
                            4 => s.color = cycle(s.color, TRAIL_COLOR_NAMES.len(), dir),
                            5 => {
                                s.death = cycle(s.death, DEATH_NAMES.len(), dir);
                                preview_death = Some((i, s.death)); // demo it on the preview square
                            }
                            _ => s.victory = cycle(s.victory, VICTORY_NAMES.len(), dir),
                        }
                    }
                }
                if rl.is_key_pressed(ready_keys[i]) {
                    ready[i] = !ready[i];
                }
            }
            if let Some((i, choice)) = preview_death {
                let (px, py) = previews[i].center();
                let col = previews[i].kind.color();
                play_death(choice, px, py, col, &mut w);
            }
            // two humans: both ready. vs CPU: just you. CPU vs CPU: either player's ready key starts it
            let both_ready = match cpu_mode {
                0 => ready[0] && ready[1],
                1 => ready[0],
                _ => ready[0] || ready[1],
            };
            if !both_ready {
                if rl.is_key_pressed(KeyboardKey::KEY_M) {
                    theme_idx = cycle(theme_idx, THEMES.len(), 1);
                    map = make_map(THEMES[theme_idx]);
                }
                if rl.is_key_pressed(KeyboardKey::KEY_N) {
                    rounds_idx = cycle(rounds_idx, round_options.len(), 1);
                }
                if rl.is_key_pressed(KeyboardKey::KEY_B) {
                    mode_idx = cycle(mode_idx, MODES.len(), 1);
                }
                if rl.is_key_pressed(KeyboardKey::KEY_ONE) {
                    cpu_prof[0] = cycle(cpu_prof[0], CPU_NAMES.len(), 1);
                }
                if rl.is_key_pressed(KeyboardKey::KEY_TWO) {
                    cpu_prof[1] = cycle(cpu_prof[1], CPU_NAMES.len(), 1);
                }
                if rl.is_key_pressed(KeyboardKey::KEY_C) {
                    cpu_mode = (cpu_mode + 1) % 3;
                }
            }

            // animated previews so you can see the trail you picked
            for i in 0..2 {
                let panel_x = 40.0 + i as f32 * 640.0;
                let pv = &mut previews[i];
                pv.apply_setup(&setups[i]);
                pv.x = panel_x + 500.0 + (t * 1.6).sin() * 55.0;
                pv.y = 220.0;
                pv.vx = (t * 1.6).cos() * 55.0 * 1.6;
                pv.vy = 0.0;
                pv.facing = if pv.vx >= 0.0 { 1.0 } else { -1.0 };
                pv.rot += pv.vx * dt / (SIZE / 2.0) * 57.3;
                emit_trail(pv, &mut w, t, dt);
            }
            update_particles(&mut w.parts, dt);

            if both_ready {
                players = [
                    Player::new(&setups[0], map.spawns[0], 1.0, keys[0]),
                    Player::new(&setups[1], map.spawns[1], -1.0, keys[1]),
                ];
                w.clear();
                result = None;
                start_theme = theme_idx;
                round_no = 1;
                wins = [0, 0];
                series_over = false;
                ko_timer = 0.0;
                vic = None;
                pending_vic = None;
                rd = Round::new();
                for i in 0..2 {
                    cpu_now[i] = if cpu_prof[i] >= 8 { (w.rng.next() * 8.0) as usize % 8 } else { cpu_prof[i] };
                }
                idle_t = 0.0;
                selecting = false;
            }
        } else {
            // ---- time control: KO slow-motion and hit-stop freeze frames ----
            let ko_active = ko_timer > 0.0;
            let mut dt_sim = dt;
            if ko_active {
                ko_timer = (ko_timer - dt).max(0.0);
                dt_sim = dt * KO_SLOW;
            }
            if w.fx.hitstop > 0.0 {
                w.fx.hitstop -= dt;
                dt_sim = 0.0;
            }

            if vic_done {
                vic = None;
                vic_done = false;
            }

            // the KO is over: the loser goes out in style, then the winner's cutscene starts
            if let Some(wi) = pending_vic {
                if ko_timer <= 0.0 && w.fx.hitstop <= 0.0 && vic.is_none() {
                    let li = 1 - wi;
                    if players[li].dead_t < 50.0 {
                        let (px, py) = players[li].center();
                        let col = players[li].kind.color();
                        play_death(setups[li].death, px, py, col, &mut w);
                        players[li].dead_t = 99.0;
                    }
                    vic = Some(victory_start(setups[wi].victory, wi, &mut players, &map, &mut w));
                    pending_vic = None;
                }
            }

            if let Some(v) = vic.as_mut() {
                // ---- victory cutscene (press SPACE / ENTER to skip) ----
                let skip = rl.is_key_pressed(KeyboardKey::KEY_SPACE) || rl.is_key_pressed(KeyboardKey::KEY_ENTER);
                let finished = victory_update(v, &mut players, &map, &mut w, dt);
                if finished || skip {
                    victory_finish(v.who, &mut players, &map);
                    vic_done = true;
                }
            } else if result.is_none() || ko_active {
                if dt_sim > 0.0 {
                    let hp_before = [players[0].hp, players[1].hp];
                    let live = result.is_none();
                    if live {
                        rd.time += dt_sim;
                    }

                    // ---- respawns (stock and hill modes) ----
                    for i in 0..2 {
                        if live && players[i].dead_t > 0.0 {
                            players[i].dead_t -= dt_sim;
                            if players[i].dead_t <= 0.0 {
                                let facing = if i == 0 { 1.0 } else { -1.0 };
                                respawn(&mut players[i], &setups[i], map.spawns[i], facing, keys[i], &mut w);
                            }
                        }
                    }

                    // ---- inputs: keyboard or CPU ----
                    let inputs = [
                        if cpu[0] {
                            bot_input(&players[0], &players[1], &mut w.rng, &cpu_profile(cpu_now[0]), t, 0.0)
                        } else {
                            read_input(&rl, &keys[0])
                        },
                        if cpu[1] {
                            bot_input(&players[1], &players[0], &mut w.rng, &cpu_profile(cpu_now[1]), t, 3.7)
                        } else {
                            read_input(&rl, &keys[1])
                        },
                    ];
                    for i in 0..2 {
                        if players[i].dead_t > 0.0 {
                            continue;
                        }
                        let (a, b) = players.split_at_mut(1);
                        let (me, foe) = if i == 0 { (&mut a[0], &mut b[0]) } else { (&mut b[0], &mut a[0]) };
                        update_player(&inputs[i], me, foe, i, &map.solids, &mut w, dt_sim);
                    }
                    collide_players(&mut players, &map.solids);
                    for p in players.iter_mut() {
                        clamp_to_arena(p);
                    }
                    for i in 0..2 {
                        let (a, b) = players.split_at_mut(1);
                        let (me, foe) = if i == 0 { (&mut a[0], &mut b[0]) } else { (&mut b[0], &mut a[0]) };
                        slam_land(me, foe, &mut w);
                    }

                    // ---- map events ----
                    if live {
                        update_events(THEMES[theme_idx], &mut rd, &mut players, &map.solids, &mut w, dt_sim);
                        update_pickups(&mut players, &map.solids, &mut rd, &mut w, dt_sim);
                    }

                    // lava hurts (and surges up in the volcano)
                    let rise = if THEMES[theme_idx] == Theme::Volcano {
                        90.0 * ease((rd.time * 0.5).sin().max(0.0))
                    } else {
                        0.0
                    };
                    let hazard_now = map.hazard.map(|hz| Rectangle::new(hz.x, hz.y - rise, hz.width, hz.height + rise));
                    if let Some(hz) = hazard_now {
                        for p in players.iter_mut() {
                            if p.dead_t <= 0.0 && overlaps(&p.rect(), &hz) {
                                p.hp = (p.hp - LAVA_DPS * dt_sim).max(0.0);
                                p.hurt_t = 0.1;
                                if w.rng.next() < 0.5 {
                                    let (cx, cy) = p.center();
                                    let mut q = Particle::new(cx + w.rng.range(-20.0, 20.0), cy + 25.0, w.rng.range(-40.0, 40.0), -120.0, 0.5, 6.0, Color::new(255, 150, 30, 255), Shape::Circle);
                                    q.grav = 400.0;
                                    w.parts.push(q);
                                }
                            }
                        }
                    }

                    update_projs(&mut players, &map.solids, &mut w, dt_sim);
                    update_strikes(&mut players, &mut w, dt_sim);
                    update_booms(&mut w.booms, dt_sim);
                    for p in players.iter_mut() {
                        emit_trail(p, &mut w, t, dt_sim);
                    }
                    update_particles(&mut w.parts, dt_sim);

                    // ---- parries and blocks ----
                    for i in 0..2 {
                        let (px, py) = players[i].center();
                        if players[i].parried {
                            players[i].parried = false;
                            let att = 1 - i;
                            players[att].stun_t = 0.8;
                            players[att].vx = 0.0;
                            players[i].ult = (players[i].ult + 20.0).min(ULT_MAX);
                            w.ring(px, py, 160.0, Color::WHITE);
                            w.ring(px, py, 100.0, Color::new(255, 230, 90, 255));
                            w.burst(px, py, 20, Color::WHITE, 500.0, 0.5, 7.0, Shape::Star, 0.0);
                            w.fx.hitstop = w.fx.hitstop.max(0.18);
                            w.fx.flash = w.fx.flash.max(0.35);
                            w.shake(12.0);
                            w.banner = Some(("PARRY!".to_string(), Color::WHITE, 0.7));
                        }
                        if players[i].blocked {
                            players[i].blocked = false;
                            let f = players[i].facing;
                            w.burst(px + f * 40.0, py, 10, Color::new(200, 230, 255, 255), 350.0, 0.3, 4.0, Shape::Streak, 0.0);
                            w.ring(px + f * 40.0, py, 50.0, Color::new(200, 230, 255, 255));
                            w.shake(4.0);
                        }
                    }

                    // ---- impact feel and the combo / ultimate economy ----
                    for i in 0..2 {
                        let drop = hp_before[i] - players[i].hp;
                        if drop >= 3.0 && players[i].hurt_t > 0.2 {
                            let att = 1 - i;
                            if rd.combo_t[att] > 0.0 {
                                rd.combo[att] += 1;
                            } else {
                                rd.combo[att] = 1;
                            }
                            rd.combo_t[att] = COMBO_WINDOW;
                            rd.combo_show[att] = 1.6;
                            let mut bonus = ((rd.combo[att] as f32 - 1.0) * 0.1).min(0.5);
                            if players[att].power_t > 0.0 {
                                bonus += 0.5;
                            }
                            let extra = drop * bonus;
                            if extra > 0.0 {
                                players[i].hp = (players[i].hp - extra).max(0.0);
                            }
                            let total = drop + extra;
                            players[att].ult = (players[att].ult + total * 1.2).min(ULT_MAX);
                            players[i].ult = (players[i].ult + total * 0.6).min(ULT_MAX);
                            if total >= 18.0 && !players[i].block_held {
                                players[i].stun_t = players[i].stun_t.max(0.3); // heavy hit stagger
                            }
                            let (px, py) = players[i].center();
                            w.shake((5.0 + total * 0.6).min(24.0));
                            w.fx.hitstop = w.fx.hitstop.max((0.03 + total * 0.003).min(0.12));
                            w.fx.punch = w.fx.punch.max((0.02 + total * 0.002).min(0.08));
                            if total >= 18.0 {
                                w.fx.flash = w.fx.flash.max(0.25);
                            }
                            w.burst(px, py, 10 + total as usize, Color::WHITE, 420.0, 0.35, 4.0, Shape::Streak, 0.0);
                            w.burst(px, py, 6, players[i].kind.color(), 300.0, 0.5, 9.0, Shape::Square, 500.0);
                            w.ring(px, py, 60.0 + total * 2.0, Color::WHITE);
                        }
                    }
                    for i in 0..2 {
                        rd.combo_t[i] = (rd.combo_t[i] - dt_sim).max(0.0);
                    }

                    // ---- stock / hill deaths: respawn instead of ending the round ----
                    if live && (mode == Mode::Stock || mode == Mode::Hill) {
                        for i in 0..2 {
                            if players[i].hp <= 0.0 && players[i].dead_t <= 0.0 {
                                players[i].dead_t = RESPAWN_TIME;
                                let (px, py) = players[i].center();
                                let col = players[i].kind.color();
                                play_death(setups[i].death, px, py, col, &mut w);
                                if mode == Mode::Stock {
                                    rd.stocks[i] -= 1;
                                } else {
                                    rd.hill[1 - i] += 5.0;
                                }
                            }
                        }
                    }

                    // ---- king of the hill: scoring and the moving zone ----
                    if live && mode == Mode::Hill {
                        rd.zone_t -= dt_sim;
                        if rd.zone_t <= 0.0 {
                            rd.zone_t = HILL_MOVE;
                            rd.zone_idx = (rd.zone_idx + 1) % map.zones.len();
                            let z = map.zones[rd.zone_idx];
                            w.ring(z.x + z.width / 2.0, z.y + z.height / 2.0, 140.0, Color::new(255, 230, 90, 255));
                        }
                        let zone = map.zones[rd.zone_idx];
                        let inside = [
                            players[0].dead_t <= 0.0 && overlaps(&players[0].rect(), &zone),
                            players[1].dead_t <= 0.0 && overlaps(&players[1].rect(), &zone),
                        ];
                        if inside[0] && !inside[1] {
                            rd.hill[0] += dt_sim;
                        } else if inside[1] && !inside[0] {
                            rd.hill[1] += dt_sim;
                        }
                    }

                    // ---- end of round: KO cinematic and tally ----
                    if result.is_none() {
                        let dead = [players[0].hp <= 0.0, players[1].hp <= 0.0];
                        let by_hp = |a: f32, b: f32| -> Option<usize> {
                            if (a - b).abs() < 0.01 {
                                None
                            } else if a > b {
                                Some(0)
                            } else {
                                Some(1)
                            }
                        };
                        // (winner, was it a knockout?)
                        let mut end: Option<(Option<usize>, bool)> = None;
                        match mode {
                            Mode::Classic | Mode::Timed => {
                                end = match dead {
                                    [true, true] => Some((None, true)),
                                    [true, false] => Some((Some(1), true)),
                                    [false, true] => Some((Some(0), true)),
                                    _ => None,
                                };
                                if end.is_none() && mode == Mode::Timed && rd.time >= TIME_LIMIT {
                                    end = Some((by_hp(players[0].hp, players[1].hp), false));
                                }
                            }
                            Mode::Stock => {
                                if rd.stocks[0] <= 0 || rd.stocks[1] <= 0 {
                                    let win = if rd.stocks[0] <= 0 && rd.stocks[1] <= 0 {
                                        None
                                    } else if rd.stocks[0] <= 0 {
                                        Some(1)
                                    } else {
                                        Some(0)
                                    };
                                    end = Some((win, true));
                                }
                            }
                            Mode::Hill => {
                                if rd.hill[0] >= HILL_TARGET || rd.hill[1] >= HILL_TARGET || rd.time >= HILL_TIME {
                                    end = Some((by_hp(rd.hill[0], rd.hill[1]), false));
                                }
                            }
                        }
                        if let Some((win, ko)) = end {
                            if let Some(i) = win {
                                wins[i] += 1;
                            }
                            series_over = round_no >= round_options[rounds_idx];
                            let reason = if ko { "" } else if mode == Mode::Hill { " (HILL POINTS)" } else { " (TIME UP)" };
                            result = Some(match win {
                                Some(i) => format!("P{} ({}) WINS THE ROUND{}", i + 1, players[i].kind.name(), reason),
                                None => format!("ROUND DRAWN{}", reason),
                            });
                            w.fx.flash = 0.9;
                            w.shake(26.0);
                            w.fx.hitstop = 0.18;
                            pending_vic = win; // the winner gets a victory cutscene (not on a draw)
                            if ko {
                                ko_timer = KO_TIME;
                                // the loser(s) go out with their chosen death animation
                                let mut fx = 0.0;
                                let mut fy = 0.0;
                                let mut n = 0.0;
                                for i in 0..2 {
                                    if dead[i] || (mode == Mode::Stock && rd.stocks[i] <= 0) {
                                        players[i].hp = 0.0;
                                        let (px, py) = players[i].center();
                                        let col = players[i].kind.color();
                                        if players[i].dead_t <= 0.0 {
                                            // (stock mode already played it when the last life went)
                                            play_death(setups[i].death, px, py, col, &mut w);
                                        }
                                        players[i].dead_t = 99.0;
                                        fx += px;
                                        fy += py;
                                        n += 1.0;
                                    }
                                }
                                if n > 0.0 {
                                    ko_focus = (fx / n, fy / n);
                                }
                            }
                        }
                    }
                }
            } else {
                // between rounds: let leftover effects fade, then wait for a key
                update_booms(&mut w.booms, dt);
                update_particles(&mut w.parts, dt);
                update_strikes(&mut players, &mut w, dt);
                // in CPU vs CPU the match plays itself: next round after a few seconds,
                // and back to the menu a bit after the final round
                if pending_vic.is_none() && cpu[0] && cpu[1] {
                    idle_t += dt;
                } else {
                    idle_t = 0.0;
                }
                let auto_next = idle_t > 3.5;
                let auto_menu = idle_t > 6.0;
                if pending_vic.is_none() && (rl.is_key_pressed(KeyboardKey::KEY_R) || (series_over && auto_menu)) {
                    idle_t = 0.0;
                    selecting = true;
                    ready = [false, false];
                    w.clear();
                    vic = None;
                    theme_idx = start_theme; // back to the map you picked
                    map = make_map(THEMES[theme_idx]);
                } else if pending_vic.is_none()
                    && !series_over
                    && (rl.is_key_pressed(KeyboardKey::KEY_SPACE) || rl.is_key_pressed(KeyboardKey::KEY_ENTER) || auto_next)
                {
                    idle_t = 0.0;
                    // next round: next theme in the rotation, fresh squares
                    round_no += 1;
                    theme_idx = cycle(theme_idx, THEMES.len(), 1);
                    map = make_map(THEMES[theme_idx]);
                    players = [
                        Player::new(&setups[0], map.spawns[0], 1.0, keys[0]),
                        Player::new(&setups[1], map.spawns[1], -1.0, keys[1]),
                    ];
                    w.clear();
                    result = None;
                    rd = Round::new();
                    for i in 0..2 {
                        // "RANDOM" personalities are re-rolled every round
                        cpu_now[i] = if cpu_prof[i] >= 8 { (w.rng.next() * 8.0) as usize % 8 } else { cpu_prof[i] };
                    }
                }
            }
        }

        // ---- draw ----
        let theme = THEMES[theme_idx];
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::BLACK);

        if selecting {
            draw_scenery(&mut d, theme, t);
            draw_world(&mut d, theme, &map, map.hazard, None, &World::new(), t);
            center_text(&mut d, "PAUSI", 8, 56, Color::WHITE);
            center_text(
                &mut d,
                &format!("STARTING MAP: {}  ({})   - press M to change", theme.name(), theme.blurb()),
                70,
                22,
                Color::YELLOW,
            );

            for i in 0..2 {
                let px = 40 + i as i32 * 640;
                let py = 100;
                d.draw_rectangle(px, py, 600, 460, Color::new(0, 0, 0, 170));
                d.draw_rectangle_lines(px, py, 600, 460, Color::WHITE);
                let label = if cpu[i] {
                    format!("PLAYER {} (CPU: {})", i + 1, CPU_NAMES[cpu_prof[i]])
                } else {
                    format!("PLAYER {}", i + 1)
                };
                text(&mut d, &label, px + 16, py + 8, 28, previews[i].kind.color());
                if ready[i] || cpu[i] {
                    text(&mut d, "READY!", px + 440, py + 8, 30, Color::LIME);
                }

                let s = &setups[i];
                let values = [
                    KINDS[s.kind].name(),
                    ABILITIES[s.abil[0]].name(),
                    ABILITIES[s.abil[1]].name(),
                    TRAIL_STYLES[s.trail].name(),
                    TRAIL_COLOR_NAMES[s.color],
                    DEATH_NAMES[s.death],
                    VICTORY_NAMES[s.victory],
                ];
                for r in 0..ROWS {
                    let ry = py + 44 + r as i32 * 36;
                    let active = row[i] == r && !ready[i];
                    if active {
                        d.draw_rectangle(px + 8, ry - 3, 584, 32, Color::new(255, 255, 255, 60));
                    }
                    text(&mut d, ROW_NAMES[r], px + 20, ry, 22, Color::WHITE);
                    let val = if active { format!("<  {}  >", values[r]) } else { values[r].to_string() };
                    text(&mut d, &val, px + 190, ry, 24, if active { Color::YELLOW } else { Color::LIGHTGRAY });
                }

                let kind = KINDS[s.kind];
                let ks = if i == 0 { ("F", "G") } else { ("COMMA", "PERIOD") };
                let a1 = ABILITIES[s.abil[0]];
                let a2 = ABILITIES[s.abil[1]];
                let by = py + 304;
                text(&mut d, &format!("{}: {} - {}", ks.0, a1.name(), a1.blurb()), px + 20, by, 20, Color::WHITE);
                text(&mut d, &format!("{}: {} - {}", ks.1, a2.name(), a2.blurb()), px + 20, by + 24, 20, Color::WHITE);
                let (knife, block, ult) = if i == 0 { ("E", "Q", "V") } else { ("SLASH", "R-SHIFT", "R-CTRL") };
                text(&mut d, &format!("{}: KNIFE   {}: BLOCK (tap = parry)", knife, block), px + 20, by + 48, 20, Color::WHITE);
                text(&mut d, &format!("{}: ULT {} - {}", ult, kind.ult_name(), kind.ult_blurb()), px + 20, by + 72, 20, Color::new(255, 220, 90, 255));
                text(&mut d, kind.passive(), px + 20, by + 96, 20, Color::new(150, 255, 170, 255));
                // describe whichever animation row you're on
                let (tag, blurb) = match row[i] {
                    5 => ("DEATH", DEATH_BLURBS[s.death]),
                    6 => ("VICTORY", VICTORY_BLURBS[s.victory]),
                    _ => ("", ""),
                };
                let note = if blurb.is_empty() && cpu[i] {
                    format!("CPU {} ({}): {}", CPU_NAMES[cpu_prof[i]], if i == 0 { "key 1" } else { "key 2" }, CPU_BLURBS[cpu_prof[i]])
                } else if blurb.is_empty() {
                    format!("DEATH: {}   VICTORY: {}", DEATH_NAMES[s.death], VICTORY_NAMES[s.victory])
                } else {
                    format!("{}: {}", tag, blurb)
                };
                text(&mut d, &note, px + 20, by + 120, 20, Color::new(255, 170, 230, 255));
            }
            draw_particles(&mut d, &w.parts);
            for i in 0..2 {
                draw_player(&mut d, &previews[i], t);
            }
            text(&mut d, "P1: W/S pick row  A/D change  F ready", 40, 572, 24, Color::YELLOW);
            text(&mut d, "P2: Up/Down pick row  Left/Right change  L ready", 640, 572, 24, Color::YELLOW);
            center_text(
                &mut d,
                &format!(
                    "ROUNDS: {} (N)    MODE: {} (B)    PLAYERS: {} (C)",
                    round_options[rounds_idx],
                    mode.name(),
                    match cpu_mode {
                        0 => "HUMAN vs HUMAN",
                        1 => "HUMAN vs CPU",
                        _ => "CPU vs CPU",
                    }
                ),
                604,
                26,
                Color::WHITE,
            );
            center_text(&mut d, mode.blurb(), 636, 22, Color::LIGHTGRAY);
            let go = match cpu_mode {
                0 => "Every round switches to the next map. Both ready = fight!",
                1 => "Press F to fight the CPU!  (key 2 changes its personality)",
                _ => "CPU vs CPU: keys 1 and 2 change their personalities - press F or L to start and watch",
            };
            center_text(&mut d, go, 664, 22, Color::LIME);
        } else {
            // ---- the cinematic camera: shake, zoom punch, KO zoom ----
            let ko_prog = if ko_timer > 0.0 { ease(1.0 - ko_timer / KO_TIME) } else { 0.0 };
            let mut zoom = 1.0 + w.fx.punch + 0.45 * ko_prog;
            let mut focus = ko_focus;
            let mut weight = ko_prog;
            if let Some(v) = vic.as_ref() {
                // victory cutscene: push in on the champion
                let wp = ease(v.t / 0.5);
                zoom = 1.0 + (VIC_ZOOM[v.idx] - 1.0) * wp + w.fx.punch;
                focus = players[v.who].center();
                weight = wp;
            }
            let shake = (w.rng.range(-1.0, 1.0) * w.fx.shake, w.rng.range(-1.0, 1.0) * w.fx.shake);
            let cam = camera_for(zoom, focus, weight, shake);
            let rise = if theme == Theme::Volcano { 90.0 * ease((rd.time * 0.5).sin().max(0.0)) } else { 0.0 };
            let hazard_now = map.hazard.map(|hz| Rectangle::new(hz.x, hz.y - rise, hz.width, hz.height + rise));
            let zone = if mode == Mode::Hill { Some(map.zones[rd.zone_idx]) } else { None };
            {
                let mut m = d.begin_mode2D(cam);
                draw_scenery(&mut m, theme, t);
                draw_world(&mut m, theme, &map, hazard_now, zone, &w, t);
                draw_particles(&mut m, &w.parts);
                for p in players.iter() {
                    if p.hp > 0.0 && p.dead_t <= 0.0 {
                        draw_player(&mut m, p, t);
                    }
                }
                if let Some(v) = vic.as_ref() {
                    victory_draw(&mut m, v, &players[v.who], t);
                }
            }

            // the HUD steps aside while a victory cutscene plays
            if vic.is_none() {
            let tags = [
                if cpu[0] { Some(CPU_NAMES[cpu_now[0]]) } else { None },
                if cpu[1] { Some(CPU_NAMES[cpu_now[1]]) } else { None },
            ];
            draw_hud(&mut d, &players, mode, &rd, t, &tags);
            // round counter and running tally, top center
            center_text(&mut d, &format!("ROUND {} / {}", round_no, round_options[rounds_idx]), 12, 24, Color::WHITE);
            center_text(&mut d, &format!("{}  -  {}", wins[0], wins[1]), 42, 44, Color::YELLOW);
            center_text(&mut d, &format!("{} - {}", theme.name(), mode.name()), 92, 18, Color::LIGHTGRAY);
            match mode {
                Mode::Timed => {
                    let left = (TIME_LIMIT - rd.time).max(0.0);
                    let col = if left < 10.0 { Color::new(255, 90, 80, 255) } else { Color::WHITE };
                    center_text(&mut d, &format!("{:.0}", left.ceil()), 118, 40, col);
                }
                Mode::Hill => {
                    let left = (HILL_TIME - rd.time).max(0.0);
                    center_text(&mut d, &format!("HILL  {:.0}  -  {:.0}   ({:.0}s)", rd.hill[0], rd.hill[1], left.ceil()), 118, 26, Color::new(255, 230, 90, 255));
                    center_text(&mut d, &format!("first to {:.0}", HILL_TARGET), 148, 18, Color::LIGHTGRAY);
                }
                _ => {}
            }
            let wind_name = match theme {
                Theme::Frozen => "BLIZZARD",
                Theme::Sea => "CURRENT",
                _ => "WIND",
            };
            if rd.wind_warn {
                center_text(&mut d, &format!("{} INCOMING!", wind_name), 150, 28, Color::WHITE);
            } else if rd.wind != 0.0 {
                let arrow = if rd.wind > 0.0 { format!("{}  >>>", wind_name) } else { format!("<<<  {}", wind_name) };
                center_text(&mut d, &arrow, 150, 28, Color::WHITE);
            }
            if theme == Theme::Space {
                if rd.zerog_warn {
                    center_text(&mut d, "ZERO-G INCOMING!", 150, 28, Color::WHITE);
                } else if rd.zerog {
                    center_text(&mut d, "ZERO-G!", 150, 28, Color::new(160, 220, 255, 255));
                }
            }
            if theme == Theme::Volcano && rise > 5.0 {
                center_text(&mut d, "LAVA SURGE!", 176, 28, Color::new(255, 130, 40, 255));
            }

            // two lines of controls on each side
            text(&mut d, "P1: A/D move  W jump (again = double, walls too)  S charge", 15, H - 56, 20, Color::WHITE);
            text(&mut d, "F/G abilities  E knife  Q block  V ultimate", 15, H - 30, 20, Color::WHITE);
            let p2a = "P2: arrows move/jump/charge";
            let p2b = "COMMA/PERIOD abilities  SLASH knife  R-SHIFT block  R-CTRL ult";
            text(&mut d, p2a, W - text_width(p2a, 20) - 15, H - 56, 20, Color::WHITE);
            text(&mut d, p2b, W - text_width(p2b, 20) - 15, H - 30, 20, Color::WHITE);

            // respawn countdowns
            for i in 0..2 {
                if players[i].dead_t > 0.0 && result.is_none() {
                    let x = if i == 0 { W / 4 } else { 3 * W / 4 };
                    let s = format!("P{} RESPAWN {:.1}", i + 1, players[i].dead_t);
                    text(&mut d, &s, x - text_width(&s, 28) / 2, H / 2, 28, Color::WHITE);
                }
            }
            }

            // ---- victory cutscene: letterbox bars and a title card ----
            if let Some(v) = vic.as_ref() {
                let bar = 90.0 * ease(v.t / 0.4);
                d.draw_rectangle(0, 0, W, bar as i32, Color::BLACK);
                d.draw_rectangle(0, H - bar as i32, W, bar as i32 + 1, Color::BLACK);
                let who = v.who;
                let title = format!("P{} {}  -  {}", who + 1, players[who].kind.name(), VICTORY_NAMES[v.idx]);
                center_text(&mut d, &title, 28, 30, Color::YELLOW);
                center_text(&mut d, "SPACE = skip", H - 60, 20, Color::LIGHTGRAY);
            }

            // ---- big ultimate / parry banner ----
            if let Some((name, col, left)) = &w.banner {
                let a = (left / 0.4).clamp(0.0, 1.0);
                let size = 70;
                let bw = text_width(name, size);
                d.draw_rectangle(0, 250, W, 100, Color::new(0, 0, 0, (150.0 * a) as u8));
                text(&mut d, name, W / 2 - bw / 2, 265, size, with_alpha(*col, (255.0 * a) as u8));
            }

            // ---- KO cinematic: letterbox bars + big K.O. text ----
            if ko_timer > 0.0 {
                let elapsed = KO_TIME - ko_timer;
                let bar = 90.0 * ease(elapsed / 0.4);
                d.draw_rectangle(0, 0, W, bar as i32, Color::BLACK);
                d.draw_rectangle(0, H - bar as i32, W, bar as i32 + 1, Color::BLACK);
                let size = (170.0 - 60.0 * ease(elapsed / 0.3)) as i32;
                let jx = w.rng.range(-4.0, 4.0) as i32;
                text(&mut d, "K.O.!", W / 2 - text_width("K.O.!", size) / 2 + jx, H / 2 - size / 2, size, Color::YELLOW);
            }

            // ---- round result banner (after the KO and victory cutscene) ----
            if ko_timer <= 0.0 && vic.is_none() && pending_vic.is_none() {
                if let Some(msg) = &result {
                    d.draw_rectangle(0, 190, W, 260, Color::new(0, 0, 0, 150));
                    center_text(&mut d, msg, 205, 50, Color::YELLOW);
                    center_text(&mut d, &format!("TALLY   P1 {}  -  {} P2", wins[0], wins[1]), 285, 40, Color::WHITE);
                    if series_over {
                        let final_text = if wins[0] > wins[1] {
                            format!("P1 WINS THE MATCH {} - {}", wins[0], wins[1])
                        } else if wins[1] > wins[0] {
                            format!("P2 WINS THE MATCH {} - {}", wins[1], wins[0])
                        } else {
                            format!("MATCH DRAWN {} - {}", wins[0], wins[1])
                        };
                        center_text(&mut d, &final_text, 345, 48, Color::LIME);
                        center_text(&mut d, "Press R to go back to the menu", 405, 28, Color::WHITE);
                    } else {
                        let next = THEMES[cycle(theme_idx, THEMES.len(), 1)].name();
                        center_text(&mut d, &format!("Next map: {}", next), 350, 30, Color::LIGHTGRAY);
                        center_text(&mut d, "Press SPACE for the next round  (R = menu)", 395, 28, Color::WHITE);
                    }
                }
            }
        }

        // white flash for big moments
        if w.fx.flash > 0.0 {
            d.draw_rectangle(0, 0, W, H, Color::new(255, 255, 255, (w.fx.flash.min(1.0) * 200.0) as u8));
        }
    }
}
