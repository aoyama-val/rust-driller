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
pub const FALL_FRAMES: i32 = 3;

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

impl CellType {
    fn from_u32(n: u32) -> Self {
        match n % 4 {
            0 => CellType::Red,
            1 => CellType::Yellow,
            2 => CellType::Green,
            3 => CellType::Blue,
            _ => panic!(),
        }
    }
}

#[derive(Clone, Copy)]
pub enum CellState {
    Normal,
    Connected,
}

#[derive(Clone, Copy)]
pub struct Cell {
    pub cell_type: CellType,
    pub leader: Option<Point>,
}

impl Cell {
    fn new() -> Self {
        Cell {
            cell_type: CellType::None,
            leader: None,
        }
    }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
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
        // let rng = StdRng::seed_from_u64(timestamp);
        let rng = StdRng::seed_from_u64(0);

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

        for y in 6..=CELLS_Y_MAX {
            for x in CELLS_X_MIN..=CELLS_X_MAX {
                game.cells[y as usize][x as usize].cell_type =
                    CellType::from_u32(game.rng.gen::<u32>());
            }
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

        match command {
            Command::None => {}
            Command::Left => {
                if self.player.p.x > CELLS_X_MIN
                    && self.cells[self.player.p.y as usize][self.player.p.x as usize - 1].cell_type
                        == CellType::None
                {
                    if self.player.state == PlayerState::Standing {
                        self.player.state = PlayerState::Walking;
                        self.player.direction = Direction::Left;
                        self.player.walking_frames = 0;
                    }
                }
            }
            Command::Right => {
                if self.player.p.x < CELLS_X_MAX
                    && self.cells[self.player.p.y as usize][self.player.p.x as usize + 1].cell_type
                        == CellType::None
                {
                    if self.player.state == PlayerState::Standing {
                        self.player.state = PlayerState::Walking;
                        self.player.direction = Direction::Right;
                        self.player.walking_frames = 0;
                    }
                }
            }
            Command::Dig => {
                match self.cells[self.player.p.y as usize + 1][self.player.p.x as usize].cell_type {
                    CellType::Red | CellType::Yellow | CellType::Green | CellType::Blue => {
                        self.set_leaders();

                        let leader = self.cells[self.player.p.y as usize + 1]
                            [self.player.p.x as usize]
                            .leader;
                        for y in CELLS_Y_MIN..=CELLS_Y_MAX {
                            for x in CELLS_X_MIN..=CELLS_X_MAX {
                                if self.cells[y as usize][x as usize].leader == leader {
                                    self.cells[y as usize][x as usize].cell_type = CellType::None;
                                }
                            }
                        }

                        for y in CELLS_Y_MIN..=CELLS_Y_MAX {
                            for x in CELLS_X_MIN..=CELLS_X_MAX {
                                if let Some(p) = self.cells[y as usize][x as usize].leader {
                                    print!("({} {}) ", p.x, p.y);
                                } else {
                                    print!("(   ) ");
                                }
                            }
                            println!("");
                        }
                    }
                    _ => {}
                }
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

    fn set_leaders(&mut self) {
        for y in CELLS_Y_MIN..=CELLS_Y_MAX {
            for x in CELLS_X_MIN..=CELLS_X_MAX {
                match self.cells[y as usize][x as usize].cell_type {
                    CellType::Red | CellType::Yellow | CellType::Green | CellType::Blue => {
                        match self.cells[y as usize][x as usize].leader {
                            None => {
                                let point = Point::new(x, y);
                                self.set_leader(x, y, point);
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn set_leader(&mut self, x: i32, y: i32, p: Point) {
        // let cell = &mut self.cells[y as usize][x as usize];
        let cell_type = self.cells[y as usize][x as usize].cell_type;
        self.cells[y as usize][x as usize].leader = Some(p);
        if x < CELLS_X_MAX
            && self.cells[y as usize][x as usize + 1].cell_type
                == self.cells[y as usize][x as usize].cell_type
            && self.cells[y as usize][x as usize + 1].leader == None
        {
            self.set_leader(x + 1, y, p);
        }
        if x > CELLS_X_MIN
            && self.cells[y as usize][x as usize - 1].cell_type
                == self.cells[y as usize][x as usize].cell_type
            && self.cells[y as usize][x as usize - 1].leader == None
        {
            self.set_leader(x - 1, y, p);
        }
        if y > CELLS_Y_MIN
            && self.cells[y as usize - 1][x as usize].cell_type
                == self.cells[y as usize][x as usize].cell_type
            && self.cells[y as usize - 1][x as usize].leader == None
        {
            self.set_leader(x, y - 1, p);
        }
        if y < CELLS_Y_MAX
            && self.cells[y as usize + 1][x as usize].cell_type
                == self.cells[y as usize][x as usize].cell_type
            && self.cells[y as usize + 1][x as usize].leader == None
        {
            self.set_leader(x, y + 1, p);
        }
    }

    fn cell<'a>(&'a self, x: i32, y: i32) -> &'a Cell {
        &self.cells[x as usize][y as usize]
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
