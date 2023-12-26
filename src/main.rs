use sdl2::event::Event;
use sdl2::gfx::primitives::DrawRenderer;
use sdl2::keyboard::Keycode;
use sdl2::mixer;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::{BlendMode, Canvas, Texture, TextureCreator};
use sdl2::sys::Font;
use sdl2::ttf::Sdl2TtfContext;
use sdl2::video::{Window, WindowContext};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};
mod model;
use crate::model::*;

struct Image<'a> {
    texture: Texture<'a>,
    w: u32,
    h: u32,
}

impl<'a> Image<'a> {
    fn new(texture: Texture<'a>) -> Self {
        let q = texture.query();
        let image = Image {
            texture,
            w: q.width,
            h: q.height,
        };
        image
    }
}

struct Resources<'a> {
    images: HashMap<String, Image<'a>>,
    chunks: HashMap<String, sdl2::mixer::Chunk>,
    fonts: HashMap<String, sdl2::ttf::Font<'a, 'a>>,
}

pub fn main() -> Result<(), String> {
    let sdl_context = sdl2::init()?;

    let video_subsystem = sdl_context.video()?;
    let window = video_subsystem
        .window("rust-driller", SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32)
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;

    sdl_context.mouse().show_cursor(false);

    init_mixer();

    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    canvas.set_blend_mode(BlendMode::Blend);

    let texture_creator = canvas.texture_creator();
    let mut resources = load_resources(&texture_creator, &mut canvas, &ttf_context);

    let mut event_pump = sdl_context.event_pump()?;

    let mut game = Game::new();

    println!("Keys:");
    println!("    Left  : Move player or dig left");
    println!("    Right : Move player or dig right");
    println!("    Down  : Dig down");
    println!("    Up    : Dig up");
    println!("    Space : Restart when game over");

    'running: loop {
        let started = SystemTime::now();

        let mut command = Command::None;
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                Event::KeyDown {
                    keycode: Some(Keycode::Space),
                    ..
                } => {
                    if game.is_over {
                        game = Game::new();
                    } else if game.is_clear {
                        game = game.next_stage();
                    }
                }
                Event::KeyDown {
                    keycode: Some(code),
                    ..
                } => {
                    match code {
                        Keycode::Left => command = Command::Left,
                        Keycode::Right => command = Command::Right,
                        Keycode::Down => command = Command::Down,
                        Keycode::Up => command = Command::Up,
                        Keycode::F1 => game.toggle_debug(),
                        _ => {}
                    };
                }
                _ => {}
            }
        }
        game.update(command);
        render(&mut canvas, &game, &mut resources)?;

        play_sounds(&mut game, &resources);

        let finished = SystemTime::now();
        let elapsed = finished.duration_since(started).unwrap();
        let frame_duration = Duration::new(0, 1_000_000_000u32 / model::FPS as u32);
        if elapsed < frame_duration {
            ::std::thread::sleep(frame_duration - elapsed)
        }
    }

    Ok(())
}

fn init_mixer() {
    let chunk_size = 1_024;
    mixer::open_audio(
        mixer::DEFAULT_FREQUENCY,
        mixer::DEFAULT_FORMAT,
        mixer::DEFAULT_CHANNELS,
        chunk_size,
    )
    .expect("cannot open audio");
    let _mixer_context = mixer::init(mixer::InitFlag::MP3).expect("cannot init mixer");
}

fn load_resources<'a>(
    texture_creator: &'a TextureCreator<WindowContext>,
    canvas: &mut Canvas<Window>,
    ttf_context: &'a Sdl2TtfContext,
) -> Resources<'a> {
    let mut resources = Resources {
        images: HashMap::new(),
        chunks: HashMap::new(),
        fonts: HashMap::new(),
    };

    let entries = fs::read_dir("resources/image").unwrap();
    for entry in entries {
        let path = entry.unwrap().path();
        let path_str = path.to_str().unwrap();
        if path_str.ends_with(".bmp") {
            let temp_surface = sdl2::surface::Surface::load_bmp(&path).unwrap();
            let texture = texture_creator
                .create_texture_from_surface(&temp_surface)
                .expect(&format!("cannot load image: {}", path_str));

            let basename = path.file_name().unwrap().to_str().unwrap();
            let image = Image::new(texture);
            resources.images.insert(basename.to_string(), image);
        }
    }

    let entries = fs::read_dir("./resources/sound").unwrap();
    for entry in entries {
        let path = entry.unwrap().path();
        let path_str = path.to_str().unwrap();
        if path_str.ends_with(".wav") {
            let chunk = mixer::Chunk::from_file(path_str)
                .expect(&format!("cannot load sound: {}", path_str));
            let basename = path.file_name().unwrap().to_str().unwrap();
            resources.chunks.insert(basename.to_string(), chunk);
        }
    }

    let entries = fs::read_dir("./resources/font").unwrap();
    for entry in entries {
        let path = entry.unwrap().path();
        let path_str = path.to_str().unwrap();
        if path_str.ends_with(".ttf") {
            let font = ttf_context
                .load_font(path_str, 32) // FIXME: サイズ固定になっちゃってる
                .expect(&format!("cannot load font: {}", path_str));
            let basename = path.file_name().unwrap().to_str().unwrap();
            resources.fonts.insert(basename.to_string(), font);
        }
    }

    resources
}

