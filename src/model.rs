use rand::prelude::*;
use std::time;

pub const UP_SPACE_HEIGHT: i32 = 6; // 初期状態の上の空間の高さ
pub const NORMAL_BLOCKS_HEIGHT: i32 = 100; // 通常ブロックがある空間の高さ
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

pub const FPS: i32 = 30;
pub const WALK_FRAMES: i32 = 3; // プレイヤーが1マス歩くのにかかるフレーム数
pub const FALL_FRAMES: i32 = 3; // プレイヤーが1マス落ちるのにかかるフレーム数
                                // pub const SHAKE_FRAMES: i32 = 48; // 落下予定のブロックがぐらついているフレーム数（揺れるアニメーションが片側4フレームなので、4の倍数）
pub const SHAKE_FRAMES: i32 = 43; // 落下予定のブロックがぐらついているフレーム数（揺れるアニメーションが片側4フレームなので、4の倍数 - 1）

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Command {
    None,
    Left,
    Right,
    Down,
    Up,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    pub fn all() -> [Direction; 4] {
        [
            Direction::Left,
            Direction::Right,
            Direction::Up,
            Direction::Down,
        ]
    }

    pub fn from_command(command: Command) -> Direction {
        match command {
            Command::Left => Direction::Left,
            Command::Right => Direction::Right,
            Command::Up => Direction::Up,
            Command::Down => Direction::Down,
            _ => panic!(),
        }
    }
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
    pub shaking_frames: i32,
    pub falling_frames: i32,
    pub fell: bool, // このフレームに落下したか
}

