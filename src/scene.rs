pub use crate::runner::{colors, Color};

use crate::grid::Grid;
use crate::runner::{DrawContext, Event, MouseButton, Scene, SceneConfig};
use crate::pathfind::{find_and_render_path, PythonPathfind};

////////////////////////////////////////////////////////////////////////////////

pub enum DrawCommand {
    AddShape(Shape),
    Clear,
}

pub enum Shape {
    Square {
        x: usize,
        y: usize,
        color: Color,
    },
    Line {
        from: (usize, usize),
        to: (usize, usize),
        width: f32,
        color: Color,
    },
    SegmentedLine {
        points: Vec<(usize, usize)>,
        width: f32,
        color: Color,
    },
}

////////////////////////////////////////////////////////////////////////////////

enum PointerMode {
    Noop,
    Set,
    Erase,
}

pub struct PathtfindScene {
    grid: Grid<bool>,
    start: (usize, usize),
    finish: (usize, usize),
    active_cell: Option<(usize, usize)>,
    pointer_mode: PointerMode,
    draw_commands: Vec<DrawCommand>,
    animation_progress: f32,
}

impl PathtfindScene {
    pub fn new(grid: Grid<bool>, start: (usize, usize), finish: (usize, usize)) -> Self {
        Self {
            grid,
            start,
            finish,
            active_cell: None,
            pointer_mode: PointerMode::Noop,
            draw_commands: vec![],
            animation_progress: 0.,
        }
    }

    fn fill_cell(&self, x: usize, y: usize, color: Color, cx: &mut DrawContext) {
        let (center_x, center_y) = self.get_cell_center(x, y);
        cx.draw_rectangle(
            center_x - 50.,
            center_y - 50.,
            100.,
            100.,
            color,
        );
    }

    fn mark_cell(&self, x: usize, y: usize, color: Color, cx: &mut DrawContext) {
        let (center_x, center_y) = self.get_cell_center(x, y);
        cx.draw_circle(center_x, center_y, 45., color);
    }

    fn draw_animation(&self, cx: &mut DrawContext) {
        let end = self
            .draw_commands
            .len()
            .min(self.animation_progress as usize);
        let start = self.draw_commands[..end].iter()
            .enumerate()
            .rfind(|(_, cmd)| matches!(cmd, DrawCommand::Clear))
            .map(|(i, _)| i + 1)
            .unwrap_or(0);
        for cmd in &self.draw_commands[start..end] {
            match cmd {
                &DrawCommand::AddShape(Shape::Square { x, y, color }) => {
                    self.fill_cell(x, y, color, cx);
                }
                &DrawCommand::AddShape(Shape::Line { from, to, width, color }) => {
                    let (x1, y1) = self.get_cell_center(from.0, from.1);
                    let (x2, y2) = self.get_cell_center(to.0, to.1);
                    cx.draw_line(x1, y1, x2, y2, width, color);
                }
                &DrawCommand::AddShape(Shape::SegmentedLine { ref points, width, color }) => {
                    for (from, to) in points.iter().zip(points.iter().skip(1)) {
                        let (x1, y1) = self.get_cell_center(from.0, from.1);
                        let (x2, y2) = self.get_cell_center(to.0, to.1);
                        cx.draw_line(x1, y1, x2, y2, width, color);
                    }
                }
                &DrawCommand::Clear => unreachable!(),
            }
        }
    }

    fn draw_bars(&self, color: Color, cx: &mut DrawContext) {
        let config = self.config();
        for x in 0..self.grid.width() + 1 {
            cx.draw_rectangle(x as f32 * 100., 0., 5., config.height, color);
        }
        for y in 0..self.grid.height() + 1 {
            cx.draw_rectangle(0., y as f32 * 100., config.width, 5., color);
        }
    }

    fn get_cell_coordinates(&self, scene_x: f32, scene_y: f32) -> (i32, i32) {
        (
            ((scene_x + 2.5) / 100.) as i32,
            ((scene_y + 2.5) / 100.) as i32,
        )
    }

    fn get_cell_center(&self, cell_x: usize, cell_y: usize) -> (f32, f32) {
        (
            52.5 + cell_x as f32 * 100.,
            52.5 + cell_y as f32 * 100.,
        )
    }

    fn apply_pointer_action(&mut self, x: usize, y: usize) {
        if (x, y) == self.start || (x, y) == self.finish {
            return;
        }
        match self.pointer_mode {
            PointerMode::Set => self.grid.set(x, y, true),
            PointerMode::Erase => self.grid.set(x, y, false),
            PointerMode::Noop => (),
        }
    }
}

impl Scene for PathtfindScene {
    fn config(&self) -> SceneConfig {
        let width = self.grid.width() * 100;
        let height = self.grid.height() * 100;
        SceneConfig {
            width: 5. + width as f32,
            height: 5. + height as f32,
            bgcolor: colors::LIGHTGRAY,
        }
    }

    fn update(&mut self, delta: f32) {
        if self.animation_progress < self.draw_commands.len() as f32 {
            self.animation_progress += 50. * delta;
        }
    }

    fn draw(&mut self, cx: &mut DrawContext) {
        for (x, y, value) in self.grid.iter() {
            if value {
                self.fill_cell(x, y, colors::GRAY, cx);
            }
        }
        if let Some((x, y)) = self.active_cell {
            let color = Color::new(1., 1., 1., 0.25);
            self.fill_cell(x, y, color, cx);
        }
        self.draw_bars(colors::WHITE, cx);
        self.draw_animation(cx);
        self.mark_cell(self.start.0, self.start.1, colors::DARKGREEN, cx);
        self.mark_cell(self.finish.0, self.finish.1, colors::DARKBLUE, cx);
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::MouseDown {
                button: MouseButton::Left,
                x: mouse_x,
                y: mouse_y,
            } => {
                let (x, y) = self.get_cell_coordinates(mouse_x, mouse_y);
                match self.grid.try_get(x, y) {
                    Some(value) => {
                        self.pointer_mode = if value {
                            PointerMode::Erase
                        } else {
                            PointerMode::Set
                        };
                        self.apply_pointer_action(x as usize, y as usize);
                    }
                    None => self.pointer_mode = PointerMode::Noop,
                }
            }
            Event::MouseUp {
                button: MouseButton::Left,
                ..
            } => {
                self.pointer_mode = PointerMode::Noop;
            }
            Event::MouseUp {
                button: MouseButton::Right,
                ..
            } => {
                self.draw_commands = find_and_render_path(
                    &PythonPathfind::default(),
                    &self.grid,
                    self.start,
                    self.finish,
                );
                self.animation_progress = 0.;
            }
            Event::MouseMoved {
                x: mouse_x,
                y: mouse_y,
            } => {
                let (x, y) = self.get_cell_coordinates(mouse_x, mouse_y);
                if self.grid.are_coordinates_valid(x, y) {
                    self.active_cell = Some((x as usize, y as usize));
                    self.apply_pointer_action(x as usize, y as usize);
                }
            }
            _ => (),
        }
    }
}
