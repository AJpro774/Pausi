use raylib::prelude::*;

// ---- Alexander's numbers ----
const W: i32 = 1280; // about 2/3 of a 1920x1080 screen
const H: i32 = 720;
const SIZE: f32 = 150.0; // 10x the old 15px square
const TOP_SPEED: f32 = 350.0; // px per second
const ACCEL: f32 = TOP_SPEED / 0.25; // reach top speed in 250ms (= 1400)
const ICE: f32 = 400.0; // slow-down after letting go (350 / 400 = 0.875s slide)
const JUMP: f32 = 200.0; // normal jump height in pixels
const MAX_CHARGE: f32 = 3.5; // full charge = 3.5x jump
const GRAVITY: f32 = 250.0; // lower = floatier
const TAP_CROUCH: f32 = 0.15; // 150ms crouch before a normal jump
const CROUCH: f32 = 5.0; // crouch depth in pixels
const SQUISH: f32 = 0.25; // 25% squish against a wall
const POP_TIME: f32 = 0.35; // 350ms to pop back
const PUSH_OFF: f32 = 2.0; // push off the wall by 2px

#[derive(Clone, Copy, PartialEq)]
enum Eyes {
    Forward,
    Left,
    Right,
    UpWide,
    Squint,
}

struct Square {
    x: f32, // left edge
    y: f32, // top edge (y grows DOWN the screen)
    vel_x: f32, // sideways speed (momentum)
    vel_y: f32,
    on_ground: bool,
    rotation: f32,
    charge: f32, // 0 to 1 second of holding down
    crouch: f32,
    tap_timer: f32,
    squish: f32,
    squish_side: f32, // which wall: 1 = right, -1 = left
    holding_wall: bool,
    pop_timer: f32,
    eyes: Eyes,
}

fn launch(sq: &mut Square, power: f32) {
    let height = JUMP * power;
    sq.vel_y = -(2.0 * GRAVITY * height).sqrt(); // negative = up
    sq.on_ground = false;
    sq.charge = 0.0;
    sq.crouch = 0.0;
    sq.eyes = Eyes::UpWide;
}

