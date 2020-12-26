pub use quad_gl::{colors, Color};

use macroquad::prelude as mq;

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

////////////////////////////////////////////////////////////////////////////////

pub struct DrawContext {}

impl DrawContext {
    pub fn draw_rectangle(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        mq::draw_rectangle(x, y, w, h, color);
    }

    pub fn draw_circle(&mut self, x: f32, y: f32, r: f32, color: Color) {
        mq::draw_circle(x, y, r, color);
    }

    pub fn draw_text(&mut self, text: &str, x: f32, y: f32, font_size: f32, color: Color) {
        mq::draw_text(text, x, y, font_size, color);
    }

    pub fn draw_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32, color: Color) {
        mq::draw_line(x1, y1, x2, y2, thickness, color);
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

impl From<MouseButton> for mq::MouseButton {
    fn from(other: MouseButton) -> Self {
        match other {
            MouseButton::Left => mq::MouseButton::Left,
            MouseButton::Right => mq::MouseButton::Right,
            MouseButton::Middle => mq::MouseButton::Middle,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

pub enum Event {
    MouseDown { button: MouseButton, x: f32, y: f32 },
    MouseUp { button: MouseButton, x: f32, y: f32 },
    MouseMoved { x: f32, y: f32 },
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
struct EventTracker {
    scene_width: f32,
    scene_height: f32,
    down_mouse_buttons: HashSet<MouseButton>,
    mouse_position: (f32, f32),
}

impl EventTracker {
    fn generate_events<'a>(&'a mut self) -> impl Iterator<Item = Event> + 'a {
        let (screen_mouse_x, screen_mouse_y) = mq::mouse_position();
        let (mouse_x, mouse_y) = self.translate_coordinates(screen_mouse_x, screen_mouse_y);

        let move_event = if (mouse_x, mouse_y) != self.mouse_position {
            self.mouse_position = (mouse_x, mouse_y);
            Some(Event::MouseMoved {
                x: mouse_x,
                y: mouse_y,
            })
        } else {
            None
        };

        let click_events = [MouseButton::Left, MouseButton::Right, MouseButton::Middle]
            .iter()
            .filter_map(move |&button| {
                let is_down = mq::is_mouse_button_down(button.into());
                let was_down = self.down_mouse_buttons.contains(&button);
                match (is_down, was_down) {
                    (true, false) => {
                        self.down_mouse_buttons.insert(button);
                        Some(Event::MouseDown {
                            button,
                            x: mouse_x,
                            y: mouse_y,
                        })
                    }
                    (false, true) => {
                        self.down_mouse_buttons.remove(&button);
                        Some(Event::MouseUp {
                            button,
                            x: mouse_x,
                            y: mouse_y,
                        })
                    }
                    _ => None,
                }
            });

        click_events.chain(move_event.into_iter())
    }

    fn translate_coordinates(&self, screen_x: f32, screen_y: f32) -> (f32, f32) {
        let screen_width = mq::screen_width();
        let screen_height = mq::screen_height();
        (
            (screen_x / screen_width) * self.scene_width,
            (1. - (screen_y / screen_height)) * self.scene_height,
        )
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy)]
pub struct SceneConfig {
    pub width: f32,
    pub height: f32,
    pub bgcolor: Color,
}

pub trait Scene {
    fn config(&self) -> SceneConfig;
    fn update(&mut self, delta: f32);
    fn draw(&mut self, cx: &mut DrawContext);
    fn handle_event(&mut self, event: Event);
}

////////////////////////////////////////////////////////////////////////////////

pub struct Runner<T: Scene> {
    scene: Rc<RefCell<T>>,
    scene_config: SceneConfig,
    event_tracker: EventTracker,
}

impl<T: Scene> Runner<T> {
    pub fn new(scene: Rc<RefCell<T>>) -> Self {
        let scene_config = scene.borrow().config();

        Self {
            scene,
            scene_config,
            event_tracker: EventTracker {
                scene_width: scene_config.width,
                scene_height: scene_config.height,
                ..Default::default()
            },
        }
    }

    pub async fn run(&mut self) {
        mq::set_camera(mq::Camera2D {
            zoom: mq::vec2(
                1. / self.scene_config.width * 2.,
                1. / self.scene_config.height * 2.,
            ),
            target: mq::vec2(self.scene_config.width / 2., self.scene_config.height / 2.),
            ..Default::default()
        });

        let mut prev_update_time = mq::get_time();

        loop {
            let now = mq::get_time();
            let delta = (now - prev_update_time) as f32;
            prev_update_time = now;

            self.update_and_draw_scene(delta);

            mq::next_frame().await
        }
    }

    fn update_and_draw_scene(&mut self, delta: f32) {
        let mut scene = self.scene.borrow_mut();

        scene.update(delta);

        for event in self.event_tracker.generate_events() {
            scene.handle_event(event);
        }

        mq::clear_background(self.scene_config.bgcolor);
        scene.draw(&mut DrawContext {});
    }
}