fn render(
    canvas: &mut Canvas<Window>,
    game: &Game,
    resources: &mut Resources,
) -> Result<(), String> {
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    // println!(
    //     "cell(0, 7) = {:?}, cell(0, 8) = {:?}",
    //     game.cell(0, 7),
    //     game.cell(0, 8)
    // );

    // render cells
    for x in CELLS_X_MIN..=CELLS_X_MAX {
        for y in 0..12 {
            let cell_y = game.camera_y + y;

            let cell = game.cell(x, cell_y);
            let shaking = cell.shaking_frames;
            let falling = cell.falling_frames;
            let offset_xs = [0, 1, 2, 1, 0, -1, -2, -1];
            let offset_x = if !cell.grounded && shaking >= 0 {
                offset_xs[(shaking as usize) % offset_xs.len()]
            } else {
                0
            };
            let offset_y = if !cell.grounded && falling >= 0 {
                clamp(
                    0,
                    ((falling as f32 / FALL_FRAMES as f32) * (CELL_SIZE as f32)) as i32,
                    CELL_SIZE as i32,
                )
            } else {
                0
            };

            match game.cell(x, cell_y).cell_type {
                CellType::None => {}
                CellType::Air => {
                    canvas.filled_ellipse(
                        ((CELL_SIZE * x) + (CELL_SIZE / 2) + offset_x) as i16,
                        ((CELL_SIZE * y) + (CELL_SIZE / 2) + offset_y) as i16,
                        (CELL_SIZE / 2) as i16,
                        (CELL_SIZE / 4) as i16,
                        // Color::RGB(209, 220, 230),
                        // Color::RGB(0x24, 0x93, 0x74),
                        Color::RGB(0x63, 0xc1, 0xa5),
                    )?;
                }
                CellType::Block => {
                    let color = match game.cell(x, cell_y).color {
                        BlockColor::Red => Color::RGB(255, 128, 128),
                        BlockColor::Yellow => Color::RGB(255, 255, 128),
                        BlockColor::Green => Color::RGB(128, 255, 128),
                        BlockColor::Blue => Color::RGB(128, 128, 255),
                        BlockColor::Clear => Color::RGB(0x63, 0xc1, 0xa5),
                        BlockColor::Brown => Color::RGB(92, 48, 28),
                    };
                    canvas.set_draw_color(color);
                    let dug_in_px = ((BLOCK_LIFE_MAX - game.cell(x, cell_y).block_life) as f32
                        / 100.0
                        * CELL_SIZE as f32) as i32;
                    canvas.fill_rect(Rect::new(
                        CELL_SIZE as i32 * x + offset_x,
                        CELL_SIZE as i32 * y + dug_in_px + offset_y,
                        CELL_SIZE as u32,
                        (CELL_SIZE - dug_in_px) as u32,
                    ))?;
                }
            }
            // 接地していないところに半透明の赤（デバッグ用）
            // if game.cell(x, cell_y).cell_type != CellType::None {
            //     if game.cell(x, cell_y).grounded == false {
            //         canvas.set_draw_color(Color::RGBA(255, 0, 0, 128));
            //         canvas.fill_rect(Rect::new(
            //             CELL_SIZE as i32 * x,
            //             CELL_SIZE as i32 * y,
            //             CELL_SIZE as u32,
            //             CELL_SIZE as u32,
            //         ))?;
            //     }
            // }
        }
    }
    // render player
    let offset_x = match game.player.state {
        PlayerState::Walking => {
            ((game.player.walking_frames as f32 / WALK_FRAMES as f32) * CELL_SIZE as f32) as i32
                * (if game.player.direction == Direction::Left {
                    -1
                } else {
                    1
                })
        }
        _ => 0,
    };
    canvas.set_draw_color(Color::RGB(0xfa, 0x17, 0x46));
    canvas.fill_rect(Rect::new(
        game.player.p.x * CELL_SIZE + offset_x,
        (game.player.p.y - game.camera_y) * CELL_SIZE,
        CELL_SIZE as u32,
        28,
    ))?;
    canvas.set_draw_color(Color::RGB(0xff, 0xc3, 0x5b));
    canvas.fill_rect(Rect::new(
        game.player.p.x * CELL_SIZE + offset_x + (CELL_SIZE - 28) / 2,
        (game.player.p.y - game.camera_y) * CELL_SIZE + 5,
        28,
        18,
    ))?;
    canvas.set_draw_color(Color::RGB(0x4b, 0xe4, 0xe9));
    canvas.fill_rect(Rect::new(
        game.player.p.x * CELL_SIZE + offset_x + 10,
        (game.player.p.y - game.camera_y) * CELL_SIZE + CELL_SIZE / 2 + 2,
        20,
        18,
    ))?;

    canvas.set_draw_color(Color::RGB(0xd2, 0xcb, 0xbd));
    canvas.fill_rect(Rect::new(
        INFO_X,
        0,
        INFO_WIDTH as u32,
        SCREEN_HEIGHT as u32,
    ))?;

    // render air
    let radius = 30;
    let circle_x = (INFO_X + INFO_WIDTH / 2) as i16;
    let circle_y = 270;
    if game.player.air > 0 {
        // 外側
        canvas.filled_pie(
            circle_x,
            circle_y,
            radius as i16,
            -90,
            -90 + (360.0 * game.player.air_percent() / 100.0f32) as i16,
            Color::RGBA(0x01, 0x2f, 0xd0, 254), // なぜかalpha=255だと他の部分まで半透明が効かなくなってしまう
        )?;
    }
    // 内側の円
    let inner_circle_color = if game.player.air_percent() >= 20.0f32 {
        Color::RGBA(0xd3, 0xe3, 0xe9, 254)
    } else {
        Color::RGBA(0xdf, 0x7a, 0x98, 254)
    };
    canvas.filled_circle(
        circle_x,
        circle_y,
        (radius / 2 - 1) as i16,
        inner_circle_color,
    )?;

    let font = resources.fonts.get_mut("boxfont2.ttf").unwrap();
    let depth = format!("{0: >4}", game.get_depth());
    render_font(
        canvas,
        font,
        depth,
        INFO_X + 5,
        180,
        Color::RGBA(0xfe, 0x54, 0x00, 255),
    );

    if game.is_over {
        canvas.set_draw_color(Color::RGBA(255, 0, 0, 128));
        canvas.fill_rect(Rect::new(
            0,
            0,
            (SCREEN_WIDTH - INFO_WIDTH) as u32,
            SCREEN_HEIGHT as u32,
        ))?;
    }

    if game.is_clear {
        let font = resources.fonts.get_mut("boxfont2.ttf").unwrap();
        render_font(
            canvas,
            font,
            "CLEAR!!".to_string(),
            140,
            240,
            Color::RGBA(255, 255, 0, 255),
        );
    }

    if game.is_debug {
        let font = resources.fonts.get_mut("boxfont2.ttf").unwrap();
        let frame_str = format!("{0: >6}", game.frame);

        canvas.set_draw_color(Color::RGBA(255, 255, 255, 255));
        canvas.fill_rect(Rect::new(0, 0, 50, 16))?;
        render_number(canvas, resources, 0, 0, 1.0, frame_str);
    }

    canvas.present();

    Ok(())
}

