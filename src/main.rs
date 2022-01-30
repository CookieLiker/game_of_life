extern crate glutin_window;
extern crate graphics;
extern crate opengl_graphics;
extern crate piston;

use glutin_window::GlutinWindow as Window;
use graphics::*;
use opengl_graphics::{GlGraphics, OpenGL};
use piston::event_loop::{EventSettings, Events};
use piston::input::{RenderArgs, RenderEvent, UpdateArgs, UpdateEvent};
use piston::window::WindowSettings;
use piston::*;
use rand::Rng;
use std::collections::LinkedList;

struct Vec2 {
    x: f64,
    y: f64,
}

struct Vec2i {
    x: i32,
    y: i32,
}

struct MouseInfo {
    left_pressed: bool,
    right_pressed: bool,
    position: Vec2,
}

const GRID_SIZE: Vec2i = Vec2i { x: 128, y: 72 };
const WINDOW_SCALE: Vec2i = Vec2i { x: 10, y: 10 };
const UPDATE_TIME_IN_SECONDS: f64 = 1.0 / 15.0;
const INITIAL_DISTRIBUTION_PROBABILITY: f64 = 0.5;

struct Board {
    grid: [bool; (GRID_SIZE.x * GRID_SIZE.y) as usize],
    timer: f64,
    dead_list: LinkedList<Vec2i>,
    alive_list: LinkedList<Vec2i>,
}

impl Board {
    fn new() -> Board {
        let mut board = Board {
            grid: [false; (GRID_SIZE.x * GRID_SIZE.y) as usize],
            timer: 0.0,
            dead_list: LinkedList::new(),
            alive_list: LinkedList::new(),
        };

        board
            .grid
            .fill_with(|| rand::thread_rng().gen_bool(INITIAL_DISTRIBUTION_PROBABILITY));

        board
    }

    fn set_cell(&mut self, coord: &Vec2i, alive: bool) {
        self.grid[(coord.y * GRID_SIZE.x + coord.x) as usize] = alive;
    }

    fn get_cell(&self, coord: &Vec2i) -> bool {
        self.grid[(coord.y * GRID_SIZE.x + coord.x) as usize]
    }

    fn get_alive_neighbors(&self, coord: &Vec2i) -> u32 {
        let mut alive_neighbors = 0;

        for y in -1..=1 {
            for x in -1..=1 {
                if x == 0 && y == 0 {
                    continue;
                }

                let cell_coord = Vec2i {
                    x: (coord.x + x).rem_euclid(GRID_SIZE.x),
                    y: (coord.y + y).rem_euclid(GRID_SIZE.y),
                };

                if self.get_cell(&cell_coord) {
                    alive_neighbors += 1;
                }
            }
        }

        alive_neighbors
    }

    fn update(&mut self, args: &UpdateArgs) {
        self.timer += args.dt;
        if self.timer <= UPDATE_TIME_IN_SECONDS {
            return;
        }

        for y in 0..GRID_SIZE.y {
            for x in 0..GRID_SIZE.x {
                let coord = Vec2i { x, y };
                let alive = self.get_cell(&coord);
                let live_neighbors = self.get_alive_neighbors(&coord);

                if alive {
                    match live_neighbors {
                        // death by underpopulation
                        0..=1 => self.dead_list.push_back(coord),
                        // stable population
                        2..=3 => {}
                        // death by overpopulation
                        4..=8 => self.dead_list.push_back(coord),
                        // not possible without bugs
                        _ => panic!("Undefined number of live neighbors, most likely a bug!"),
                    }
                } else {
                    // Any dead cell with exactly three live neighbours becomes a live cell, as if by reproduction
                    if live_neighbors == 3 {
                        self.alive_list.push_back(coord);
                    }
                }
            }
        }

        // Flush all the dead cells
        loop {
            match self.dead_list.pop_front() {
                Some(coord) => self.set_cell(&coord, false),
                None => break,
            }
        }

        // Birth all the new cells
        loop {
            match self.alive_list.pop_front() {
                Some(coord) => self.set_cell(&coord, true),
                None => break,
            }
        }

        self.timer -= UPDATE_TIME_IN_SECONDS;
    }

    fn draw(&self, c: &Context, gl: &mut GlGraphics) {
        for y in 0..GRID_SIZE.y {
            for x in 0..GRID_SIZE.x {
                if !self.get_cell(&Vec2i { x, y }) {
                    continue;
                }

                let r = rectangle::rectangle_by_corners(
                    0.0,
                    0.0,
                    WINDOW_SCALE.x as f64,
                    WINDOW_SCALE.y as f64,
                );

                let transform = c
                    .transform
                    .trans((x * WINDOW_SCALE.x) as f64, (y * WINDOW_SCALE.y) as f64);

                rectangle(color::WHITE, r, transform, gl);
            }
        }
    }
}

pub struct App {
    gl: GlGraphics, // OpenGL drawing backend.
    board: Board,
    paused: bool,
}

impl App {
    fn render(&mut self, args: &RenderArgs) {
        use graphics::*;

        self.gl.draw(args.viewport(), |c, gl| {
            // Clear the screen.
            clear(color::BLACK, gl);

            // Draw the grid
            self.board.draw(&c, gl);
        });
    }

    fn update(&mut self, args: &UpdateArgs, mouse_info: &MouseInfo) {
        let mouse_pos = Vec2i {
            x: mouse_info.position.x as i32 / WINDOW_SCALE.x,
            y: mouse_info.position.y as i32 / WINDOW_SCALE.y,
        };

        if mouse_info.left_pressed {
            self.board.set_cell(&mouse_pos, true);
        } else if mouse_info.right_pressed {
            self.board.set_cell(&mouse_pos, false);
        }

        if self.paused {
            return;
        }

        self.board.update(args);
    }
}

fn main() {
    let opengl = OpenGL::V3_2;
    let mut window: Window = WindowSettings::new(
        "The Game of Life",
        [
            (GRID_SIZE.x * WINDOW_SCALE.x) as u32,
            (GRID_SIZE.y * WINDOW_SCALE.y) as u32,
        ],
    )
    .graphics_api(opengl)
    .exit_on_esc(true)
    .resizable(false)
    .build()
    .unwrap();

    let mut app = App {
        gl: GlGraphics::new(opengl),
        board: Board::new(),
        paused: false,
    };

    let mut mouse_info = MouseInfo {
        left_pressed: false,
        right_pressed: false,
        position: Vec2 { x: 0.0, y: 0.0 },
    };

    let mut events = Events::new(EventSettings::new());
    while let Some(e) = events.next(&mut window) {
        if let Some(args) = e.render_args() {
            app.render(&args);
        }

        if let Some(args) = e.update_args() {
            app.update(&args, &mouse_info);
        }

        match e.press_args() {
            Some(Button::Keyboard(Key::Space)) => app.paused = !app.paused,
            Some(Button::Mouse(MouseButton::Left)) => mouse_info.left_pressed = true,
            Some(Button::Mouse(MouseButton::Right)) => mouse_info.right_pressed = true,
            _ => {}
        }

        match e.release_args() {
            Some(Button::Mouse(MouseButton::Left)) => mouse_info.left_pressed = false,
            Some(Button::Mouse(MouseButton::Right)) => mouse_info.right_pressed = false,
            _ => {}
        }

        if let Some(mouse_pos) = e.mouse_cursor_args() {
            mouse_info.position.x = mouse_pos[0];
            mouse_info.position.y = mouse_pos[1];
        }
    }
}
