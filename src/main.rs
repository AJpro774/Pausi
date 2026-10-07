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
    fn blurb(self) -> [&'static str; 2] {
        match self {
            Kind::Dasher => ["1: Dash - rams for 15 dmg", "2: Slam (in air) - shockwave, 20 dmg"],
            Kind::Bomber => ["1: Bomb - 25 dmg, blast launches you too", "2: Air hop - boost anywhere"],
            Kind::Shielder => ["1: Shield - blocks damage, reflects bombs", "2: Pulse - 12 dmg, big knockback"],
        }
    }
    // (ability 1 cooldown, ability 2 cooldown)
    fn cooldowns(self) -> (f32, f32) {
        match self {
            Kind::Dasher => (2.0, 3.0),
            Kind::Bomber => (1.5, 2.5),
            Kind::Shielder => (3.0, 1.5),
        }
    }
}

#[derive(Clone, Copy)]
struct Keys {
    left: KeyboardKey,
    right: KeyboardKey,
    up: KeyboardKey,
    down: KeyboardKey,
    a1: KeyboardKey,
    a2: KeyboardKey,
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
    keys: Keys,
    cd1: f32,
    cd2: f32,
    dash_t: f32,
    dash_hit: bool,
    slam: bool,
    shield_t: f32,
    hurt_t: f32,
}

impl Player {
    fn new(kind: Kind, x: f32, facing: f32, keys: Keys) -> Self {
        Player {
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
            kind,
            keys,
            cd1: 0.0,
            cd2: 0.0,
            dash_t: 0.0,
            dash_hit: false,
            slam: false,
            shield_t: 0.0,
            hurt_t: 0.0,
        }
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
        self.vx += kx;
        if ky != 0.0 {
            self.vy = ky;
            self.on_ground = false;
        }
        self.hurt_t = 0.25;
    }
}

struct Solid {
    r: Rectangle,
    one_way: bool, // platforms: jump up through them, land on top
}

struct Bomb {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    owner: usize,
    life: f32,
}

struct Boom {
    x: f32,
    y: f32,
    r: f32,
    t: f32,
}

fn overlaps(a: &Rectangle, b: &Rectangle) -> bool {
    a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y
}

fn level() -> Vec<Solid> {
    let floor_y = H as f32 - FLOOR_H;
    let block = |x, y, w, h| Solid { r: Rectangle::new(x, y, w, h), one_way: false };
    let plat = |x, y, w| Solid { r: Rectangle::new(x, y, w, 14.0), one_way: true };
    vec![
        block(0.0, floor_y, W as f32, FLOOR_H), // floor
        block(590.0, floor_y - 70.0, 100.0, 70.0), // center block
        block(0.0, 300.0, 70.0, 30.0), // wall ledges
        block(W as f32 - 70.0, 300.0, 70.0, 30.0),
        plat(140.0, 520.0, 240.0),
        plat(900.0, 520.0, 240.0),
        plat(520.0, 420.0, 240.0),
        plat(200.0, 330.0, 200.0),
        plat(880.0, 330.0, 200.0),
    ]
}

// move + collide with walls, blocks and platforms
fn step(p: &mut Player, solids: &[Solid], dt: f32) {
    p.x += p.vx * dt;
    let max_x = W as f32 - SIZE;
    if p.x < 0.0 {
        p.x = 0.0;
        p.vx = 0.0;
    } else if p.x > max_x {
        p.x = max_x;
        p.vx = 0.0;
    }
    for s in solids {
        if s.one_way || !overlaps(&p.rect(), &s.r) {
            continue;
        }
        if p.x + SIZE / 2.0 < s.r.x + s.r.width / 2.0 {
            p.x = s.r.x - SIZE;
        } else {
            p.x = s.r.x + s.r.width;
        }
        p.vx = 0.0;
    }

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
        } else if !s.one_way && p.vy < 0.0 {
            p.y = s.r.y + s.r.height; // bonk head
            p.vy = 0.0;
        }
    }
    if p.y < 0.0 {
        p.y = 0.0;
        p.vy = p.vy.max(0.0);
    }
}

fn launch(p: &mut Player, power: f32) {
    p.vy = -(2.0 * GRAVITY * JUMP * power).sqrt();
    p.on_ground = false;
    p.charge = 0.0;
    p.crouch = 0.0;
}