fn render_number(
    canvas: &mut Canvas<Window>,
    resources: &Resources,
    x: i32,
    y: i32,
    scale: f32,
    numstr: String,
) {
    let mut x = x;
    let image = resources.images.get("numbers.bmp").unwrap();
    let digit_width_in_px = 8;
    for c in numstr.chars() {
        if 0x30 <= c as i32 && c as i32 <= 0x39 {
            canvas
                .copy(
                    &image.texture,
                    Rect::new(
                        digit_width_in_px * (c as i32 - 0x30),
                        0,
                        digit_width_in_px as u32,
                        image.h,
                    ),
                    Rect::new(
                        x,
                        y,
                        (digit_width_in_px as f32 * scale) as u32,
                        (image.h as f32 * scale) as u32,
                    ),
                )
                .unwrap();
        }
        x += (digit_width_in_px as f32 * scale) as i32;
    }
}

fn render_font(
    canvas: &mut Canvas<Window>,
    font: &sdl2::ttf::Font,
    text: String,
    x: i32,
    y: i32,
    color: Color,
) {
    let texture_creator = canvas.texture_creator();

    let surface = font.render(&text).blended(color).unwrap();
    let texture = texture_creator
        .create_texture_from_surface(&surface)
        .unwrap();
    canvas
        .copy(
            &texture,
            None,
            Rect::new(x, y, texture.query().width, texture.query().height),
        )
        .unwrap();
}

fn play_sounds(game: &mut Game, resources: &Resources) {
    for sound_key in &game.requested_sounds {
        let chunk = resources
            .chunks
            .get(&sound_key.to_string())
            .expect("cannot get sound");
        sdl2::mixer::Channel::all()
            .play(&chunk, 0)
            .expect("cannot play sound");
    }
    game.requested_sounds = Vec::new();
}
