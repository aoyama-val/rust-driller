use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mixer;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::{BlendMode, Canvas, Texture, TextureCreator};
use sdl2::video::{Window, WindowContext};
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, SystemTime};
mod model;
use crate::model::*;

const FPS: u32 = 30;

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

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    canvas.set_blend_mode(BlendMode::Blend);

    let texture_creator = canvas.texture_creator();
    let mut resources = load_resources(&texture_creator, &mut canvas);

    let mut event_pump = sdl_context.event_pump()?;

    let mut game = Game::new();

    println!("Keys:");
    println!("    Left  : Move player left");
    println!("    Right : Move player right");
    println!("    Shift : Dig");
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
                    }
                }
                Event::KeyDown {
                    keycode: Some(code),
                    ..
                } => {
                    command = match code {
                        Keycode::Left => Command::Left,
                        Keycode::Right => Command::Right,
                        Keycode::LShift => Command::Dig,
                        Keycode::RShift => Command::Dig,
                        _ => Command::None,
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
        let frame_duration = Duration::new(0, 1_000_000_000u32 / FPS);
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
) -> Resources<'a> {
    let mut resources = Resources {
        images: HashMap::new(),
        chunks: HashMap::new(),
    };

    let sound_paths = ["crash.wav"];
    for path in sound_paths {
        let full_path = "resources/sound/".to_string() + path;
        let chunk =
            mixer::Chunk::from_file(full_path).expect(&format!("cannot load sound: {}", path));
        resources.chunks.insert(path.to_string(), chunk);
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

    // render player
    canvas.set_draw_color(Color::RGB(0, 0, 255));
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
    canvas.fill_rect(Rect::new(
        game.player.p.x * CELL_SIZE + offset_x,
        (game.player.p.y - game.camera_y) * CELL_SIZE,
        CELL_SIZE as u32,
        CELL_SIZE as u32,
    ))?;

    // render cells
    for x in CELLS_X_MIN..=CELLS_X_MAX {
        for y in 0..12 {
            if y > CELLS_Y_MAX {
                break;
            }
            match game.cells[game.camera_y as usize + y as usize][x as usize].cell_type {
                CellType::None => {}
                CellType::Air => {}
                CellType::Box => {
                    canvas.set_draw_color(Color::RGB(92, 48, 28));
                    canvas.fill_rect(Rect::new(
                        CELL_SIZE as i32 * x,
                        CELL_SIZE as i32 * y,
                        CELL_SIZE as u32,
                        CELL_SIZE as u32,
                    ))?;
                }
                CellType::Red => {
                    canvas.set_draw_color(Color::RGB(255, 128, 128));
                    canvas.fill_rect(Rect::new(
                        CELL_SIZE as i32 * x,
                        CELL_SIZE as i32 * y,
                        CELL_SIZE as u32,
                        CELL_SIZE as u32,
                    ))?;
                }
                CellType::Yellow => {
                    canvas.set_draw_color(Color::RGB(255, 255, 128));
                    canvas.fill_rect(Rect::new(
                        CELL_SIZE as i32 * x,
                        CELL_SIZE as i32 * y,
                        CELL_SIZE as u32,
                        CELL_SIZE as u32,
                    ))?;
                }
                CellType::Green => {
                    canvas.set_draw_color(Color::RGB(128, 255, 128));
                    canvas.fill_rect(Rect::new(
                        CELL_SIZE as i32 * x,
                        CELL_SIZE as i32 * y,
                        CELL_SIZE as u32,
                        CELL_SIZE as u32,
                    ))?;
                }
                CellType::Blue => {
                    canvas.set_draw_color(Color::RGB(128, 128, 255));
                    canvas.fill_rect(Rect::new(
                        CELL_SIZE as i32 * x,
                        CELL_SIZE as i32 * y,
                        CELL_SIZE as u32,
                        CELL_SIZE as u32,
                    ))?;
                }
            }
        }
    }

    canvas.present();

    Ok(())
}

fn render_number(
    canvas: &mut Canvas<Window>,
    resources: &Resources,
    x: i32,
    y: i32,
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
                    Rect::new(x, y, digit_width_in_px as u32, image.h),
                )
                .unwrap();
        }
        x += digit_width_in_px;
    }
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
