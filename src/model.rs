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
pub const AIR_SPAWN_INTERVAL: i32 = 20;
pub const BLOCK_LIFE_MAX: i32 = 100;

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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CellType {
    None,
    Air,
    Block,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum BlockColor {
    Red,
    Yellow,
    Green,
    Blue,
    Clear,
    Brown,
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
    pub leader: Option<Point>, // 同じ色のひとかたまりのリーダー component leader
    pub block_life: i32,
    pub grounded: bool,
}

impl Cell {
    fn new() -> Self {
        Cell {
            cell_type: CellType::None,
            color: BlockColor::Red,
            leader: None,
            block_life: BLOCK_LIFE_MAX,
            grounded: false,
        }
    }
}

impl std::fmt::Debug for Cell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let color = match self.color {
            BlockColor::Red => "R",
            BlockColor::Yellow => "Y",
            BlockColor::Green => "G",
            BlockColor::Blue => "B",
            BlockColor::Clear => "C",
            BlockColor::Brown => "O",
        };
        let grounded = if self.grounded { "o" } else { "x" };
        let leader = if let Some(p) = self.leader {
            format!("{},{}", p.x, p.y)
        } else {
            "None".to_string()
        };
        match self.cell_type {
            CellType::None => write!(f, "None").unwrap(),
            CellType::Air => write!(f, "Air ").unwrap(),
            CellType::Block => write!(f, "{}({}){:?}", color, grounded, leader).unwrap(),
        };
        Ok(())
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
            // p: Point::new(5, 13),
            air: AIR_MAX,
            direction: Direction::Left,
            walking_frames: 0,
            falling_frames: 0,
            state: PlayerState::Standing,
        };
        player
    }

    pub fn air_percent(&self) -> f32 {
        (self.air as f32 / AIR_MAX as f32) * 100.0f32
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
    pub depth: i32,
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
            depth: 0,
        };

        // ランダムに通常ブロックを敷き詰める
        for y in UP_SPACE_HEIGHT..=CELLS_Y_MAX {
            for x in CELLS_X_MIN..=CELLS_X_MAX {
                game.cell_mut(x, y).cell_type = CellType::Block;
                if game.rng.gen_bool(0.05) {
                    game.cell_mut(x, y).color = BlockColor::Brown;
                } else {
                    game.cell_mut(x, y).color = BlockColor::from_u32(game.rng.gen::<u32>());
                }
            }
        }

        // airを配置
        let mut depth = UP_SPACE_HEIGHT;
        while depth < CELLS_Y_LEN {
            let x = game.rng.gen::<u32>() % (CELLS_X_LEN as u32);
            let y = depth as u32 + game.rng.gen::<u32>() % (AIR_SPAWN_INTERVAL as u32);
            if y < CELLS_Y_LEN as u32 {
                game.cell_mut(x as i32, y as i32).cell_type = CellType::Air;
            }
            depth += AIR_SPAWN_INTERVAL;
        }

        // クリアブロックを配置
        for y in 0..CLEAR_BLOCKS_HEIGHT {
            for x in CELLS_X_MIN..=CELLS_X_MAX {
                game.cell_mut(x, CELLS_Y_MAX - y).cell_type = CellType::Block;
                game.cell_mut(x, CELLS_Y_MAX - y).color = BlockColor::Clear;
            }
        }

        game.print_blocks();
        game
    }

    pub fn print_blocks(&self) {
        println!("{:?}", self.player.p);
        for y in CELLS_Y_MIN..=CELLS_Y_MAX {
            print!("{: >3}: ", y);
            for x in CELLS_X_MIN..=CELLS_X_MAX {
                if self.player.p.x == x && self.player.p.y == y {
                    print!("\x1b[0;31m{:?} \x1b[0m", self.cell(x, y));
                } else {
                    print!("{:?} ", self.cell(x, y));
                }
            }
            print!("\n");
        }
    }

    pub fn next_stage(&self) -> Self {
        let mut game = Game::new();
        game.depth = self.depth;
        game
    }

    pub fn update(&mut self, command: Command) {
        if self.is_over || self.is_clear {
            return;
        }

        self.player_move();

        self.set_leaders();

        match command {
            Command::Left | Command::Right | Command::Up | Command::Down => {
                self.dig_or_move(command)
            }
            Command::None => {}
        }

        self.update_grounded();

        self.fall_ungrounded_blocks();

        // ブロックにつぶされたらゲームオーバー
        if self.cell(self.player.p.x, self.player.p.y).cell_type == CellType::Block {
            self.is_over = true;
            self.requested_sounds.push("crash.wav");
        }

        // エアを取得
        if self.cell(self.player.p.x, self.player.p.y).cell_type == CellType::Air {
            self.cell_mut(self.player.p.x, self.player.p.y).cell_type = CellType::None;
            self.player.air = clamp(0, self.player.air + (AIR_MAX as f32 * 0.2) as i32, AIR_MAX);
            self.requested_sounds.push("shrink.wav");
        }

        // エア消費
        self.player.air -= 1;
        if self.player.air <= 0 {
            self.is_over = true;
            self.requested_sounds.push("crash.wav");
        }

        self.camera_y = self.player.p.y - 5;

        self.frame += 1;
        self.score = self.frame / 30;
    }

    // 落下や移動中のアニメーション処理
    fn player_move(&mut self) {
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
                self.depth += 1;
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
    }

    // カーソルキーの入力に応じて、掘る、または移動開始する
    fn dig_or_move(&mut self, command: Command) {
        match command {
            Command::Left => {
                if self.player.state == PlayerState::Standing && self.player.p.x > CELLS_X_MIN {
                    match self.cell(self.player.p.x - 1, self.player.p.y).cell_type {
                        CellType::None | CellType::Air => {
                            self.player.state = PlayerState::Walking;
                            self.player.direction = Direction::Left;
                            self.player.walking_frames = 0;
                        }
                        CellType::Block => {
                            self.break_cell(self.player.p.x - 1, self.player.p.y);
                        }
                    }
                }
            }
            Command::Right => {
                if self.player.state == PlayerState::Standing && self.player.p.x < CELLS_X_MAX {
                    match self.cell(self.player.p.x + 1, self.player.p.y).cell_type {
                        CellType::None | CellType::Air => {
                            self.player.state = PlayerState::Walking;
                            self.player.direction = Direction::Right;
                            self.player.walking_frames = 0;
                        }
                        CellType::Block => {
                            self.break_cell(self.player.p.x + 1, self.player.p.y);
                        }
                    }
                }
            }
            Command::Down => match self.cell(self.player.p.x, self.player.p.y + 1).cell_type {
                CellType::Block => {
                    self.break_cell(self.player.p.x, self.player.p.y + 1);
                }
                CellType::Air => {
                    self.player.p.y += 1;
                }
                _ => {}
            },
            Command::Up => match self.cell(self.player.p.x, self.player.p.y - 1).cell_type {
                CellType::Block => {
                    self.break_cell(self.player.p.x, self.player.p.y - 1);
                }
                CellType::Air => {
                    self.player.p.y += 1;
                }
                _ => {}
            },
            _ => return,
        }
        self.print_blocks();
    }

    // ブロックが接地しているか判定して記録する
    fn update_grounded(&mut self) {
        // いったん全部falseにする
        for y in CELLS_Y_MIN..=CELLS_Y_MAX {
            for x in CELLS_X_MIN..=CELLS_X_MAX {
                self.cell_mut(x, y).grounded = false;
            }
        }
        // 下からループして
        for y in (CELLS_Y_MIN..=CELLS_Y_MAX).rev() {
            for x in CELLS_X_MIN..=CELLS_X_MAX {
                if self.cell(x, y).grounded == false {
                    // 一番底のクリアブロック、または1個下に接地したブロックまたはエアがあるならそこも接地している
                    let grounded = y == CELLS_Y_MAX
                        || (self.cell(x, y + 1).cell_type != CellType::None
                            && self.cell(x, y + 1).grounded);
                    if grounded {
                        match self.cell(x, y).cell_type {
                            CellType::None => {}
                            CellType::Air => self.cell_mut(x, y).grounded = true,
                            CellType::Block => {
                                // つながったブロックを全部接地にする
                                let component = self.get_component(x, y);
                                for point in &component {
                                    self.cell_mut(point.x, point.y).grounded = true;
                                }
                            }
                        };
                    }
                }
            }
        }
    }

    // 接地していないブロックを落とす
    fn fall_ungrounded_blocks(&mut self) {
        // 下からループして
        for y in (CELLS_Y_MIN..=CELLS_Y_MAX).rev() {
            for x in CELLS_X_MIN..=CELLS_X_MAX {
                if self.cell(x, y).cell_type != CellType::None
                    && !self.cell(x, y).grounded
                    && y < CELLS_Y_MAX
                {
                    // self.cells[(y + 1][x] = self.cells[y][x];
                    *self.cell_mut(x, y + 1) = *self.cell(x, y);
                    self.cell_mut(x, y).cell_type = CellType::None;
                    // *self.cell_mut(x, y) = *self.cell(x, y - 1);
                    // self.cell_mut(x, y + 1).color = self.cell(x, y).color;
                }
            }
        }
    }

    // 指定したブロックとつながっているブロックの座標のリストを返す
    fn get_component(&self, x: i32, y: i32) -> Vec<Point> {
        let mut result = Vec::new();
        for yi in CELLS_Y_MIN..=CELLS_Y_MAX {
            for xi in CELLS_X_MIN..=CELLS_X_MAX {
                if self.cell(xi, yi).leader == self.cell(x, y).leader {
                    result.push(Point::new(xi, yi));
                }
            }
        }
        result
    }

    fn break_cell(&mut self, cell_x: i32, cell_y: i32) {
        if self.cell(cell_x, cell_y).color == BlockColor::Clear {
            self.is_clear = true;
            self.requested_sounds.push("clear.wav");
        }

        if self.cell(cell_x, cell_y).color == BlockColor::Brown {
            self.cell_mut(cell_x, cell_y).block_life -= 25;
        } else {
            self.cell_mut(cell_x, cell_y).block_life = 0;
        }
        if self.cell(cell_x, cell_y).block_life > 0 {
            return;
        }
        if self.cell(cell_x, cell_y).color == BlockColor::Brown {
            self.player.air = clamp(0, self.player.air - (AIR_MAX as f32 * 0.23) as i32, AIR_MAX);
            self.requested_sounds.push("break_brown.wav");
        }

        // つながっているブロックを消去
        let leader = self.cell(cell_x, cell_y).leader;
        for y in CELLS_Y_MIN..=CELLS_Y_MAX {
            for x in CELLS_X_MIN..=CELLS_X_MAX {
                if self.cell(x, y).leader == leader {
                    self.cell_mut(x, y).cell_type = CellType::None;
                }
            }
        }
    }

    // 全ブロックのつながりを判定
    fn set_leaders(&mut self) {
        for y in CELLS_Y_MIN..=CELLS_Y_MAX {
            for x in CELLS_X_MIN..=CELLS_X_MAX {
                self.cell_mut(x, y).leader = None;
            }
        }

        for y in CELLS_Y_MIN..=CELLS_Y_MAX {
            for x in CELLS_X_MIN..=CELLS_X_MAX {
                if self.cell(x, y).leader == None {
                    let point = Point::new(x, y);
                    self.set_leader(x, y, point);
                }
            }
        }
    }

    fn set_leader(&mut self, x: i32, y: i32, p: Point) {
        if self.cell(x, y).cell_type != CellType::Block {
            return;
        }

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
        self.depth
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
