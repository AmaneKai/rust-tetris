pub const BOARD_W: usize = 10;
pub const BOARD_H: usize = 20;

const FULL_ROW: u16 = 0x03FF;

pub const LOCK_DELAY: f32 = 0.5;
const MAX_LOCK_RESETS: u8 = 15;

const INITIAL_DROP_SPEED: f32 = 0.8;
const MIN_DROP_SPEED: f32 = 0.1;
const SPEED_STEP_PER_LEVEL: f32 = 0.07;
const LINES_PER_LEVEL: u32 = 10;
const LINE_SCORES: [u32; 5] = [0, 40, 100, 300, 1200];

const SPAWN_X: i32 = 3;
const SPAWN_Y: i32 = -1;

/// Rotates a 4x4 mask clockwise inside its top-left `n`x`n` box
/// Bit for cell (r, c) is `r * 4 + (3 - c)`, so a row nibble reads left to right
const fn rotate_cw(mask: u16, n: usize) -> u16 {
    let mut out = 0u16;
    let mut r = 0;

    while r < n {
        let mut c = 0;

        while c < n {
            if (mask >> (r * 4 + (3 - c))) & 1 == 1 {
                out |= 1 << (c * 4 + (3 - (n - 1 - r)));
            }
            c += 1;
        }

        r += 1;
    }

    out
}

/// Packs all four rotation states into one `u64` (16 bits each)
/// `n == 0` marks a piece that does not rotate
const fn pack(base: u16, n: usize) -> u64 {
    let mut out = 0u64;
    let mut mask = base;
    let mut rot = 0;
    while rot < 4 {
        out |= (mask as u64) << (rot * 16);
        if n > 0 {
            mask = rotate_cw(mask, n);
        }
        rot += 1;
    }
    out
}

const fn rows(r0: u16, r1: u16, r2: u16, r3: u16) -> u16 {
    r0 | (r1 << 4) | (r2 << 8) | (r3 << 12)
}

/// Order: I, O, T, S, Z, J, L
pub const PIECES: [u64; 7] = [
    pack(rows(0b0000, 0b1111, 0b0000, 0b0000), 4),
    pack(rows(0b0110, 0b0110, 0b0000, 0b0000), 0),
    pack(rows(0b0100, 0b1110, 0b0000, 0b0000), 3),
    pack(rows(0b0110, 0b1100, 0b0000, 0b0000), 3),
    pack(rows(0b1100, 0b0110, 0b0000, 0b0000), 3),
    pack(rows(0b1000, 0b1110, 0b0000, 0b0000), 3),
    pack(rows(0b0010, 0b1110, 0b0000, 0b0000), 3),
];

#[inline]
pub fn piece_mask(id: u8, rot: u8) -> u16 {
    (PIECES[id as usize] >> ((rot & 3) as u32 * 16)) as u16
}

/// One `u16` per row; column `x` lives in bit `9 - x`
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Board {
    pub rows: [u16; BOARD_H],
}

impl Board {
    pub const fn new() -> Self {
        Board { rows: [0; BOARD_H] }
    }

    /// True if `mask` placed with its top-left at (x, y) hits a wall, the floor or a block
    /// Cells above the board (y < 0) are free but still bound by the side walls
    pub fn collides(&self, mask: u16, x: i32, y: i32) -> bool {
        for r in 0..4i32 {
            let nibble = ((mask >> (r * 4)) & 0xF) as u32;

            if nibble == 0 {
                continue;
            }
            if !(-3..=9).contains(&x) {
                return true;
            }
            // Aligned with `board_row << 3`; bits outside 3..=12 are past a wall.
            let bits = nibble << (9 - x);
            if bits & !((FULL_ROW as u32) << 3) != 0 {
                return true;
            }

            let by = y + r;
            if by >= BOARD_H as i32 {
                return true;
            }
            if by >= 0 && ((self.rows[by as usize] as u32) << 3) & bits != 0 {
                return true;
            }
        }

        false
    }

    /// ORs `mask` onto the board. Cells above the board are dropped.
    pub fn place(&mut self, mask: u16, x: i32, y: i32) {
        for r in 0..4i32 {
            let nibble = ((mask >> (r * 4)) & 0xF) as u32;
            let by = y + r;

            if nibble != 0 && (0..BOARD_H as i32).contains(&by) {
                self.rows[by as usize] |= ((nibble << (9 - x)) >> 3) as u16;
            }
        }
    }

