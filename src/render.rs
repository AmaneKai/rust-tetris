//! All drawing: colors, layout constants and the Game Boy look.

use crate::game::{piece_mask, GameState, BOARD_H, BOARD_W};
use macroquad::prelude::*;

pub const WINDOW_TITLE: &str = "Game Boy Tetris";
pub const WINDOW_WIDTH: i32 = 480;
pub const WINDOW_HEIGHT: i32 = 640;

const BLOCK: f32 = 24.0;
const BOARD_X: f32 = 60.0;
const BOARD_Y: f32 = 30.0; // board spans y = 30..510
const BOARD_PX_W: f32 = BOARD_W as f32 * BLOCK;
const BOARD_PX_H: f32 = BOARD_H as f32 * BLOCK;

const HELP_X: f32 = 40.0;
const HELP_Y: f32 = 525.0;
const HELP_W: f32 = 400.0;
const HELP_H: f32 = 100.0; // bottom edge at y = 625, inside the 640px window

const GB_DARK: Color = Color::new(0.06, 0.22, 0.06, 1.0);
const GB_MED_DARK: Color = Color::new(0.19, 0.38, 0.19, 1.0);
const GB_MED_LIGHT: Color = Color::new(0.55, 0.67, 0.06, 1.0);
const GB_LIGHT: Color = Color::new(0.68, 0.89, 0.18, 1.0);

pub struct Renderer {
    blink_timer: f32,
}

impl Renderer {
    pub fn new() -> Self {
        Renderer { blink_timer: 0.0 }
    }

    pub fn update(&mut self, dt: f32) {
        self.blink_timer += dt;
    }

    fn blink(&self, rate: f32) -> bool {
        (self.blink_timer * rate) as i32 % 2 == 0
    }

    pub fn draw(&self, state: &GameState) {
        clear_background(GB_LIGHT);

        draw_frame(BOARD_X, BOARD_Y, BOARD_PX_W, BOARD_PX_H, 8.0);
        draw_grid();
        draw_stack(state);
        draw_ghost(state);
        draw_active(state);
        self.draw_sidebar(state);

        if state.show_help {
            self.draw_help();
        }
        if state.game_over {
            self.draw_game_over();
        }
    }

    fn draw_sidebar(&self, state: &GameState) {
        let x = BOARD_X + BOARD_PX_W + 30.0;
        let mut y = BOARD_Y + 10.0;

        draw_text("TETRIS", x, y, 28.0, GB_DARK);
        y += 35.0;

        draw_next_box(state.next, x, y);
        y += 120.0;

        for (label, value) in [
            ("SCORE", format!("{:06}", state.score)),
            ("LINES", format!("{:03}", state.lines)),
            ("LEVEL", format!("{:02}", state.level)),
        ] {
            draw_text(label, x, y, 18.0, GB_DARK);
            y += 20.0;
            draw_text(&value, x, y, 22.0, GB_DARK);
            y += 32.0;
        }

        if self.blink(2.0) {
            draw_text("PRESS H", x, y, 16.0, GB_MED_DARK);
            draw_text("FOR HELP", x, y + 14.0, 16.0, GB_MED_DARK);
        }
    }

    fn draw_help(&self) {
        draw_frame(HELP_X, HELP_Y, HELP_W, HELP_H, 4.0);

        let x = HELP_X + 12.0;
        let mut y = HELP_Y + 18.0;
        draw_text("CONTROLS", x, y, 18.0, GB_DARK);
        y += 17.0;
        for line in [
            "A/D, LEFT/RIGHT  Move (hold to repeat)",
            "W/UP             Rotate",
            "S/DOWN           Soft drop",
            "SPACE            Hard drop",
        ] {
            draw_text(line, x, y, 16.0, GB_DARK);
            y += 15.0;
        }
        if self.blink(3.0) {
            draw_text("H: close", HELP_X + HELP_W - 80.0, HELP_Y + 18.0, 16.0, GB_MED_DARK);
        }
    }