fn main() {
    let (mut rl, thread) = raylib::init().size(W, H).title("SteamUP").build();
    rl.set_target_fps(60);

    let right_edge = W as f32 - SIZE;
    let ground = H as f32 - SIZE;

    let mut sq = Square {
        x: 0.0,
        y: 0.0,
        vel_x: 0.0,
        vel_y: 0.0,
        on_ground: false,
        rotation: 0.0,
        charge: 0.0,
        crouch: 0.0,
        tap_timer: 0.0,
        squish: 0.0,
        squish_side: 1.0,
        holding_wall: false,
        pop_timer: 0.0,
        eyes: Eyes::Forward,
    };

    while !rl.window_should_close() {
        let dt = rl.get_frame_time();

        // ---- momentum: roll and slide ----
        let mut push = 0.0;
        if rl.is_key_down(KeyboardKey::KEY_RIGHT) { push += 1.0; }
        if rl.is_key_down(KeyboardKey::KEY_LEFT) { push -= 1.0; }

        if push != 0.0 {
            sq.vel_x += push * ACCEL * dt;
        } else {
            let slow = (ICE * dt).min(sq.vel_x.abs());
            sq.vel_x -= sq.vel_x.signum() * slow;
        }
        sq.vel_x = sq.vel_x.clamp(-TOP_SPEED, TOP_SPEED);
        sq.x += sq.vel_x * dt;
        sq.rotation += sq.vel_x * dt / (SIZE / 2.0) * 57.3; // roll

        // when it stops, settle flat on the nearest side
        if sq.vel_x == 0.0 {
            let flat = (sq.rotation / 90.0).round() * 90.0;
            sq.rotation += (flat - sq.rotation) * (10.0 * dt).min(1.0);
        }

        // eyes look where you're going
        if sq.on_ground && sq.crouch == 0.0 && !sq.holding_wall {
            sq.eyes = if push > 0.0 { Eyes::Right } else if push < 0.0 { Eyes::Left } else { Eyes::Forward };
        }

        // ---- walls: stop, squish while held, pop back on release ----
        if sq.x <= 0.0 || sq.x >= right_edge {
            sq.x = sq.x.clamp(0.0, right_edge);
            sq.vel_x = 0.0;
            let wall = if sq.x <= 0.0 { -1.0 } else { 1.0 };
            if push == wall && sq.pop_timer <= 0.0 {
                sq.holding_wall = true;
                sq.squish_side = wall;
                sq.squish = SQUISH;
                sq.eyes = Eyes::Squint;
            }
        }
        if sq.holding_wall && push != sq.squish_side {
            sq.holding_wall = false;
            sq.pop_timer = POP_TIME;
            sq.x -= sq.squish_side * PUSH_OFF;
            sq.eyes = Eyes::Forward;
        }
        if sq.pop_timer > 0.0 {
            sq.pop_timer = (sq.pop_timer - dt).max(0.0);
            sq.squish = SQUISH * (sq.pop_timer / POP_TIME);
        }

        // ---- crouch and charge ----
        if rl.is_key_down(KeyboardKey::KEY_DOWN) && sq.on_ground {
            sq.charge = (sq.charge + dt).min(1.0);
            sq.crouch = CROUCH;
            sq.eyes = Eyes::Squint;
        } else if sq.tap_timer <= 0.0 {
            sq.charge = 0.0;
            sq.crouch = 0.0;
        }

        // ---- jump ----
        if rl.is_key_pressed(KeyboardKey::KEY_UP) && sq.on_ground && sq.tap_timer <= 0.0 {
            if sq.charge > 0.0 {
                let power = 1.0 + sq.charge * (MAX_CHARGE - 1.0); // 1x up to 3.5x
                launch(&mut sq, power);
            } else {
                sq.tap_timer = TAP_CROUCH;
                sq.crouch = CROUCH;
                sq.eyes = Eyes::Squint;
            }
        }
        if sq.tap_timer > 0.0 {
            sq.tap_timer -= dt;
            if sq.tap_timer <= 0.0 {
                launch(&mut sq, 1.0);
            }
        }

        // ---- gravity ----
        if !sq.on_ground {
            sq.vel_y += GRAVITY * dt;
            sq.y += sq.vel_y * dt;
            if sq.y >= ground {
                sq.y = ground;
                sq.vel_y = 0.0;
                sq.on_ground = true;
                if sq.eyes == Eyes::UpWide {
                    sq.eyes = Eyes::Forward;
                }
            }
        }

        // ---- draw ----
        let w = SIZE * (1.0 - sq.squish);
        let h = SIZE - sq.crouch;
        let body_x = if sq.squish_side > 0.0 { sq.x + SIZE - w } else { sq.x }; // stay against the wall
        let cx = body_x + w / 2.0;
        let cy = sq.y + SIZE - h / 2.0;

        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::SKYBLUE);
        d.draw_rectangle_pro(
            Rectangle::new(cx, cy, w, h),
            Vector2::new(w / 2.0, h / 2.0),
            sq.rotation,
            Color::ORANGE,
        );

        // eyes: (look offset, eye width, eye height)
        let (look_x, look_y, ew, eh) = match sq.eyes {
            Eyes::Forward => (0.0, 0.0, 2.0, 2.0),
            Eyes::Left => (-2.0, 0.0, 2.0, 2.0),
            Eyes::Right => (2.0, 0.0, 2.0, 2.0),
            Eyes::UpWide => (0.0, -1.5, 3.0, 3.0),
            Eyes::Squint => (0.0, 0.0, 3.0, 1.0),
        };
        let big = SIZE / 15.0; // how many times bigger than the old 15px square
        let (s, c) = sq.rotation.to_radians().sin_cos();
        for side in [-1.0_f32, 1.0] {
            let ox = (side * 3.0 + look_x) * big * (w / SIZE);
            let oy = (-2.0 + look_y) * big - sq.crouch * 0.3;
            let ex = cx + ox * c - oy * s; // spin the eye with the body
            let ey = cy + ox * s + oy * c;
            d.draw_rectangle_pro(
                Rectangle::new(ex, ey, ew * big, eh * big),
                Vector2::new(ew * big / 2.0, eh * big / 2.0),
                sq.rotation,
                Color::WHITE,
            );
        }
    }
}