    /// Removes full rows in a single bottom-up pass and returns how many were removed.
    pub fn clear_lines(&mut self) -> u32 {
        let mut write = BOARD_H;
        for read in (0..BOARD_H).rev() {
            if self.rows[read] != FULL_ROW {
                write -= 1;
                self.rows[write] = self.rows[read];
            }
        }

        let cleared = write;
        for row in &mut self.rows[..write] {
            *row = 0;
        }

        cleared as u32
    }

}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

/// Self-contained 7-bag with an xorshift64 generator
#[derive(Clone, Debug)]
pub struct Bag {
    order: [u8; 7],
    idx: usize,
    rng: u64,
}

impl Bag {
    pub fn new(seed: u64) -> Self {
        let mut bag = Bag {
            order: [0, 1, 2, 3, 4, 5, 6],
            idx: 7,
            rng: seed | 1,
        };
        bag.refill();
        bag
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng = x;
        x
    }

    fn refill(&mut self) {
        for i in (1..7).rev() {
            let j = (self.next_u64() % (i as u64 + 1)) as usize;
            self.order.swap(i, j);
        }
        self.idx = 0;
    }

    pub fn next(&mut self) -> u8 {
        if self.idx == 7 {
            self.refill();
        }
        let id = self.order[self.idx];
        self.idx += 1;
        id
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Piece {
    pub id: u8,
    pub rot: u8,
    pub x: i32,
    pub y: i32,
}

impl Piece {
    fn spawn(id: u8) -> Self {
        Piece { id, rot: 0, x: SPAWN_X, y: SPAWN_Y }
    }

    #[inline]
    pub fn mask(&self) -> u16 {
        piece_mask(self.id, self.rot)
    }
}

#[derive(Clone, Debug)]
pub struct GameState {
    pub board: Board,
    pub piece: Piece,
    pub next: u8,
    pub score: u32,
    pub lines: u32,
    pub level: u32,
    pub game_over: bool,
    pub show_help: bool,
    bag: Bag,
    gravity_timer: f32,
    lock_timer: f32,
    lock_resets: u8,
}

impl GameState {
    pub fn new(seed: u64) -> Self {
        let mut bag = Bag::new(seed);
        let first = bag.next();
        let next = bag.next();
        GameState {
            board: Board::new(),
            piece: Piece::spawn(first),
            next,
            score: 0,
            lines: 0,
            level: 1,
            game_over: false,
            show_help: true,
            bag,
            gravity_timer: 0.0,
            lock_timer: 0.0,
            lock_resets: 0,
        }
    }

    pub fn reset(&mut self, seed: u64) {
        *self = GameState::new(seed);
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub fn drop_interval(&self) -> f32 {
        (INITIAL_DROP_SPEED - (self.level - 1) as f32 * SPEED_STEP_PER_LEVEL).max(MIN_DROP_SPEED)
    }

    fn fits(&self, piece: &Piece) -> bool {
        !self.board.collides(piece.mask(), piece.x, piece.y)
    }

    pub fn is_grounded(&self) -> bool {
        self.board.collides(self.piece.mask(), self.piece.x, self.piece.y + 1)
    }

    /// Advances gravity and lock delay by `dt` seconds.
    pub fn update(&mut self, dt: f32) {
        if self.game_over {
            return;
        }

        if self.is_grounded() {
            self.lock_timer += dt;
            if self.lock_timer >= LOCK_DELAY {
                self.lock_piece();
            }
            return;
        }

        self.lock_timer = 0.0;
        self.gravity_timer += dt;
        let interval = self.drop_interval();
        while self.gravity_timer >= interval {
            self.gravity_timer -= interval;
            self.piece.y += 1;
            if self.is_grounded() {
                self.gravity_timer = 0.0;
                break;
            }
        }
    }

    /// Moving or rotating while grounded buys more lock time, up to a cap.
    fn on_piece_moved(&mut self) {
        if self.is_grounded() && self.lock_resets < MAX_LOCK_RESETS {
            self.lock_resets += 1;
            self.lock_timer = 0.0;
        }
    }

    pub fn move_horizontal(&mut self, dx: i32) -> bool {
        if self.game_over {
            return false;
        }
        let moved = Piece { x: self.piece.x + dx, ..self.piece };
        if !self.fits(&moved) {
            return false;
        }
        self.piece = moved;
        self.on_piece_moved();
        true
    }

    /// Moves down one row. Returns false if the piece is already resting.
    pub fn soft_drop(&mut self) -> bool {
        if self.game_over {
            return false;
        }

        let moved = Piece { y: self.piece.y + 1, ..self.piece };
        if !self.fits(&moved) {
            return false;
        }

        self.piece = moved;
        self.gravity_timer = 0.0;

        true
    }

    /// Clockwise rotation with simple horizontal wall kicks.
    pub fn rotate_cw(&mut self) -> bool {
        if self.game_over {
            return false;
        }

        let rot = (self.piece.rot + 1) & 3;
        for kick in [0, -1, 1, -2, 2] {
            let rotated = Piece { rot, x: self.piece.x + kick, ..self.piece };
            if self.fits(&rotated) {
                self.piece = rotated;
                self.on_piece_moved();
                return true;
            }
        }

        false
    }

    pub fn hard_drop(&mut self) {
        if self.game_over {
            return;
        }

        self.piece.y = self.ghost_y();
        self.lock_piece();
    }

    /// Landing row of the current piece.
    pub fn ghost_y(&self) -> i32 {
        let mask = self.piece.mask();
        let mut y = self.piece.y;

        while !self.board.collides(mask, self.piece.x, y + 1) {
            y += 1;
        }

        y
    }

    fn lock_piece(&mut self) {
        let mask = self.piece.mask();
        let (x, y) = (self.piece.x, self.piece.y);

        // Any block still above the visible board means the stack topped out.
        let topped_out = (0..4).any(|r| (mask >> (r * 4)) & 0xF != 0 && y + (r as i32) < 0);

        self.board.place(mask, x, y);
        if topped_out {
            self.game_over = true;
            return;
        }

        let cleared = self.board.clear_lines();
        if cleared > 0 {
            self.score += LINE_SCORES[cleared as usize] * self.level;
            self.lines += cleared;
            self.level = self.lines / LINES_PER_LEVEL + 1;
        }

        self.spawn_next();
    }

    fn spawn_next(&mut self) {
        self.piece = Piece::spawn(self.next);
        self.next = self.bag.next();
        self.gravity_timer = 0.0;
        self.lock_timer = 0.0;
        self.lock_resets = 0;

        if !self.fits(&self.piece) {
            self.game_over = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cells(mask: u16) -> u32 {
        mask.count_ones()
    }

    #[test]
    fn every_rotation_has_four_cells() {
        for id in 0..7 {
            for rot in 0..4 {
                assert_eq!(cells(piece_mask(id, rot)), 4, "piece {id} rot {rot}");
            }
        }
    }

    #[test]
    fn rotations_are_distinct_where_expected() {
        // I has 2 unique states, O 1, the rest 4.
        let unique = |id: u8| {
            let mut seen = [0u16; 4];
            let mut n = 0;
            for rot in 0..4 {
                let m = piece_mask(id, rot);
                if !seen[..n].contains(&m) {
                    seen[n] = m;
                    n += 1;
                }
            }
            n
        };
        assert_eq!(unique(0), 4);
        assert_eq!(unique(1), 1);
        for id in 2..7 {
            assert_eq!(unique(id), 4);
        }
    }

    #[test]
    fn rotating_four_times_returns_to_start() {
        for id in 0..7 {
            assert_eq!(piece_mask(id, 0), piece_mask(id, 4));
        }
    }

    #[test]
    fn walls_and_floor_collide() {
        let b = Board::new();
        let i = piece_mask(0, 0); // horizontal I on row 1 of its box
        assert!(!b.collides(i, 0, 0));
        assert!(!b.collides(i, 6, 0));
        assert!(b.collides(i, -1, 0));
        assert!(b.collides(i, 7, 0));
        assert!(b.collides(i, 3, 19)); // row 1 would land on y = 20
        assert!(!b.collides(i, 3, 18));
    }

    #[test]
    fn vertical_piece_touches_edges() {
        let b = Board::new();
        let i_vert = piece_mask(0, 1); // column 2 of its box
        assert!(!b.collides(i_vert, -2, 0));
        assert!(b.collides(i_vert, -3, 0));
        assert!(!b.collides(i_vert, 7, 0));
        assert!(b.collides(i_vert, 8, 0));
    }

    #[test]
    fn cells_above_board_are_free() {
        let b = Board::new();
        assert!(!b.collides(piece_mask(0, 1), 3, -3));
    }

    #[test]
    fn place_then_collide() {
        let mut b = Board::new();
        let o = piece_mask(1, 0);
        b.place(o, 4, 18);
        // O occupies box columns 1..=2, i.e. board columns 5 and 6 at x = 4.
        assert_eq!(b.rows[18], 0b0000011000);
        assert_eq!(b.rows[19], 0b0000011000);
        assert!(b.collides(o, 4, 18));
        assert!(b.collides(o, 5, 17));
        assert!(!b.collides(o, 2, 18));
    }

    #[test]
    fn clear_lines_compacts_rows() {
        let mut b = Board::new();
        b.rows[19] = FULL_ROW;
        b.rows[18] = 0b1010101010;
        b.rows[17] = FULL_ROW;
        b.rows[16] = 0b0000000001;
        assert_eq!(b.clear_lines(), 2);
        assert_eq!(b.rows[19], 0b1010101010);
        assert_eq!(b.rows[18], 0b0000000001);
        assert!(b.rows[..18].iter().all(|&r| r == 0));
    }

    #[test]
    fn clear_lines_with_nothing_full() {
        let mut b = Board::new();
        b.rows[19] = 0x3FE;
        assert_eq!(b.clear_lines(), 0);
        assert_eq!(b.rows[19], 0x3FE);
    }

    #[test]
    fn bag_deals_each_piece_once_per_seven() {
        let mut bag = Bag::new(12345);
        for _ in 0..20 {
            let mut seen = [false; 7];
            for _ in 0..7 {
                let id = bag.next() as usize;
                assert!(!seen[id]);
                seen[id] = true;
            }
        }
    }

    #[test]
    fn bag_is_deterministic_and_seed_dependent() {
        let draw = |seed| {
            let mut b = Bag::new(seed);
            [b.next(), b.next(), b.next(), b.next(), b.next(), b.next(), b.next()]
        };
        assert_eq!(draw(7), draw(7));
        assert!((1..50).any(|s| draw(s) != draw(7)));
        let _ = Bag::new(0).next(); // zero seed must not lock the generator
    }

    #[test]
    fn ghost_lands_on_floor() {
        let g = GameState::new(1);
        let y = g.ghost_y();
        assert!(g.board.collides(g.piece.mask(), g.piece.x, y + 1));
        assert!(!g.board.collides(g.piece.mask(), g.piece.x, y));
    }

    #[test]
    fn lock_delay_holds_then_locks() {
        let mut g = GameState::new(1);
        g.piece.y = g.ghost_y();
        let first = g.piece;
        g.update(LOCK_DELAY * 0.5);
        assert_eq!(g.piece, first);
        assert!(g.board.rows.iter().all(|&r| r == 0));
        g.update(LOCK_DELAY * 0.6);
        assert!(g.board.rows.iter().any(|&r| r != 0));
    }

    #[test]
    fn sliding_while_grounded_resets_lock_timer() {
        let mut g = GameState::new(1);
        g.piece = Piece { id: 1, rot: 0, x: 3, y: 0 };
        g.piece.y = g.ghost_y();
        g.update(LOCK_DELAY * 0.8);
        assert!(g.move_horizontal(1));
        g.update(LOCK_DELAY * 0.8);
        assert!(g.board.rows.iter().all(|&r| r == 0), "should not have locked yet");
    }

    #[test]
    fn lock_resets_are_capped() {
        let mut g = GameState::new(1);
        g.piece = Piece { id: 1, rot: 0, x: 3, y: 0 };
        g.piece.y = g.ghost_y();
        for i in 0..40 {
            g.move_horizontal(if i % 2 == 0 { 1 } else { -1 });
        }
        g.update(LOCK_DELAY + 0.01);
        assert!(g.board.rows.iter().any(|&r| r != 0));
    }

    #[test]
    fn hard_drop_scores_line_clear() {
        let mut g = GameState::new(1);
        // Bottom row full except columns 4 and 5; drop an O into the gap.
        g.board.rows[19] = FULL_ROW & !0b0000110000;
        g.board.rows[18] = FULL_ROW & !0b0000110000;
        g.piece = Piece { id: 1, rot: 0, x: 3, y: 0 };
        g.hard_drop();
        assert_eq!(g.lines, 2);
        assert_eq!(g.score, 100);
        assert!(!g.game_over);
        assert!(g.board.rows.iter().all(|&r| r == 0));
    }

    #[test]
    fn level_advances_every_ten_lines() {
        let mut g = GameState::new(1);
        g.lines = 9;
        g.board.rows[19] = FULL_ROW & !0b0000110000;
        g.piece = Piece { id: 1, rot: 0, x: 3, y: 0 };
        g.hard_drop();
        assert_eq!(g.lines, 10);
        assert_eq!(g.level, 2);
        assert!(g.drop_interval() < INITIAL_DROP_SPEED);
    }

    #[test]
    fn score_is_multiplied_by_level() {
        let mut g = GameState::new(1);
        g.level = 3;
        g.lines = 20;
        g.board.rows[19] = FULL_ROW & !0b0000110000;
        g.piece = Piece { id: 1, rot: 0, x: 3, y: 0 };
        g.hard_drop();
        assert_eq!(g.score, 40 * 3);
    }

    #[test]
    fn blocked_spawn_is_game_over() {
        let mut g = GameState::new(1);
        // Block the spawn area (columns 3..=6 of the top row) for every piece.
        g.board.rows[0] = 0b0001111000;
        g.piece = Piece { id: 1, rot: 0, x: 0, y: 10 };
        g.hard_drop();
        assert!(g.game_over);
    }

    #[test]
    fn locking_above_the_board_is_game_over() {
        let mut g = GameState::new(1);
        for r in 0..BOARD_H {
            g.board.rows[r] = 0b1000000000;
        }
        // Vertical I with its top cell off-screen, resting on the stack.
        g.piece = Piece { id: 0, rot: 1, x: 3, y: -2 };
        g.piece.y = g.ghost_y();
        g.hard_drop();
        assert!(!g.game_over, "free column, piece lands fully on board");

        let mut g = GameState::new(1);
        g.board.rows[0] = 0b0000100000; // fills column 5, forcing a lock at y = -2
        g.piece = Piece { id: 0, rot: 1, x: 3, y: -2 };
        g.hard_drop();
        assert!(g.game_over);
    }

    #[test]
    fn rotation_kicks_off_the_wall() {
        let mut g = GameState::new(1);
        g.piece = Piece { id: 0, rot: 1, x: -2, y: 5 }; // vertical I against left wall
        assert!(g.rotate_cw());
        assert!(!g.board.collides(g.piece.mask(), g.piece.x, g.piece.y));
        assert_eq!(g.piece.rot, 2);
    }

    #[test]
    fn rotation_fails_when_boxed_in() {
        let mut g = GameState::new(1);
        g.piece = Piece { id: 0, rot: 1, x: 3, y: 10 };
        for r in 0..BOARD_H {
            g.board.rows[r] = FULL_ROW & !(1 << (9 - 5));
        }
        assert!(!g.rotate_cw());
        assert_eq!(g.piece.rot, 1);
    }

    #[test]
    fn gravity_pulls_piece_down() {
        let mut g = GameState::new(1);
        let y0 = g.piece.y;
        g.update(g.drop_interval() + 0.001);
        assert_eq!(g.piece.y, y0 + 1);
    }

    #[test]
    fn game_over_freezes_state() {
        let mut g = GameState::new(1);
        g.game_over = true;
        let p = g.piece;
        g.update(5.0);
        assert!(!g.move_horizontal(1) && !g.rotate_cw() && !g.soft_drop());
        g.hard_drop();
        assert_eq!(g.piece, p);
    }
}