#[allow(clippy::too_many_arguments)]
fn update_player(
    rl: &RaylibHandle,
    me: &mut Player,
    foe: &mut Player,
    my_idx: usize,
    solids: &[Solid],
    bombs: &mut Vec<Bomb>,
    booms: &mut Vec<Boom>,
    dt: f32,
) {
    me.cd1 = (me.cd1 - dt).max(0.0);
    me.cd2 = (me.cd2 - dt).max(0.0);
    me.shield_t = (me.shield_t - dt).max(0.0);
    me.hurt_t = (me.hurt_t - dt).max(0.0);

    // ---- momentum: roll and slide ----
    let mut push = 0.0;
    if rl.is_key_down(me.keys.right) {
        push += 1.0;
    }
    if rl.is_key_down(me.keys.left) {
        push -= 1.0;
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
    if rl.is_key_down(me.keys.down) && me.on_ground {
        me.charge = (me.charge + dt).min(1.0);
        me.crouch = CROUCH;
    } else if me.tap_timer <= 0.0 {
        me.charge = 0.0;
        me.crouch = 0.0;
    }
    if rl.is_key_pressed(me.keys.up) && me.on_ground && me.tap_timer <= 0.0 {
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

    // ---- abilities ----
    let cd = me.kind.cooldowns();
    if rl.is_key_pressed(me.keys.a1) && me.cd1 <= 0.0 {
        match me.kind {
            Kind::Dasher => {
                me.dash_t = DASH_TIME;
                me.dash_hit = false;
                me.cd1 = cd.0;
            }
            Kind::Bomber => {
                bombs.push(Bomb {
                    x: me.x + SIZE / 2.0,
                    y: me.y + SIZE / 2.0,
                    vx: me.facing * 550.0 + me.vx * 0.5,
                    vy: -300.0,
                    owner: my_idx,
                    life: 3.0,
                });
                me.cd1 = cd.0;
            }
            Kind::Shielder => {
                me.shield_t = SHIELD_TIME;
                me.cd1 = cd.0;
            }
        }
    }
    if rl.is_key_pressed(me.keys.a2) && me.cd2 <= 0.0 {
        match me.kind {
            Kind::Dasher => {
                if !me.on_ground {
                    me.slam = true;
                    me.cd2 = cd.1;
                }
            }
            Kind::Bomber => {
                me.vy = -(2.0 * GRAVITY * JUMP * 1.2).sqrt();
                me.on_ground = false;
                me.cd2 = cd.1;
                booms.push(Boom { x: me.x + SIZE / 2.0, y: me.y + SIZE, r: 50.0, t: 0.0 });
            }
            Kind::Shielder => {
                let reach = 170.0;
                let rx = if me.facing > 0.0 { me.x + SIZE } else { me.x - reach };
                let zone = Rectangle::new(rx, me.y, reach, SIZE);
                if overlaps(&zone, &foe.rect()) {
                    foe.hurt(12.0, me.facing * 700.0, -250.0);
                }
                booms.push(Boom {
                    x: if me.facing > 0.0 { me.x + SIZE + 40.0 } else { me.x - 40.0 },
                    y: me.y + SIZE / 2.0,
                    r: 80.0,
                    t: 0.0,
                });
                me.cd2 = cd.1;
            }
        }
    }
    if me.slam {
        me.vy = me.vy.max(SLAM_SPEED);
    }

    step(me, solids, dt);

    // ---- ability results ----
    if me.dash_t > 0.0 && !me.dash_hit && overlaps(&me.rect(), &foe.rect()) {
        foe.hurt(15.0, me.facing * 600.0, -250.0);
        me.dash_hit = true;
        me.vx *= 0.3;
    }
    if me.slam && me.on_ground {
        me.slam = false;
        let (mx, _) = me.center();
        let (fx, _) = foe.center();
        booms.push(Boom { x: mx, y: me.y + SIZE, r: 250.0, t: 0.0 });
        if (fx - mx).abs() < 250.0 && foe.y + SIZE >= me.y + SIZE - 40.0 {
            foe.hurt(20.0, (fx - mx).signum() * 500.0, -350.0);
        }
    }

    // ---- eyes ----
    me.eyes = if me.crouch > 0.0 || me.hurt_t > 0.0 {
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

fn explode(players: &mut [Player; 2], owner: usize, cx: f32, cy: f32, booms: &mut Vec<Boom>) {
    booms.push(Boom { x: cx, y: cy, r: BLAST_R, t: 0.0 });
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

    if p.shield_t > 0.0 {
        d.draw_circle(cx as i32, cy as i32, SIZE * 0.95, Color::new(120, 220, 255, 70));
        d.draw_circle_lines(cx as i32, cy as i32, SIZE * 0.95, Color::new(120, 220, 255, 255));
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

fn draw_hud(d: &mut impl RaylibDraw, players: &[Player; 2]) {
    let labels = [("F", "G"), (",", ".")];
    for (i, p) in players.iter().enumerate() {
        let bar_w = 420.0;
        let x = if i == 0 { 20.0 } else { W as f32 - 20.0 - bar_w };
        d.draw_rectangle(x as i32 - 3, 17, bar_w as i32 + 6, 30, Color::DARKGRAY);
        let fill = bar_w * p.hp / MAX_HP;
        let fx = if i == 0 { x } else { x + bar_w - fill };
        d.draw_rectangle(fx as i32, 20, fill as i32, 24, p.kind.color());
        d.draw_text(&format!("P{} {}", i + 1, p.kind.name()), x as i32, 50, 20, Color::BLACK);

        let cds = [(p.cd1, p.kind.cooldowns().0, labels[i].0), (p.cd2, p.kind.cooldowns().1, labels[i].1)];
        for (k, (cd, max, label)) in cds.iter().enumerate() {
            let px = x + 200.0 + k as f32 * 100.0;
            d.draw_rectangle(px as i32, 52, 80, 14, Color::DARKGRAY);
            let ready = 1.0 - cd / max;
            let col = if *cd <= 0.0 { Color::LIME } else { Color::GRAY };
            d.draw_rectangle(px as i32, 52, (80.0 * ready) as i32, 14, col);
            d.draw_text(label, px as i32 + 36, 52, 14, Color::BLACK);
        }
    }
}

fn draw_world(d: &mut impl RaylibDraw, solids: &[Solid], bombs: &[Bomb], booms: &[Boom]) {
    for s in solids {
        let col = if s.one_way { Color::BROWN } else { Color::DARKGRAY };
        d.draw_rectangle(s.r.x as i32, s.r.y as i32, s.r.width as i32, s.r.height as i32, col);
    }
    for b in bombs {
        d.draw_circle(b.x as i32, b.y as i32, 9.0, Color::BLACK);
        d.draw_circle(b.x as i32, b.y as i32, 4.0, Color::RED);
    }
    for b in booms {
        let p = (b.t / BOOM_TIME).min(1.0);
        let r = b.r * (0.4 + 0.6 * p);
        let fade = (255.0 * (1.0 - p)) as u8;
        d.draw_circle(b.x as i32, b.y as i32, r, Color::new(255, 200, 50, fade / 2));
        d.draw_circle_lines(b.x as i32, b.y as i32, r, Color::new(255, 120, 0, fade));
    }
}

fn center_text(d: &mut impl RaylibDraw, text: &str, y: i32, size: i32, color: Color) {
    let w = (text.len() as f32 * size as f32 * 0.5) as i32;
    d.draw_text(text, W / 2 - w / 2, y, size, color);
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
        },
        Keys {
            left: KeyboardKey::KEY_LEFT,
            right: KeyboardKey::KEY_RIGHT,
            up: KeyboardKey::KEY_UP,
            down: KeyboardKey::KEY_DOWN,
            a1: KeyboardKey::KEY_COMMA,
            a2: KeyboardKey::KEY_PERIOD,
        },
    ];
    let spawns = [(120.0, 1.0), (W as f32 - 120.0 - SIZE, -1.0)];

    let solids = level();
    let mut bombs: Vec<Bomb> = Vec::new();
    let mut booms: Vec<Boom> = Vec::new();

    let mut selecting = true;
    let mut sel = [0usize, 1usize];
    let mut ready = [false, false];
    let mut players = [
        Player::new(KINDS[0], spawns[0].0, spawns[0].1, keys[0]),
        Player::new(KINDS[1], spawns[1].0, spawns[1].1, keys[1]),
    ];
    let mut result: Option<String> = None;

    while !rl.window_should_close() {
        let dt = rl.get_frame_time().min(0.05);

        if selecting {
            for i in 0..2 {
                if !ready[i] {
                    if rl.is_key_pressed(keys[i].left) {
                        sel[i] = (sel[i] + KINDS.len() - 1) % KINDS.len();
                    }
                    if rl.is_key_pressed(keys[i].right) {
                        sel[i] = (sel[i] + 1) % KINDS.len();
                    }
                }
                if rl.is_key_pressed(keys[i].a1) {
                    ready[i] = !ready[i];
                }
            }
            if ready[0] && ready[1] {
                players = [
                    Player::new(KINDS[sel[0]], spawns[0].0, spawns[0].1, keys[0]),
                    Player::new(KINDS[sel[1]], spawns[1].0, spawns[1].1, keys[1]),
                ];
                bombs.clear();
                booms.clear();
                result = None;
                selecting = false;
            }
        } else {
            if result.is_none() {
                for i in 0..2 {
                    let (a, b) = players.split_at_mut(1);
                    let (me, foe) = if i == 0 { (&mut a[0], &mut b[0]) } else { (&mut b[0], &mut a[0]) };
                    update_player(&rl, me, foe, i, &solids, &mut bombs, &mut booms, dt);
                }

                // ---- bombs ----
                let mut i = 0;
                while i < bombs.len() {
                    let b = &mut bombs[i];
                    b.vy += BOMB_GRAVITY * dt;
                    b.x += b.vx * dt;
                    b.y += b.vy * dt;
                    b.life -= dt;
                    let mut blow = b.life <= 0.0 || b.x < 0.0 || b.x > W as f32 || b.y > H as f32;
                    let br = Rectangle::new(b.x - 8.0, b.y - 8.0, 16.0, 16.0);
                    for s in &solids {
                        if !s.one_way && overlaps(&br, &s.r) {
                            blow = true;
                        }
                    }
                    let foe = 1 - b.owner;
                    if overlaps(&br, &players[foe].rect()) {
                        if players[foe].shield_t > 0.0 {
                            // reflected back at the thrower
                            b.vx = -b.vx;
                            b.vy = -b.vy.abs();
                            b.owner = foe;
                            b.life = 3.0;
                        } else {
                            blow = true;
                        }
                    }
                    if blow {
                        let (bx, by, owner) = (b.x, b.y, b.owner);
                        bombs.remove(i);
                        explode(&mut players, owner, bx, by, &mut booms);
                    } else {
                        i += 1;
                    }
                }
                for b in booms.iter_mut() {
                    b.t += dt;
                }
                booms.retain(|b| b.t < BOOM_TIME);

                let dead = [players[0].hp <= 0.0, players[1].hp <= 0.0];
                result = match dead {
                    [true, true] => Some("DRAW".to_string()),
                    [true, false] => Some(format!("P2 ({}) WINS", players[1].kind.name())),
                    [false, true] => Some(format!("P1 ({}) WINS", players[0].kind.name())),
                    _ => None,
                };
            } else {
                for b in booms.iter_mut() {
                    b.t += dt;
                }
                booms.retain(|b| b.t < BOOM_TIME);
                if rl.is_key_pressed(KeyboardKey::KEY_R) {
                    selecting = true;
                    ready = [false, false];
                }
            }
        }

        // ---- draw ----
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::SKYBLUE);

        if selecting {
            center_text(&mut d, "PAUSI", 40, 60, Color::BLACK);
            center_text(&mut d, "pick a square - ready up with your ability 1 key", 110, 24, Color::DARKBLUE);
            for (k, kind) in KINDS.iter().enumerate() {
                let x = 330.0 + k as f32 * 240.0;
                d.draw_rectangle(x as i32, 230, 100, 100, kind.color());
                d.draw_text(kind.name(), x as i32 - 5, 345, 22, Color::BLACK);
                for i in 0..2 {
                    if sel[i] == k {
                        let col = if ready[i] { Color::DARKGREEN } else { Color::BLACK };
                        let y = 375 + i as i32 * 26;
                        d.draw_text(&format!("P{}{}", i + 1, if ready[i] { " READY" } else { "" }), x as i32, y, 22, col);
                    }
                }
            }
            for i in 0..2 {
                let k = KINDS[sel[i]];
                let x = if i == 0 { 60 } else { 700 };
                d.draw_text(&format!("P{}: {}", i + 1, k.name()), x, 470, 26, k.color());
                d.draw_text(k.blurb()[0], x, 505, 20, Color::BLACK);
                d.draw_text(k.blurb()[1], x, 530, 20, Color::BLACK);
            }
            d.draw_text("P1: A/D choose, F ready", 60, 640, 20, Color::DARKGRAY);
            d.draw_text("P2: Left/Right choose, ',' ready", 700, 640, 20, Color::DARKGRAY);
        } else {
            draw_world(&mut d, &solids, &bombs, &booms);
            for p in players.iter() {
                draw_player(&mut d, p);
            }
            draw_hud(&mut d, &players);
            d.draw_text("P1: A/D move, W jump, S charge, F/G abilities", 20, H - 30, 18, Color::WHITE);
            d.draw_text(
                "P2: arrows, ','/'.' abilities",
                W - 300,
                H - 30,
                18,
                Color::WHITE,
            );
            if let Some(text) = &result {
                center_text(&mut d, text, 250, 64, Color::BLACK);
                center_text(&mut d, "press R to pick again", 330, 28, Color::DARKBLUE);
            }
        }
    }
}
