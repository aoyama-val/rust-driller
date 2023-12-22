use rand::prelude::*;
use std::time;

pub const CELL_SIZE: i32 = 40;
pub const INFO_WIDTH: i32 = 100;
pub const INFO_X: i32 = CELL_SIZE * CELLS_X_LEN;
pub const SCREEN_WIDTH: i32 = CELL_SIZE * CELLS_X_LEN + INFO_WIDTH;
pub const SCREEN_HEIGHT: i32 = CELL_SIZE * 12;

pub const UP_SPACE_HEIGHT: i32 = 6; // 初期状態の上の空間の高さ
pub const NORMAL_BLOCKS_HEIGHT: i32 = 30; // 通常ブロックがある空間の高さ
pub const CLEAR_BLOCKS_HEIGHT: i32 = 7; // 底にあるクリアブロックの高さ
pub const CELLS_X_LEN: i32 = 9;
pub const CELLS_X_MIN: i32 = 0;
pub const CELLS_X_MAX: i32 = CELLS_X_LEN - 1;
pub const CELLS_Y_LEN: i32 = UP_SPACE_HEIGHT + NORMAL_BLOCKS_HEIGHT + CLEAR_BLOCKS_HEIGHT;
pub const CELLS_Y_MIN: i32 = 0;
pub const CELLS_Y_MAX: i32 = CELLS_Y_LEN - 1;

pub const AIR_MAX: i32 = 3000;

pub const WALK_FRAMES: i32 = 3; // 1マス歩くのにかかるフレーム数
pub const FALL_FRAMES: i32 = 3; // 1マス落ちるのにかかるフレーム数

