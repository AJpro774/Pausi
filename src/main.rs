use raylib::prelude::*;

// ---- Alexander's movement numbers (unchanged feel) ----
const W: i32 = 1280;
const H: i32 = 720;
const SIZE: f32 = 60.0; // smaller than the solo prototype so two fit in an arena
const TOP_SPEED: f32 = 350.0; // px per second
const ACCEL: f32 = TOP_SPEED / 0.25; // reach top speed in 250ms
const ICE: f32 = 400.0; // slide after letting go
const JUMP: f32 = 200.0; // normal jump height in pixels
const MAX_CHARGE: f32 = 3.5; // full charge = 3.5x jump
const GRAVITY: f32 = 250.0;
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
const MELEE_DMG: f32 = 10.0;
const MELEE_REACH: f32 = 80.0;
const LAVA_DPS: f32 = 35.0;
const MAX_SPEED: f32 = 1500.0; // knockback never launches anyone faster than this
const BLINK_DIST: f32 = 260.0;
const STRIKE_DELAY: f32 = 0.9;
const FROZEN_TIME: f32 = 1.3;
const BURN_TIME: f32 = 2.5;
const KO_TIME: f32 = 1.8; // slow-motion KO cinematic
const KO_SLOW: f32 = 0.25;

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
}

const KINDS: [Kind; 3] = [Kind::Dasher, Kind::Bomber, Kind::Shielder];

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Kind::Dasher => "DASHER",
            Kind::Bomber => "BOMBER",
            Kind::Shielder => "SHIELDER",
        }
    }
    fn color(self) -> Color {
        match self {
            Kind::Dasher => Color::ORANGE,
            Kind::Bomber => Color::new(220, 60, 60, 255),
            Kind::Shielder => Color::new(40, 170, 160, 255),
        }
    }
    // indices into ABILITIES
    fn default_loadout(self) -> [usize; 2] {
        match self {
            Kind::Dasher => [0, 1],
            Kind::Bomber => [2, 3],
            Kind::Shielder => [4, 5],
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
}

impl Setup {
    fn new(kind: usize) -> Self {
        Setup { kind, abil: KINDS[kind].default_loadout(), trail: 1, color: 0 }
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
}

impl Particle {
    fn new(x: f32, y: f32, vx: f32, vy: f32, life: f32, size: f32, color: Color, shape: Shape) -> Self {
        Particle { x, y, vx, vy, life, max: life, size, rot: 0.0, color, shape, grow: false, grav: 0.0 }
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
            rng: Rng(2463534242),
            fx: Fx { shake: 0.0, hitstop: 0.0, punch: 0.0, flash: 0.0 },
        }
    }

    fn clear(&mut self) {
        self.projs.clear();
        self.booms.clear();
        self.parts.clear();
        self.strikes.clear();
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
// players and world
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
}

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
    kind: Kind,
    abilities: [Ability; 2],
    trail_style: usize,
    trail_color: usize,
    trail_acc: f32,
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
}

impl Player {
    fn new(setup: &Setup, x: f32, facing: f32, keys: Keys) -> Self {
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
            hp: MAX_HP,
            kind: KINDS[setup.kind],
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

    // damage + knockback; shield blocks everything
    fn hurt(&mut self, dmg: f32, kx: f32, ky: f32) {
        if self.shield_t > 0.0 {
            return;
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
}

const THEMES: [Theme; 3] = [Theme::Meadow, Theme::Skyline, Theme::Volcano];

impl Theme {
    fn name(self) -> &'static str {
        match self {
            Theme::Meadow => "SUNNY MEADOW",
            Theme::Skyline => "NIGHT SKYLINE",
            Theme::Volcano => "VOLCANO",
        }
    }
    fn blurb(self) -> &'static str {
        match self {
            Theme::Meadow => "open hills and floating islands",
            Theme::Skyline => "rooftops and towers with a gap between them",
            Theme::Volcano => "lava on the floor hurts - stay off it!",
        }
    }
}

struct Map {
    solids: Vec<Solid>,
    hazard: Option<Rectangle>, // lava: damages while you touch it
    spawns: [f32; 2],
}

fn make_map(theme: Theme) -> Map {
    let floor_y = H as f32 - FLOOR_H;
    let b = |x: f32, y: f32, w: f32, h: f32| Solid { r: Rectangle::new(x, y, w, h) };
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
            spawns: [120.0, W as f32 - 120.0 - SIZE],
        },
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
    if !overlaps(&a.rect(), &b.rect()) {
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

fn step_once(p: &mut Player, solids: &[Solid], dt: f32) {
    p.x += p.vx * dt;
    push_out_x(p, solids);

    if p.dash_t <= 0.0 {
        p.vy += GRAVITY * dt;
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

// move + collide; fast movers are split into small steps so nobody tunnels
fn step(p: &mut Player, solids: &[Solid], dt: f32) {
    let moves = (p.vx.abs().max(p.vy.abs()) * dt / 20.0).ceil().clamp(1.0, 8.0) as i32;
    let sub = dt / moves as f32;
    for _ in 0..moves {
        step_once(p, solids, sub);
    }
    clamp_to_arena(p);
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
            p.hurt(25.0 * f, nx * 600.0 * f, ny * 600.0 * f - 200.0);
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

        let mut end = b.life <= 0.0 || b.x < 0.0 || b.x > W as f32 || b.y > H as f32 || b.y < -50.0;
        let br = Rectangle::new(b.x - 8.0, b.y - 8.0, 16.0, 16.0);
        if solids.iter().any(|s| overlaps(&br, &s.r)) {
            end = true;
        }
        let foe = 1 - b.owner;
        let mut hit = false;
        if overlaps(&br, &players[foe].rect()) {
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
// player update
// =====================================================================

fn update_player(rl: &RaylibHandle, me: &mut Player, foe: &mut Player, my_idx: usize, solids: &[Solid], w: &mut World, dt: f32) {
    me.cd1 = (me.cd1 - dt).max(0.0);
    me.cd2 = (me.cd2 - dt).max(0.0);
    me.shield_t = (me.shield_t - dt).max(0.0);
    me.hurt_t = (me.hurt_t - dt).max(0.0);
    me.frozen_t = (me.frozen_t - dt).max(0.0);
    let can_act = me.frozen_t <= 0.0;
    let (cx, cy) = me.center();

    // ---- burning and healing over time ----
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
        me.hp = (me.hp + 9.0 * dt).min(MAX_HP);
        if w.rng.next() < 0.5 {
            let q = Particle::new(cx + w.rng.range(-25.0, 25.0), cy + 10.0, 0.0, -70.0, 0.7, 6.0, Color::new(110, 255, 140, 255), Shape::Star);
            w.parts.push(q);
        }
    }
    if me.frozen_t > 0.0 && w.rng.next() < 0.3 {
        let q = Particle::new(cx + w.rng.range(-25.0, 25.0), cy + w.rng.range(-25.0, 25.0), 0.0, 30.0, 0.5, 4.0, Color::new(200, 240, 255, 255), Shape::Star);
        w.parts.push(q);
    }

    // ---- momentum: roll and slide ----
    let mut push = 0.0;
    if can_act {
        if rl.is_key_down(me.keys.right) {
            push += 1.0;
        }
        if rl.is_key_down(me.keys.left) {
            push -= 1.0;
        }
    }
    if push != 0.0 {
        me.facing = push;
    }

    if me.dash_t > 0.0 {
        me.dash_t -= dt;
        me.vx = me.facing * DASH_SPEED;
        me.vy = 0.0;
    } else if push != 0.0 && me.vx * push < TOP_SPEED {
        me.vx += push * ACCEL * dt;
        if me.vx * push > TOP_SPEED {
            me.vx = push * TOP_SPEED;
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
    if can_act && rl.is_key_down(me.keys.down) && me.on_ground {
        me.charge = (me.charge + dt).min(1.0);
        me.crouch = CROUCH;
    } else if me.tap_timer <= 0.0 {
        me.charge = 0.0;
        me.crouch = 0.0;
    }
    if can_act && rl.is_key_pressed(me.keys.up) && me.on_ground && me.tap_timer <= 0.0 {
        if me.charge > 0.0 {
            let power = 1.0 + me.charge * (MAX_CHARGE - 1.0);
            launch(me, power);
        } else {
            me.tap_timer = TAP_CROUCH;
            me.crouch = CROUCH;
        }
    }
    if me.tap_timer > 0.0 {
        me.tap_timer -= dt;
        if me.tap_timer <= 0.0 {
            launch(me, 1.0);
        }
    }

    // ---- loadout abilities ----
    let loadout = me.abilities;
    if can_act && rl.is_key_pressed(me.keys.a1) && me.cd1 <= 0.0 {
        use_ability(loadout[0], me, foe, my_idx, solids, w);
        me.cd1 = loadout[0].cooldown();
    }
    if can_act && rl.is_key_pressed(me.keys.a2) && me.cd2 <= 0.0 {
        use_ability(loadout[1], me, foe, my_idx, solids, w);
        me.cd2 = loadout[1].cooldown();
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
    if can_act && rl.is_key_pressed(me.keys.melee) && me.melee_cd <= 0.0 && me.melee_t <= 0.0 {
        me.melee_t = MELEE_TIME;
        me.melee_cd = MELEE_CD;
        me.melee_hit = false;
    }

    step(me, solids, dt);

    if me.melee_t > 0.0 && !me.melee_hit {
        let progress = 1.0 - me.melee_t / MELEE_TIME;
        if (0.2..0.85).contains(&progress) {
            let rx = if me.facing > 0.0 { me.x + SIZE } else { me.x - MELEE_REACH };
            let zone = Rectangle::new(rx, me.y - 10.0, MELEE_REACH, SIZE + 20.0);
            if overlaps(&zone, &foe.rect()) {
                foe.hurt(MELEE_DMG, me.facing * 350.0, -180.0);
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
    me.eyes = if me.crouch > 0.0 || me.hurt_t > 0.0 || me.frozen_t > 0.0 {
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
    if style == Trail::Off {
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
fn draw_knife(d: &mut impl RaylibDraw, px: f32, py: f32, theta: f32, flip: f32) {
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
        let (w, h) = (x1 - x0, y1 - y0);
        let lx = (x0 + x1) / 2.0 * flip;
        let ly = (y0 + y1) / 2.0;
        let cx = px + lx * c - ly * s;
        let cy = py + lx * s + ly * c;
        d.draw_rectangle_pro(Rectangle::new(cx, cy, w, h), Vector2::new(w / 2.0, h / 2.0), theta, col);
    }
}

fn draw_player(d: &mut impl RaylibDraw, p: &Player) {
    let w = SIZE;
    let h = SIZE - p.crouch;
    let cx = p.x + SIZE / 2.0;
    let cy = p.y + SIZE - h / 2.0;
    let body = if p.hurt_t > 0.0 { Color::WHITE } else { p.kind.color() };
    d.draw_rectangle_pro(Rectangle::new(cx, cy, w, h), Vector2::new(w / 2.0, h / 2.0), p.rot, body);

    let (look_x, look_y, ew, eh) = match p.eyes {
        Eyes::Forward => (0.0, 0.0, 2.0, 2.0),
        Eyes::Left => (-2.0, 0.0, 2.0, 2.0),
        Eyes::Right => (2.0, 0.0, 2.0, 2.0),
        Eyes::UpWide => (0.0, -1.5, 3.0, 3.0),
        Eyes::Squint => (0.0, 0.0, 3.0, 1.0),
    };
    let eye_col = if p.hurt_t > 0.0 { Color::BLACK } else { Color::WHITE };
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
    if p.melee_t > 0.0 {
        let progress = 1.0 - p.melee_t / MELEE_TIME;
        let angle = (-60.0 + 150.0 * progress) * p.facing; // raised -> slashed down
        draw_knife(d, cx + p.facing * SIZE * 0.35, cy + SIZE * 0.1, angle, p.facing);
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
            let prog = 1.0 - s.t / STRIKE_DELAY;
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

fn draw_hud(d: &mut impl RaylibDraw, players: &[Player; 2]) {
    let labels = [("F", "G"), ("COMMA", "PERIOD")];
    for (i, p) in players.iter().enumerate() {
        let bar_w = 420.0;
        let x = if i == 0 { 20.0 } else { W as f32 - 20.0 - bar_w };
        d.draw_rectangle(x as i32 - 3, 17, bar_w as i32 + 6, 30, Color::DARKGRAY);
        let fill = bar_w * p.hp / MAX_HP;
        let fx = if i == 0 { x } else { x + bar_w - fill };
        d.draw_rectangle(fx as i32, 20, fill as i32, 24, p.kind.color());
        text(d, &format!("P{} {}", i + 1, p.kind.name()), x as i32, 52, 28, Color::WHITE);

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
    }
}

fn draw_world(d: &mut impl RaylibDraw, theme: Theme, map: &Map, w: &World, t: f32) {
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
        }
    }
    if let Some(hz) = map.hazard {
        let pulse = 0.5 + 0.5 * (t * 4.0).sin();
        d.draw_rectangle((hz.x - 6.0) as i32, hz.y as i32 - 6, hz.width as i32 + 12, 24, Color::new(255, (90.0 + 60.0 * pulse) as u8, 20, 255));
        d.draw_rectangle(hz.x as i32, hz.y as i32 - 3, hz.width as i32, 8, Color::new(255, 220, 90, 230));
        for i in 0..8 {
            let bx = hz.x + 20.0 + i as f32 * 40.0;
            let by = hz.y - 6.0 - ((t * 20.0 + i as f32 * 9.0) % 14.0);
            d.draw_circle(bx as i32, by as i32, 3.0, Color::new(255, 200, 60, 220));
        }
    }
    draw_strikes(d, &w.strikes, t);
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

const ROWS: usize = 5;
const ROW_NAMES: [&str; ROWS] = ["CHARACTER", "ABILITY 1", "ABILITY 2", "TRAIL", "TRAIL COLOR"];

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
        },
        Keys {
            left: KeyboardKey::KEY_LEFT,
            right: KeyboardKey::KEY_RIGHT,
            up: KeyboardKey::KEY_UP,
            down: KeyboardKey::KEY_DOWN,
            a1: KeyboardKey::KEY_COMMA,
            a2: KeyboardKey::KEY_PERIOD,
            melee: KeyboardKey::KEY_SLASH,
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
    let mut start_theme = 0usize;
    let mut round_no = 1u32;
    let mut wins = [0u32; 2];
    let mut series_over = false;

    // ---- cinematic state ----
    let mut ko_timer = 0.0f32;
    let mut ko_focus = (W as f32 / 2.0, H as f32 / 2.0);

    while !rl.window_should_close() {
        let dt = rl.get_frame_time().min(0.05);
        let t = rl.get_time() as f32;

        // screen effects calm down in real time
        w.fx.shake *= 0.86f32.powf(dt * 60.0);
        if w.fx.shake < 0.3 {
            w.fx.shake = 0.0;
        }
        w.fx.punch *= 0.88f32.powf(dt * 60.0);
        w.fx.flash = (w.fx.flash - dt * 2.2).max(0.0);

        if selecting {
            // ---- loadout menus ----
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
                            _ => s.color = cycle(s.color, TRAIL_COLOR_NAMES.len(), dir),
                        }
                    }
                }
                if rl.is_key_pressed(ready_keys[i]) {
                    ready[i] = !ready[i];
                }
            }
            if rl.is_key_pressed(KeyboardKey::KEY_M) && !(ready[0] && ready[1]) {
                theme_idx = cycle(theme_idx, THEMES.len(), 1);
                map = make_map(THEMES[theme_idx]);
            }
            if rl.is_key_pressed(KeyboardKey::KEY_N) && !(ready[0] && ready[1]) {
                rounds_idx = cycle(rounds_idx, round_options.len(), 1);
            }

            // animated previews so you can see the trail you picked
            for i in 0..2 {
                let panel_x = 40.0 + i as f32 * 640.0;
                let pv = &mut previews[i];
                pv.apply_setup(&setups[i]);
                pv.x = panel_x + 500.0 + (t * 1.6).sin() * 55.0;
                pv.y = 260.0;
                pv.vx = (t * 1.6).cos() * 55.0 * 1.6;
                pv.vy = 0.0;
                pv.facing = if pv.vx >= 0.0 { 1.0 } else { -1.0 };
                pv.rot += pv.vx * dt / (SIZE / 2.0) * 57.3;
                emit_trail(pv, &mut w, t, dt);
            }
            update_particles(&mut w.parts, dt);

            if ready[0] && ready[1] {
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

            if result.is_none() || ko_active {
                if dt_sim > 0.0 {
                    let hp_before = [players[0].hp, players[1].hp];
                    for i in 0..2 {
                        let (a, b) = players.split_at_mut(1);
                        let (me, foe) = if i == 0 { (&mut a[0], &mut b[0]) } else { (&mut b[0], &mut a[0]) };
                        update_player(&rl, me, foe, i, &map.solids, &mut w, dt_sim);
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

                    // lava hurts
                    if let Some(hz) = map.hazard {
                        for p in players.iter_mut() {
                            if overlaps(&p.rect(), &hz) {
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

                    // ---- impact feel: every real hit shakes, freezes and sparks ----
                    for i in 0..2 {
                        let drop = hp_before[i] - players[i].hp;
                        if drop >= 3.0 && players[i].hp < hp_before[i] && players[i].hurt_t > 0.2 {
                            let (px, py) = players[i].center();
                            w.shake((5.0 + drop * 0.6).min(24.0));
                            w.fx.hitstop = w.fx.hitstop.max((0.03 + drop * 0.003).min(0.12));
                            w.fx.punch = w.fx.punch.max((0.02 + drop * 0.002).min(0.08));
                            if drop >= 18.0 {
                                w.fx.flash = w.fx.flash.max(0.25);
                            }
                            w.burst(px, py, 10 + drop as usize, Color::WHITE, 420.0, 0.35, 4.0, Shape::Streak, 0.0);
                            w.burst(px, py, 6, players[i].kind.color(), 300.0, 0.5, 9.0, Shape::Square, 500.0);
                            w.ring(px, py, 60.0 + drop * 2.0, Color::WHITE);
                        }
                    }

                    // ---- end of round: KO cinematic and tally ----
                    if result.is_none() {
                        let dead = [players[0].hp <= 0.0, players[1].hp <= 0.0];
                        let winner: Option<Option<usize>> = match dead {
                            [true, true] => Some(None),
                            [true, false] => Some(Some(1)),
                            [false, true] => Some(Some(0)),
                            _ => None,
                        };
                        if let Some(win) = winner {
                            if let Some(i) = win {
                                wins[i] += 1;
                            }
                            series_over = round_no >= round_options[rounds_idx];
                            result = Some(match win {
                                Some(i) => format!("P{} ({}) WINS THE ROUND", i + 1, players[i].kind.name()),
                                None => "ROUND DRAWN".to_string(),
                            });
                            ko_timer = KO_TIME;
                            w.fx.flash = 0.9;
                            w.shake(26.0);
                            w.fx.hitstop = 0.18;
                            // the loser(s) burst apart
                            let mut fx = 0.0;
                            let mut fy = 0.0;
                            let mut n = 0.0;
                            for i in 0..2 {
                                if dead[i] {
                                    let (px, py) = players[i].center();
                                    let col = players[i].kind.color();
                                    w.burst(px, py, 36, col, 600.0, 1.1, 14.0, Shape::Square, 500.0);
                                    w.burst(px, py, 20, Color::WHITE, 700.0, 0.6, 7.0, Shape::Star, 0.0);
                                    w.burst(px, py, 20, Color::WHITE, 800.0, 0.5, 3.0, Shape::Streak, 0.0);
                                    w.ring(px, py, 260.0, Color::WHITE);
                                    w.ring(px, py, 170.0, col);
                                    fx += px;
                                    fy += py;
                                    n += 1.0;
                                }
                            }
                            ko_focus = (fx / n, fy / n);
                        }
                    }
                }
            } else {
                // between rounds: let leftover effects fade, then wait for a key
                update_booms(&mut w.booms, dt);
                update_particles(&mut w.parts, dt);
                update_strikes(&mut players, &mut w, dt);
                if rl.is_key_pressed(KeyboardKey::KEY_R) {
                    selecting = true;
                    ready = [false, false];
                    w.clear();
                    theme_idx = start_theme; // back to the map you picked
                    map = make_map(THEMES[theme_idx]);
                } else if !series_over
                    && (rl.is_key_pressed(KeyboardKey::KEY_SPACE) || rl.is_key_pressed(KeyboardKey::KEY_ENTER))
                {
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
                }
            }
        }

        // ---- draw ----
        let theme = THEMES[theme_idx];
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::BLACK);

        if selecting {
            draw_scenery(&mut d, theme, t);
            draw_world(&mut d, theme, &map, &World::new(), t);
            center_text(&mut d, "PAUSI", 14, 60, Color::WHITE);
            center_text(
                &mut d,
                &format!("STARTING MAP: {}  ({})   - press M to change", theme.name(), theme.blurb()),
                84,
                24,
                Color::YELLOW,
            );

            for i in 0..2 {
                let px = 40 + i as i32 * 640;
                let py = 130;
                d.draw_rectangle(px, py, 600, 440, Color::new(0, 0, 0, 165));
                d.draw_rectangle_lines(px, py, 600, 440, Color::WHITE);
                text(&mut d, &format!("PLAYER {}", i + 1), px + 16, py + 10, 30, previews[i].kind.color());

                let s = &setups[i];
                let values = [
                    KINDS[s.kind].name(),
                    ABILITIES[s.abil[0]].name(),
                    ABILITIES[s.abil[1]].name(),
                    TRAIL_STYLES[s.trail].name(),
                    TRAIL_COLOR_NAMES[s.color],
                ];
                for r in 0..ROWS {
                    let ry = py + 60 + r as i32 * 48;
                    let active = row[i] == r && !ready[i];
                    if active {
                        d.draw_rectangle(px + 8, ry - 4, 584, 40, Color::new(255, 255, 255, 60));
                    }
                    text(&mut d, ROW_NAMES[r], px + 20, ry, 24, Color::WHITE);
                    let val = if active { format!("<  {}  >", values[r]) } else { values[r].to_string() };
                    text(&mut d, &val, px + 190, ry, 26, if active { Color::YELLOW } else { Color::LIGHTGRAY });
                }

                let ks = if i == 0 { ("F", "G") } else { ("COMMA", "PERIOD") };
                let a1 = ABILITIES[s.abil[0]];
                let a2 = ABILITIES[s.abil[1]];
                text(&mut d, &format!("{}: {} - {}", ks.0, a1.name(), a1.blurb()), px + 20, py + 310, 20, Color::WHITE);
                text(&mut d, &format!("{}: {} - {}", ks.1, a2.name(), a2.blurb()), px + 20, py + 340, 20, Color::WHITE);
                let knife = if i == 0 { "E" } else { "SLASH" };
                text(&mut d, &format!("{}: KNIFE - melee swing, 10 dmg, no cooldown", knife), px + 20, py + 370, 20, Color::WHITE);
                if ready[i] {
                    text(&mut d, "READY!", px + 440, py + 400, 32, Color::LIME);
                }
            }
            draw_particles(&mut d, &w.parts);
            for i in 0..2 {
                draw_player(&mut d, &previews[i]);
            }
            text(&mut d, "P1: W/S pick row  A/D change  F ready", 40, 585, 24, Color::YELLOW);
            text(&mut d, "P2: Up/Down pick row  Left/Right change  L ready", 640, 585, 24, Color::YELLOW);
            center_text(
                &mut d,
                &format!("ROUNDS: {}   (press N to change - every round switches to the next map)", round_options[rounds_idx]),
                622,
                26,
                Color::WHITE,
            );
            center_text(&mut d, "Both players ready = fight!", 658, 24, Color::LIME);
        } else {
            // ---- the cinematic camera: shake, zoom punch, KO zoom ----
            let ko_prog = if ko_timer > 0.0 { ease(1.0 - ko_timer / KO_TIME) } else { 0.0 };
            let zoom = 1.0 + w.fx.punch + 0.45 * ko_prog;
            let shake = (w.rng.range(-1.0, 1.0) * w.fx.shake, w.rng.range(-1.0, 1.0) * w.fx.shake);
            let cam = camera_for(zoom, ko_focus, ko_prog, shake);
            {
                let mut m = d.begin_mode2D(cam);
                draw_scenery(&mut m, theme, t);
                draw_world(&mut m, theme, &map, &w, t);
                draw_particles(&mut m, &w.parts);
                for p in players.iter() {
                    if p.hp > 0.0 {
                        draw_player(&mut m, p);
                    }
                }
            }

            draw_hud(&mut d, &players);
            // round counter and running tally, top center
            center_text(&mut d, &format!("ROUND {} / {}", round_no, round_options[rounds_idx]), 12, 24, Color::WHITE);
            center_text(&mut d, &format!("{}  -  {}", wins[0], wins[1]), 42, 44, Color::YELLOW);
            center_text(&mut d, theme.name(), 92, 20, Color::LIGHTGRAY);
            let p1 = "P1: A/D move  W jump  S charge  F/G abilities  E knife";
            let p2 = "P2: arrows  COMMA / PERIOD abilities  SLASH knife";
            text(&mut d, p1, 15, H - 30, 20, Color::WHITE);
            text(&mut d, p2, W - text_width(p2, 20) - 15, H - 30, 20, Color::WHITE);

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

            // ---- round result banner (after the KO cinematic) ----
            if ko_timer <= 0.0 {
                if let Some(msg) = &result {
                    d.draw_rectangle(0, 190, W, 260, Color::new(0, 0, 0, 150));
                    center_text(&mut d, msg, 205, 56, Color::YELLOW);
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
