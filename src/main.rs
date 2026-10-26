use macroquad::prelude::*;

mod game;
mod render;

use game::GameState;
use render::{Renderer, WINDOW_HEIGHT, WINDOW_TITLE, WINDOW_WIDTH};

const DAS: f32 = 0.15;
const ARR: f32 = 0.04;

fn window_conf() -> Conf {
    Conf {
        window_title: WINDOW_TITLE.to_owned(),
        window_width: WINDOW_WIDTH,
        window_height: WINDOW_HEIGHT,
        window_resizable: false,
        ..Default::default()
    }
}

/// Fires once on press, pauses for `das`, then fires every `arr` while held.
struct Repeater {
    das: f32,
    arr: f32,
    timer: f32,
    held: bool,
}

impl Repeater {
    fn new(das: f32, arr: f32) -> Self {
        Repeater { das, arr, timer: 0.0, held: false }
    }

    fn reset(&mut self) {
        self.held = false;
    }

    /// Returns how many times the action should fire this frame.
    fn tick(&mut self, down: bool, dt: f32) -> u32 {
        if !down {
            self.held = false;
            return 0;
        }
        if !self.held {
            self.held = true;
            self.timer = self.das;
            return 1;
        }
        self.timer -= dt;
        let mut fires = 0;
        while self.timer <= 0.0 {
            self.timer += self.arr;
            fires += 1;
        }
        fires
    }
}

fn new_seed() -> u64 {
    (macroquad::miniquad::date::now() * 1000.0) as u64
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut state = GameState::new(new_seed());
    let mut renderer = Renderer::new();
    let mut left = Repeater::new(DAS, ARR);
    let mut right = Repeater::new(DAS, ARR);
    let mut down = Repeater::new(0.0, ARR);
    let mut last_dir = 0;

    loop {
        let dt = get_frame_time();

        if is_key_pressed(KeyCode::H) {
            state.toggle_help();
        }

        if state.game_over {
            if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter) {
                state.reset(new_seed());
                left.reset();
                right.reset();
                down.reset();
            }
        } else {
            let l_down = is_key_down(KeyCode::Left) || is_key_down(KeyCode::A);
            let r_down = is_key_down(KeyCode::Right) || is_key_down(KeyCode::D);
            if is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::A) {
                last_dir = -1;
            }
            if is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::D) {
                last_dir = 1;
            }
            // With both held, the most recently pressed direction wins.
            let want_left = l_down && !(r_down && last_dir == 1);
            let want_right = r_down && !(l_down && last_dir == -1);

            for _ in 0..left.tick(want_left, dt) {
                state.move_horizontal(-1);
            }
            for _ in 0..right.tick(want_right, dt) {
                state.move_horizontal(1);
            }

            if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W) {
                state.rotate_cw();
            }

            let soft = is_key_down(KeyCode::Down) || is_key_down(KeyCode::S);
            for _ in 0..down.tick(soft, dt) {
                state.soft_drop();
            }

            if is_key_pressed(KeyCode::Space) {
                state.hard_drop();
            }

            state.update(dt);
        }

        renderer.update(dt);
        renderer.draw(&state);

        next_frame().await;
    }
}