    fn draw_game_over(&self) {
        draw_rectangle(
            0.0,
            0.0,
            screen_width(),
            screen_height(),
            Color::new(GB_DARK.r, GB_DARK.g, GB_DARK.b, 0.8),
        );

        let (w, h) = (240.0, 100.0);
        let x = (screen_width() - w) / 2.0;
        let y = (screen_height() - h) / 2.0;
        draw_frame(x, y, w, h, 4.0);

        draw_text("GAME OVER", x + 60.0, y + 40.0, 28.0, GB_DARK);
        if self.blink(2.0) {
            draw_text("SPACE TO RESTART", x + 36.0, y + 70.0, 20.0, GB_MED_DARK);
        }
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Three-layer bezel around a light panel of the given inner size.
fn draw_frame(x: f32, y: f32, w: f32, h: f32, thickness: f32) {
    draw_rectangle(x - thickness, y - thickness, w + thickness * 2.0, h + thickness * 2.0, GB_DARK);
    let t = thickness - 2.0;
    draw_rectangle(x - t, y - t, w + t * 2.0, h + t * 2.0, GB_MED_DARK);
    let t = (thickness / 2.0 - 1.0).max(1.0);
    draw_rectangle(x - t, y - t, w + t * 2.0, h + t * 2.0, GB_MED_LIGHT);
    draw_rectangle(x, y, w, h, GB_LIGHT);
}

fn draw_grid() {
    for i in 0..=BOARD_W {
        let px = BOARD_X + i as f32 * BLOCK;
        draw_line(px, BOARD_Y, px, BOARD_Y + BOARD_PX_H, 1.0, GB_MED_LIGHT);
    }
    for i in 0..=BOARD_H {
        let py = BOARD_Y + i as f32 * BLOCK;
        draw_line(BOARD_X, py, BOARD_X + BOARD_PX_W, py, 1.0, GB_MED_LIGHT);
    }
}

fn draw_stack(state: &GameState) {
    for (y, &row) in state.board.rows.iter().enumerate() {
        if row == 0 {
            continue;
        }
        for x in 0..BOARD_W {
            if (row >> (BOARD_W - 1 - x)) & 1 == 1 {
                draw_block(BOARD_X + x as f32 * BLOCK, BOARD_Y + y as f32 * BLOCK, BLOCK);
            }
        }
    }
}

/// Calls `f(col, row)` for each cell of a 4x4 mask, in board coordinates when offset.
fn for_cells(mask: u16, mut f: impl FnMut(i32, i32)) {
    for r in 0..4 {
        for c in 0..4 {
            if (mask >> (r * 4 + (3 - c))) & 1 == 1 {
                f(c, r);
            }
        }
    }
}

fn draw_active(state: &GameState) {
    let p = state.piece;
    for_cells(p.mask(), |c, r| {
        let (x, y) = (p.x + c, p.y + r);
        if y >= 0 {
            draw_block(BOARD_X + x as f32 * BLOCK, BOARD_Y + y as f32 * BLOCK, BLOCK);
        }
    });
}

fn draw_ghost(state: &GameState) {
    let p = state.piece;
    let gy = state.ghost_y();
    for_cells(p.mask(), |c, r| {
        let (x, y) = (p.x + c, gy + r);
        if y >= 0 {
            let px = BOARD_X + x as f32 * BLOCK + 1.0;
            let py = BOARD_Y + y as f32 * BLOCK + 1.0;
            let s = BLOCK - 2.0;
            draw_rectangle_lines(px, py, s, s, 1.0, GB_MED_DARK);
            draw_rectangle_lines(px + 2.0, py + 2.0, s - 4.0, s - 4.0, 1.0, GB_MED_DARK);
        }
    });
}

/// Game Boy relief block with a size-independent bevel.
fn draw_block(x: f32, y: f32, size: f32) {
    let (x, y, s) = (x + 1.0, y + 1.0, size - 2.0);
    draw_rectangle(x, y, s, s, GB_DARK);
    draw_rectangle(x + 1.0, y + 1.0, s - 3.0, s - 3.0, GB_MED_DARK);
    draw_rectangle(x + 3.0, y + 3.0, s - 7.0, s - 7.0, GB_MED_LIGHT);
}

fn draw_next_box(id: u8, x: f32, y: f32) {
    const BOX: f32 = 80.0;
    const CELL: f32 = 16.0;
    draw_text("NEXT", x, y - 8.0, 18.0, GB_DARK);
    draw_frame(x, y, BOX, BOX, 4.0);

    let mask = piece_mask(id, 0);
    let (mut min_c, mut max_c, mut min_r, mut max_r) = (3, 0, 3, 0);
    for_cells(mask, |c, r| {
        min_c = min_c.min(c);
        max_c = max_c.max(c);
        min_r = min_r.min(r);
        max_r = max_r.max(r);
    });

    // Center the occupied cells (not the 4x4 box) inside the preview.
    let w = (max_c - min_c + 1) as f32 * CELL;
    let h = (max_r - min_r + 1) as f32 * CELL;
    let ox = x + (BOX - w) / 2.0 - min_c as f32 * CELL;
    let oy = y + (BOX - h) / 2.0 - min_r as f32 * CELL;
    for_cells(mask, |c, r| draw_block(ox + c as f32 * CELL, oy + r as f32 * CELL, CELL));
}
