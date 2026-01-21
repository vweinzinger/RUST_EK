use rand::prelude::*;

pub const BOARD_W: i32 = 10;
pub const BOARD_H: i32 = 20;

// Cell value meanings:
// 0 = empty
// 1..=7 = a tetromino kind (also used for coloring)
pub type Cell = u8;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Step {
    /// Piece moved down by 1.
    Moved,
    /// Piece locked, optionally cleared lines, then a new piece spawned.
    Locked { cleared: u32, game_over: bool },
    /// Game was already over, no-op.
    GameOver,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Tetromino {
    I = 1,
    O = 2,
    T = 3,
    S = 4,
    Z = 5,
    J = 6,
    L = 7,
}

impl Tetromino {
    pub fn id(self) -> Cell {
        self as Cell
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Piece {
    pub kind: Tetromino,
    pub rot: u8,
    pub x: i32,
    pub y: i32,
}

// 4 rotations, each with 4 blocks, in a 4x4 local grid.
// (dx, dy) are offsets from the piece's (x, y) origin.
const SHAPES: [[[(i8, i8); 4]; 4]; 7] = [
    // I
    [
        [(0, 1), (1, 1), (2, 1), (3, 1)],
        [(2, 0), (2, 1), (2, 2), (2, 3)],
        [(0, 2), (1, 2), (2, 2), (3, 2)],
        [(1, 0), (1, 1), (1, 2), (1, 3)],
    ],
    // O
    [
        [(1, 0), (2, 0), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (2, 1)],
    ],
    // T
    [
        [(1, 0), (0, 1), (1, 1), (2, 1)],
        [(1, 0), (1, 1), (2, 1), (1, 2)],
        [(0, 1), (1, 1), (2, 1), (1, 2)],
        [(1, 0), (0, 1), (1, 1), (1, 2)],
    ],
    // S
    [
        [(1, 0), (2, 0), (0, 1), (1, 1)],
        [(1, 0), (1, 1), (2, 1), (2, 2)],
        [(1, 1), (2, 1), (0, 2), (1, 2)],
        [(0, 0), (0, 1), (1, 1), (1, 2)],
    ],
    // Z
    [
        [(0, 0), (1, 0), (1, 1), (2, 1)],
        [(2, 0), (1, 1), (2, 1), (1, 2)],
        [(0, 1), (1, 1), (1, 2), (2, 2)],
        [(1, 0), (0, 1), (1, 1), (0, 2)],
    ],
    // J
    [
        [(0, 0), (0, 1), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (1, 2)],
        [(0, 1), (1, 1), (2, 1), (2, 2)],
        [(1, 0), (1, 1), (0, 2), (1, 2)],
    ],
    // L
    [
        [(2, 0), (0, 1), (1, 1), (2, 1)],
        [(1, 0), (1, 1), (1, 2), (2, 2)],
        [(0, 1), (1, 1), (2, 1), (0, 2)],
        [(0, 0), (1, 0), (1, 1), (1, 2)],
    ],
];

fn shape_index(kind: Tetromino) -> usize {
    (kind.id() as usize) - 1
}

fn blocks_for(piece: Piece) -> [(i32, i32); 4] {
    let rot = (piece.rot % 4) as usize;
    let shape = &SHAPES[shape_index(piece.kind)][rot];
    let mut out = [(0, 0); 4];
    for (i, (dx, dy)) in shape.iter().enumerate() {
        out[i] = (piece.x + (*dx as i32), piece.y + (*dy as i32));
    }
    out
}

#[derive(Debug, Clone)]
pub struct Game {
    board: Vec<Cell>,
    current: Piece,
    next: Tetromino,
    rng: StdRng,
    score: u32,
    lines: u32,
    game_over: bool,
}

impl Game {
    pub fn new() -> Self {
        // StdRng is deterministic; seed from the OS to vary each run.
        let seed: u64 = rand::random();
        let mut g = Self {
            board: vec![0; (BOARD_W * BOARD_H) as usize],
            current: Piece {
                kind: Tetromino::I,
                rot: 0,
                x: 3,
                y: 0,
            },
            next: Tetromino::I,
            rng: StdRng::seed_from_u64(seed),
            score: 0,
            lines: 0,
            game_over: false,
        };

        g.next = g.random_piece();
        g.spawn_new_piece();
        g
    }

    pub fn reset(&mut self) {
        self.board.fill(0);
        self.score = 0;
        self.lines = 0;
        self.game_over = false;
        self.next = self.random_piece();
        self.spawn_new_piece();
    }

    pub fn board(&self) -> &[Cell] {
        &self.board
    }

    pub fn current_piece(&self) -> Piece {
        self.current
    }

    pub fn ghost_piece(&self) -> Piece {
        // Project the current piece down until it would collide.
        let mut p = self.current;
        loop {
            let mut next = p;
            next.y += 1;
            if self.is_valid(next) {
                p = next;
            } else {
                return p;
            }
        }
    }

    pub fn score(&self) -> u32 {
        self.score
    }

    pub fn lines(&self) -> u32 {
        self.lines
    }

    pub fn level(&self) -> u32 {
        (self.lines / 10) + 1
    }

    pub fn is_game_over(&self) -> bool {
        self.game_over
    }

    pub fn cell(&self, x: i32, y: i32) -> Cell {
        if x < 0 || x >= BOARD_W || y < 0 || y >= BOARD_H {
            return 0;
        }
        self.board[(y * BOARD_W + x) as usize]
    }

    pub fn tick(&mut self) -> Step {
        if self.game_over {
            return Step::GameOver;
        }

        if self.try_move(0, 1) {
            return Step::Moved;
        }

        self.lock_piece();
        let cleared = self.clear_lines();
        self.apply_score(cleared);
        self.spawn_new_piece();

        Step::Locked {
            cleared,
            game_over: self.game_over,
        }
    }

    pub fn move_left(&mut self) {
        if !self.game_over {
            self.try_move(-1, 0);
        }
    }

    pub fn move_right(&mut self) {
        if !self.game_over {
            self.try_move(1, 0);
        }
    }

    pub fn soft_drop(&mut self) -> Step {
        self.tick()
    }

    pub fn hard_drop(&mut self) -> Step {
        if self.game_over {
            return Step::GameOver;
        }

        while self.try_move(0, 1) {}
        self.lock_piece();
        let cleared = self.clear_lines();
        self.apply_score(cleared);
        self.spawn_new_piece();

        Step::Locked {
            cleared,
            game_over: self.game_over,
        }
    }

    pub fn rotate_cw(&mut self) {
        if self.game_over {
            return;
        }
        let mut rotated = self.current;
        rotated.rot = (rotated.rot + 1) % 4;

        // Small "wall kick" offsets to make rotation feel less frustrating.
        // Not full SRS, but good enough for a simple implementation.
        const KICKS: [i32; 5] = [0, -1, 1, -2, 2];
        for dx in KICKS {
            let mut candidate = rotated;
            candidate.x += dx;
            if self.is_valid(candidate) {
                self.current = candidate;
                break;
            }
        }
    }

    fn random_piece(&mut self) -> Tetromino {
        match self.rng.random_range(0..7) {
            0 => Tetromino::I,
            1 => Tetromino::O,
            2 => Tetromino::T,
            3 => Tetromino::S,
            4 => Tetromino::Z,
            5 => Tetromino::J,
            _ => Tetromino::L,
        }
    }

    fn spawn_new_piece(&mut self) {
        let kind = self.next;
        self.next = self.random_piece();

        self.current = Piece {
            kind,
            rot: 0,
            x: 3,
            y: 0,
        };

        if !self.is_valid(self.current) {
            self.game_over = true;
        }
    }

    fn try_move(&mut self, dx: i32, dy: i32) -> bool {
        let mut moved = self.current;
        moved.x += dx;
        moved.y += dy;
        if self.is_valid(moved) {
            self.current = moved;
            true
        } else {
            false
        }
    }



    fn is_valid(&self, piece: Piece) -> bool {
        for (x, y) in blocks_for(piece) {
            if x < 0 || x >= BOARD_W || y >= BOARD_H {
                return false;
            }
            if y >= 0 {
                let idx = (y * BOARD_W + x) as usize;
                if self.board[idx] != 0 {
                    return false;
                }
            }
        }
        true
    }


    fn lock_piece(&mut self) {
        let id = self.current.kind.id();
        for (x, y) in blocks_for(self.current) {
            if y < 0 {
                continue;
            }
            let idx = (y * BOARD_W + x) as usize;
            self.board[idx] = id;
        }
    }

    fn clear_lines(&mut self) -> u32 {
        let mut cleared = 0u32;
        let mut y = BOARD_H - 1;
        while y >= 0 {
            let mut full = true;
            for x in 0..BOARD_W {
                if self.board[(y * BOARD_W + x) as usize] == 0 {
                    full = false;
                    break;
                }
            }

            if full {
                cleared += 1;
                // Move all rows [0..y) down by one.
                for yy in (1..=y).rev() {
                    for x in 0..BOARD_W {
                        let from = ((yy - 1) * BOARD_W + x) as usize;
                        let to = (yy * BOARD_W + x) as usize;
                        self.board[to] = self.board[from];
                    }
                }
                // Clear top row.
                for x in 0..BOARD_W {
                    self.board[x as usize] = 0;
                }
                // Stay on same y to check the shifted row.
            } else {
                y -= 1;
            }
        }

        self.lines += cleared;
        cleared
    }

    fn apply_score(&mut self, cleared: u32) {
        let lvl = self.level();
        let add = match cleared {
            1 => 40,
            2 => 100,
            3 => 300,
            4 => 1200,
            _ => 0,
        };
        self.score = self.score.saturating_add(add * lvl);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_game_has_empty_board() {
        let g = Game::new();
        assert_eq!(g.board().len(), (BOARD_W * BOARD_H) as usize);
        assert!(g.board().iter().any(|&c| c == 0));
    }

    #[test]
    fn piece_blocks_in_bounds_on_spawn() {
        let g = Game::new();
        for (x, y) in blocks_for(g.current_piece()) {
            assert!(x >= 0 && x < BOARD_W);
            assert!(y >= 0 && y < BOARD_H);
        }
    }
}
