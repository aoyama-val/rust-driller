use rand::prelude::*;
use std::{collections::HashMap, time};

pub const CELL_SIZE: i32 = 40;
pub const SCREEN_WIDTH: i32 = CELL_SIZE * CELLS_X_LEN;
pub const SCREEN_HEIGHT: i32 = CELL_SIZE * 12;
pub const CELLS_X_LEN: i32 = 9;
pub const CELLS_X_MIN: i32 = 0;
pub const CELLS_X_MAX: i32 = CELLS_X_LEN - 1;
pub const CELLS_Y_LEN: i32 = 30;
pub const CELLS_Y_MIN: i32 = 0;
pub const CELLS_Y_MAX: i32 = CELLS_Y_LEN - 1;
pub const AIR_MAX: i32 = 100;
pub const WALK_FRAMES: i32 = 3;
pub const FALL_FRAMES: i32 = 15;

pub enum Command {
    None,
    Left,
    Right,
    Dig,
}

#[derive(Clone, Eq, PartialEq)]
pub enum Direction {
    Left,
    Right,
    Down,
    Up,
}

impl Direction {
    pub fn opposite(&self) -> Direction {
        match self {
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
            Direction::Down => Direction::Up,
            Direction::Up => Direction::Down,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum CellType {
    None,
    Air,
    Box,
    Red,
    Yellow,
    Green,
    Blue,
}

#[derive(Clone, Copy)]
pub enum CellState {
    Normal,
    Connected,
}

#[derive(Clone, Copy)]
pub struct Cell {
    pub cell_type: CellType,
}

impl Cell {
    fn new() -> Self {
        Cell {
            cell_type: CellType::None,
        }
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Self {
        Point {
            x: (CELLS_X_LEN + x) % CELLS_X_LEN,
            y: (CELLS_Y_LEN + y) % CELLS_Y_LEN,
        }
    }

    pub fn neighbor(&self, direction: Direction) -> Self {
        match direction {
            Direction::Left => Self::new(self.x - 1, self.y),
            Direction::Right => Self::new(self.x + 1, self.y),
            Direction::Up => Self::new(self.x, self.y - 1),
            Direction::Down => Self::new(self.x, self.y + 1),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum PlayerState {
    Standing,
    Walking,
    Falling,
}

pub struct Player {
    pub p: Point,
    pub air: i32,
    pub state: PlayerState,
    pub direction: Direction,
    pub walking_frames: i32,
    pub falling_frames: i32,
}

impl Player {
    pub fn new() -> Self {
        let player = Player {
            p: Point::new(CELLS_X_LEN / 2, 5),
            air: AIR_MAX,
            direction: Direction::Left,
            walking_frames: 0,
            falling_frames: 0,
            state: PlayerState::Standing,
        };
        player
    }

    pub fn do_move(&mut self) {
        self.air -= 1;
    }
}

pub struct Game {
    pub rng: StdRng,
    pub is_over: bool,
    pub frame: i32,
    pub player: Player,
    pub score: i32,
    pub requested_sounds: Vec<&'static str>,
    pub cells: [[Cell; CELLS_X_LEN as usize]; CELLS_Y_LEN as usize],
    pub camera_y: i32,
}

impl Game {
    pub fn new() -> Self {
        let now = time::SystemTime::now();
        let timestamp = now
            .duration_since(time::UNIX_EPOCH)
            .expect("SystemTime before UNIX EPOCH!")
            .as_secs();
        let rng = StdRng::seed_from_u64(timestamp);

        let mut game = Game {
            rng: rng,
            is_over: false,
            frame: 0,
            player: Player::new(),
            score: 0,
            requested_sounds: Vec::new(),
            cells: [[Cell::new(); CELLS_X_LEN as usize]; CELLS_Y_LEN as usize],
            camera_y: 0,
        };

        game.cells[0][0] = Cell {
            cell_type: CellType::Red,
        };
        game.cells[1][0] = Cell {
            cell_type: CellType::Yellow,
        };
        game.cells[2][0] = Cell {
            cell_type: CellType::Green,
        };
        game.cells[3][0] = Cell {
            cell_type: CellType::Blue,
        };
        game.cells[4][0] = Cell {
            cell_type: CellType::Box,
        };

        for x in CELLS_X_MIN..=CELLS_X_MAX {
            game.cells[6][x as usize].cell_type = CellType::Red;
        }

        for y in 0..7 {
            for x in CELLS_X_MIN..=CELLS_X_MAX {
                game.cells[CELLS_Y_MAX as usize - y as usize][x as usize].cell_type = CellType::Box;
            }
        }

        game
    }

    pub fn update(&mut self, command: Command) {
        if self.is_over {
            return;
        }

        println!("{:?}", self.player.state);
        if self.cells[self.player.p.y as usize + 1][self.player.p.x as usize].cell_type
            == CellType::None
            && self.player.state != PlayerState::Falling
        {
            self.player.state = PlayerState::Falling;

            self.player.falling_frames = 0;
        }

        if self.player.state == PlayerState::Falling {
            self.player.falling_frames += 1;
            if self.player.falling_frames >= FALL_FRAMES {
                self.player.falling_frames = 0;
                self.player.p.y += 1;
                self.player.state = PlayerState::Standing;
            }
        }

        match command {
            Command::None => {}
            Command::Left => {
                if self.player.p.x > CELLS_X_MIN {
                    if self.player.state == PlayerState::Standing {
                        self.player.state = PlayerState::Walking;
                        self.player.direction = Direction::Left;
                        self.player.walking_frames = 0;
                    }
                }
            }
            Command::Right => {
                if self.player.p.x < CELLS_X_MAX {
                    if self.player.state == PlayerState::Standing {
                        self.player.state = PlayerState::Walking;
                        self.player.direction = Direction::Right;
                        self.player.walking_frames = 0;
                    }
                }
            }
            Command::Dig => {
                self.cells[self.player.p.y as usize + 1][self.player.p.x as usize].cell_type =
                    CellType::None;
            }
        }

        if self.player.state == PlayerState::Walking {
            self.player.walking_frames += 1;
            if self.player.walking_frames >= WALK_FRAMES {
                if self.player.direction == Direction::Left {
                    self.player.p.x -= 1;
                } else {
                    self.player.p.x += 1;
                }
                self.player.state = PlayerState::Standing;
            }
        }

        if self.player.air <= 0 {
            self.is_over = true;
            self.requested_sounds.push("crash.wav");
        }

        self.camera_y = self.player.p.y - 5;
        if self.camera_y > CELLS_Y_LEN - 7 {
            self.camera_y > CELLS_Y_LEN - 7;
        }

        self.frame += 1;
        self.score = self.frame / 30;
    }
}

fn clamp<T: PartialOrd>(min: T, value: T, max: T) -> T {
    if value < min {
        return min;
    }
    if value > max {
        return max;
    }
    value
}