impl Cell {
    fn new() -> Self {
        Cell {
            cell_type: CellType::None,
            color: BlockColor::Red,
            leader: None,
            block_life: BLOCK_LIFE_MAX,
            grounded: false,
            shaking_frames: -1,
            falling_frames: -1,
            fell: false,
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
            CellType::Block => write!(
                f,
                "{}({}){:?} {}",
                color, grounded, leader, self.falling_frames
            )
            .unwrap(),
        };
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Self {
        assert!(x >= CELLS_X_MIN);
        assert!(x <= CELLS_X_MAX);
        assert!(y >= CELLS_Y_MIN);
        assert!(y <= CELLS_Y_MAX);
        Point { x: x, y: y }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
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
    pub is_debug: bool,
    pub is_over: bool,
    pub is_clear: bool,
    pub frame: i32,
    pub player: Player,
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
        let rng = StdRng::seed_from_u64(timestamp);
        println!("random seed = {}", timestamp);
        // let rng = StdRng::seed_from_u64(0);

        let mut game = Game {
            rng: rng,
            is_debug: false,
            is_over: false,
            is_clear: false,
            frame: -1,
            player: Player::new(),
            requested_sounds: Vec::new(),
            cells: [[Cell::new(); CELLS_X_LEN as usize]; CELLS_Y_LEN as usize],
            camera_y: 0,
            depth: 0,
        };

        // ランダムに通常ブロックを敷き詰める
        for y in UP_SPACE_HEIGHT..=CELLS_Y_MAX {
            for x in CELLS_X_MIN..=CELLS_X_MAX {
                let p = Point::new(x, y);
                game.cell_mut(p).cell_type = CellType::Block;
                if game.rng.gen_bool(0.05) {
                    game.cell_mut(p).color = BlockColor::Brown;
                } else {
                    game.cell_mut(p).color = BlockColor::from_u32(game.rng.gen::<u32>());
                }
            }
        }

        // airを配置
        let mut depth = UP_SPACE_HEIGHT;
        while depth < CELLS_Y_LEN {
            let x = game.rng.gen::<u32>() % (CELLS_X_LEN as u32);
            let y = depth as u32 + game.rng.gen::<u32>() % (AIR_SPAWN_INTERVAL as u32);
            if y < CELLS_Y_LEN as u32 {
                let p = Point::new(x as i32, y as i32);
                game.cell_mut(p).cell_type = CellType::Air;
            }
            depth += AIR_SPAWN_INTERVAL;
        }

        // クリアブロックを配置
        for y in 0..CLEAR_BLOCKS_HEIGHT {
            for x in CELLS_X_MIN..=CELLS_X_MAX {
                let p = Point::new(x, CELLS_Y_MAX - y);
                game.cell_mut(p).cell_type = CellType::Block;
                game.cell_mut(p).color = BlockColor::Clear;
            }
        }

        game
    }

    pub fn toggle_debug(&mut self) {
        self.is_debug = !self.is_debug;
        println!("is_debug: {}", self.is_debug);
    }

    // デバッグ用：ブロックの状態を表示
    #[allow(dead_code)]
    pub fn print_blocks(&self) {
        println!("{:?}", self.player.p);
        for y in CELLS_Y_MIN..=CELLS_Y_MAX {
            print!("{: >3}: ", y);
            for x in CELLS_X_MIN..=CELLS_X_MAX {
                let p = Point::new(x, y);
                if self.player.p == p {
                    print!("\x1b[0;31m{:?} \x1b[0m", self.cell(p));
                } else {
                    print!("{:?} ", self.cell(p));
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
        self.frame += 1; // updateの最初でframeをインクリメント（early returnした場合も増加するように）

        if self.is_over || self.is_clear {
            return;
        }

        self.player_move();

        self.fall_ungrounded_blocks();

        self.set_leaders();

        self.erase_connected_blocks();

        match command {
            Command::Left | Command::Right | Command::Up | Command::Down => {
                self.dig_or_walk(Direction::from_command(command.clone()));
                if self.is_clear {
                    return;
                }
            }
            Command::None => {}
        }

        self.update_grounded();

        // エアを取得
        if self.cell(self.player.p).cell_type == CellType::Air {
            self.cell_mut(self.player.p.clone()).cell_type = CellType::None;
            self.player.air = clamp(0, self.player.air + (AIR_MAX as f32 * 0.2) as i32, AIR_MAX);
            self.requested_sounds.push("shrink.wav");
        }

        // エア消費
        self.player.air -= 1;
        if self.player.air <= 0 {
            self.is_over = true;
            self.requested_sounds.push("crash.wav");
        }

        // ブロックにつぶされたらゲームオーバー
        if self.cell(self.player.p).cell_type == CellType::Block {
            self.is_over = true;
            self.requested_sounds.push("crash.wav");
        }

        self.camera_y = self.player.p.y - 5;
    }

    // 落下や歩行中のアニメーション処理
    fn player_move(&mut self) {
        // 下に足場が無ければ落下中にする
        if let Some(down) = self.neighbor(self.player.p, Direction::Down) {
            if (self.cell(down).cell_type == CellType::None
                || self.cell(down).cell_type == CellType::Air)
                && self.player.state != PlayerState::Falling
            {
                self.player.state = PlayerState::Falling;
                self.player.falling_frames = 0;
            }
        }

        // 落下中
        if self.player.state == PlayerState::Falling {
            self.player.falling_frames += 1;
            if self.player.falling_frames >= FALL_FRAMES {
                // 1マス分落下完了
                self.player.falling_frames = 0;
                self.player.p.y += 1;
                self.player.state = PlayerState::Standing;
                self.depth += 1;
            }
        }

        // 歩行中
        if self.player.state == PlayerState::Walking {
            self.player.walking_frames += 1;
            if self.player.walking_frames >= WALK_FRAMES {
                // 1マス分歩行完了
                if self.player.direction == Direction::Left {
                    self.player.p.x -= 1;
                } else {
                    self.player.p.x += 1;
                }
                self.player.state = PlayerState::Standing;
            }
        }
    }

    // 指定方向に掘る、または歩行開始する
    fn dig_or_walk(&mut self, direction: Direction) {
        match direction {
            Direction::Left | Direction::Right => {
                if self.player.state == PlayerState::Standing {
                    if let Some(p) = self.neighbor(self.player.p, direction) {
                        match self.cell(p).cell_type {
                            CellType::None | CellType::Air => {
                                self.player.state = PlayerState::Walking;
                                self.player.direction = direction;
                                self.player.walking_frames = 0;
                            }
                            CellType::Block => {
                                self.dig(p);
                            }
                        }
                    }
                }
            }
            Direction::Up | Direction::Down => {
                if let Some(p) = self.neighbor(self.player.p, direction) {
                    match self.cell(p).cell_type {
                        CellType::Block => self.dig(p),
                        _ => {}
                    }
                }
            }
        }
    }

    // 落下したブロックが指定個数以上つながったら消す
    fn erase_connected_blocks(&mut self) {
        for y in CELLS_Y_MIN..=CELLS_Y_MAX {
            for x in CELLS_X_MIN..=CELLS_X_MAX {
                let p = Point::new(x, y);
                if self.cell(p).cell_type == CellType::Block && self.cell(p).fell {
                    let component = self.get_component(p);
                    if component.len() >= 4 {
                        for point in component {
                            self.cell_mut(point).cell_type = CellType::None;
                        }
                    }
                }
            }
        }
    }

    // ブロックが接地しているか判定して記録する
    fn update_grounded(&mut self) {
        // いったん全部falseにする
        for y in CELLS_Y_MIN..=CELLS_Y_MAX {
            for x in CELLS_X_MIN..=CELLS_X_MAX {
                let p = Point::new(x, y);
                self.cell_mut(p).grounded = false;
            }
        }
        // 下からループして
        for y in (CELLS_Y_MIN..=CELLS_Y_MAX).rev() {
            for x in CELLS_X_MIN..=CELLS_X_MAX {
                let p = Point::new(x, y);
                if self.cell(p).grounded == false {
                    // 一番底のクリアブロック、または1個下に接地したブロックまたはエアがあるならそこも接地している
                    let down = self.neighbor(p, Direction::Down);
                    let grounded = down == None
                        || (self.cell(down.unwrap()).cell_type != CellType::None
                            && self.cell(down.unwrap()).grounded);
                    if grounded {
                        match self.cell(p).cell_type {
                            CellType::None => {}
                            CellType::Air => self.cell_mut(p).grounded = true,
                            CellType::Block => {
                                // つながったブロックを全部接地にする
                                let component = self.get_component(p);
                                for point in component {
                                    self.cell_mut(point).grounded = true;
                                    self.cell_mut(point).shaking_frames = -1;
                                    self.cell_mut(point).falling_frames = -1;
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
                let p = Point::new(x, y);

                self.cell_mut(p).fell = false;
                if self.cell(p).cell_type != CellType::None {
                    if !self.cell(p).grounded {
                        if self.cell(p).shaking_frames < 0 {
                            // 揺らし開始
                            self.cell_mut(p).shaking_frames = 0;
                        } else if self.cell(p).shaking_frames <= SHAKE_FRAMES {
                            // 揺らし中
                            self.cell_mut(p).shaking_frames += 1;
                        } else {
                            // 揺らし終わった
                            if self.cell(p).falling_frames < 0 {
                                // 揺らし終わったら落下開始
                                self.cell_mut(p).falling_frames = 0;
                            } else if self.cell(p).falling_frames <= FALL_FRAMES {
                                self.cell_mut(p).falling_frames += 1;
                            } else {
                                // 落下し終わったらセル移動
                                let down = self.neighbor(p, Direction::Down).unwrap();
                                *self.cell_mut(down) = *self.cell(p);
                                self.cell_mut(p).cell_type = CellType::None;
                                self.cell_mut(down).fell = true;

                                // 下にエアがあったら潰す
                                if let Some(down2) = self.neighbor(down, Direction::Down) {
                                    if self.cell(down2).cell_type == CellType::Air {
                                        self.cell_mut(down2).cell_type = CellType::None;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 指定したブロックとつながっているブロックの座標のリストを返す
    fn get_component(&self, p: Point) -> Vec<Point> {
        let mut result = Vec::new();
        for yi in CELLS_Y_MIN..=CELLS_Y_MAX {
            for xi in CELLS_X_MIN..=CELLS_X_MAX {
                let xiyi = Point::new(xi, yi);
                if self.cell(xiyi).leader == self.cell(p).leader {
                    result.push(xiyi);
                }
            }
        }
        result
    }

    // 指定された箇所を掘る
    fn dig(&mut self, p: Point) {
        if self.cell(p).color == BlockColor::Clear {
            self.is_clear = true;
            self.requested_sounds.push("clear.wav");
        }

        if self.cell(p).color == BlockColor::Brown {
            self.cell_mut(p).block_life -= 25;
        } else {
            self.cell_mut(p).block_life = 0;
        }
        if self.cell(p).block_life > 0 {
            return;
        }
        if self.cell(p).color == BlockColor::Brown {
            self.player.air = clamp(0, self.player.air - (AIR_MAX as f32 * 0.23) as i32, AIR_MAX);
            self.requested_sounds.push("break_brown.wav");
        }

        // つながっているブロックを消去
        let leader = self.cell(p).leader;
        for y in CELLS_Y_MIN..=CELLS_Y_MAX {
            for x in CELLS_X_MIN..=CELLS_X_MAX {
                let xy = Point::new(x, y);
                if self.cell(xy).leader == leader {
                    self.cell_mut(xy).cell_type = CellType::None;
                }
            }
        }
    }

    // 全ブロックのつながり方を判定
    fn set_leaders(&mut self) {
        for y in CELLS_Y_MIN..=CELLS_Y_MAX {
            for x in CELLS_X_MIN..=CELLS_X_MAX {
                let p = Point::new(x, y);
                self.cell_mut(p).leader = None;
            }
        }

        for y in CELLS_Y_MIN..=CELLS_Y_MAX {
            for x in CELLS_X_MIN..=CELLS_X_MAX {
                let p = Point::new(x, y);
                if self.cell(p).leader == None {
                    self.set_leader(p, p);
                }
            }
        }
    }

    fn set_leader(&mut self, p: Point, leader: Point) {
        if self.cell(p).cell_type != CellType::Block {
            return;
        }

        self.cell_mut(p).leader = Some(leader);
        let directions = Direction::all();
        for direction in directions {
            if let Some(neighbor) = self.neighbor(p, direction) {
                if self.cell(neighbor).color == self.cell(p).color
                    && self.cell(neighbor).leader == None
                {
                    self.set_leader(neighbor, leader);
                }
            }
        }
    }

    pub fn neighbor(&self, p: Point, direction: Direction) -> Option<Point> {
        match direction {
            Direction::Left => {
                if p.x - 1 >= CELLS_X_MIN {
                    Some(Point::new(p.x - 1, p.y))
                } else {
                    None
                }
            }
            Direction::Right => {
                if p.x + 1 <= CELLS_X_MAX {
                    Some(Point::new(p.x + 1, p.y))
                } else {
                    None
                }
            }
            Direction::Up => {
                if p.y - 1 >= CELLS_Y_MIN {
                    Some(Point::new(p.x, p.y - 1))
                } else {
                    None
                }
            }
            Direction::Down => {
                if p.y + 1 <= CELLS_Y_MAX {
                    Some(Point::new(p.x, p.y + 1))
                } else {
                    None
                }
            }
        }
    }

    pub fn cell<'a>(&'a self, p: Point) -> &'a Cell {
        &self.cells[p.y as usize][p.x as usize]
    }

    fn cell_mut<'a>(&'a mut self, p: Point) -> &'a mut Cell {
        &mut self.cells[p.y as usize][p.x as usize]
    }

    pub fn get_depth(&self) -> i32 {
        self.depth
    }
}

pub fn clamp<T: PartialOrd>(min: T, value: T, max: T) -> T {
    if value < min {
        return min;
    }
    if value > max {
        return max;
    }
    value
}