pub enum Command {
    None,
    Left,
    Right,
    Down,
    Up,
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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CellType {
    None,
    Air,
    Box,
    Block,
}

#[derive(Clone, Copy)]
pub enum CellState {
    Normal,
    Connected,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum BlockColor {
    Red,
    Yellow,
    Green,
    Blue,
    Clear,
}

impl BlockColor {
    fn from_u32(n: u32) -> Self {
        match n % 4 {
            0 => BlockColor::Red,
            1 => BlockColor::Yellow,
            2 => BlockColor::Green,
            3 => BlockColor::Blue,
            _ => panic!(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Cell {
    pub cell_type: CellType,
    pub color: BlockColor,
    pub leader: Option<Point>,
}

impl Cell {
    fn new() -> Self {
        Cell {
            cell_type: CellType::None,
            color: BlockColor::Red,
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
}

pub struct Game {
    pub rng: StdRng,
    pub is_over: bool,
    pub is_clear: bool,
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
            is_clear: false,
            frame: 0,
            player: Player::new(),
            score: 0,
            requested_sounds: Vec::new(),
            cells: [[Cell::new(); CELLS_X_LEN as usize]; CELLS_Y_LEN as usize],
            camera_y: 0,
        };

        // ランダムに通常ブロックを敷き詰める
        for y in UP_SPACE_HEIGHT..=CELLS_Y_MAX {
            for x in CELLS_X_MIN..=CELLS_X_MAX {
                game.cell_mut(x, y).cell_type = CellType::Block;
                game.cell_mut(x, y).color = BlockColor::from_u32(game.rng.gen::<u32>());
            }
        }

        // TODO: airを配置
        // TODO: Boxを配置

        // クリアブロックを配置
        for y in 0..CLEAR_BLOCKS_HEIGHT {
            for x in CELLS_X_MIN..=CELLS_X_MAX {
                game.cell_mut(x, CELLS_Y_MAX - y).cell_type = CellType::Block;
                game.cell_mut(x, CELLS_Y_MAX - y).color = BlockColor::Clear;
            }
        }

        for y in CELLS_Y_MIN..=CELLS_Y_MAX {
            print!("{}: ", y);
            for x in CELLS_X_MIN..=CELLS_X_MAX {
                print!("{:?} ", game.cell(x, y).cell_type);
            }
            print!("\n");
        }
        println!("{}", game.player.p.y);

        game
    }

    pub fn update(&mut self, command: Command) {
        if self.is_over || self.is_clear {
            return;
        }

        // println!("{:?}", self.player.state);

        if self.cell(self.player.p.x, self.player.p.y + 1).cell_type == CellType::None
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
                println!("left");
                if self.player.state == PlayerState::Standing && self.player.p.x > CELLS_X_MIN {
                    match self.cell(self.player.p.x - 1, self.player.p.y).cell_type {
                        CellType::None | CellType::Air => {
                            self.player.state = PlayerState::Walking;
                            self.player.direction = Direction::Left;
                            self.player.walking_frames = 0;
                        }
                        CellType::Block | CellType::Box => {
                            self.break_cell(self.player.p.x - 1, self.player.p.y);
                        }
                    }
                }
            }
            Command::Right => {
                println!("right");
                if self.player.state == PlayerState::Standing && self.player.p.x < CELLS_X_MAX {
                    match self.cell(self.player.p.x + 1, self.player.p.y).cell_type {
                        CellType::None | CellType::Air => {
                            self.player.state = PlayerState::Walking;
                            self.player.direction = Direction::Right;
                            self.player.walking_frames = 0;
                        }
                        CellType::Block | CellType::Box => {
                            self.break_cell(self.player.p.x + 1, self.player.p.y);
                        }
                    }
                }
            }
            Command::Down => match self.cell(self.player.p.x, self.player.p.y + 1).cell_type {
                CellType::Block => {
                    self.break_cell(self.player.p.x, self.player.p.y + 1);
                }
                _ => {}
            },
            Command::Up => match self.cell(self.player.p.x, self.player.p.y - 1).cell_type {
                CellType::Block => {
                    self.break_cell(self.player.p.x, self.player.p.y - 1);
                }
                _ => {}
            },
        }

        self.player.air -= 1;
        if self.player.air <= 0 {
            self.is_over = true;
            self.requested_sounds.push("crash.wav");
        }

        if self.player.p.y >= CELLS_Y_LEN - 7 {
            self.is_clear = true;
        }

        self.camera_y = self.player.p.y - 5;

        self.frame += 1;
        self.score = self.frame / 30;
    }

    fn break_cell(&mut self, cell_x: i32, cell_y: i32) {
        if self.cell(cell_x, cell_y).color == BlockColor::Clear {
            self.is_clear = true;
        }

        self.set_leaders();

        // つながっているブロックを消去
        let leader = self.cell(cell_x, cell_y).leader;
        for y in CELLS_Y_MIN..=CELLS_Y_MAX {
            for x in CELLS_X_MIN..=CELLS_X_MAX {
                if self.cell(x, y).leader == leader {
                    self.cell_mut(x, y).cell_type = CellType::None;
                }
            }
        }

        // for y in CELLS_Y_MIN..=CELLS_Y_MAX {
        //     for x in CELLS_X_MIN..=CELLS_X_MAX {
        //         if let Some(p) = self.cell(x, y).leader {
        //             print!("({} {}) ", p.x, p.y);
        //         } else {
        //             print!("(   ) ");
        //         }
        //     }
        //     println!("");
        // }
    }

    // 全ブロックのつながりを判定
    fn set_leaders(&mut self) {
        for y in CELLS_Y_MIN..=CELLS_Y_MAX {
            for x in CELLS_X_MIN..=CELLS_X_MAX {
                if self.cell(x, y).cell_type == CellType::Block {
                    if self.cell(x, y).leader == None {
                        let point = Point::new(x, y);
                        self.set_leader(x, y, point);
                    }
                }
            }
        }
    }

    fn set_leader(&mut self, x: i32, y: i32, p: Point) {
        self.cell_mut(x, y).leader = Some(p);
        if x < CELLS_X_MAX
            && self.cell(x + 1, y).color == self.cell(x, y).color
            && self.cell(x + 1, y).leader == None
        {
            self.set_leader(x + 1, y, p);
        }
        if x > CELLS_X_MIN
            && self.cell(x - 1, y).color == self.cell(x, y).color
            && self.cell(x - 1, y).leader == None
        {
            self.set_leader(x - 1, y, p);
        }
        if y > CELLS_Y_MIN
            && self.cell(x, y - 1).color == self.cell(x, y).color
            && self.cell(x, y - 1).leader == None
        {
            self.set_leader(x, y - 1, p);
        }
        if y < CELLS_Y_MAX
            && self.cell(x, y + 1).color == self.cell(x, y).color
            && self.cell(x, y + 1).leader == None
        {
            self.set_leader(x, y + 1, p);
        }
    }

    pub fn cell<'a>(&'a self, x: i32, y: i32) -> &'a Cell {
        &self.cells[y as usize][x as usize]
    }

    fn cell_mut<'a>(&'a mut self, x: i32, y: i32) -> &'a mut Cell {
        &mut self.cells[y as usize][x as usize]
    }

    pub fn get_depth(&self) -> i32 {
        std::cmp::max(self.player.p.y - UP_SPACE_HEIGHT + 1, 0)
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
