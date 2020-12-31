pub mod grid;
pub mod pathfind;
pub mod pywrappers;
pub mod runner;
pub mod scene;

#[cfg(target_arch = "wasm32")]
pub mod ui;

use grid::Grid;
use runner::Runner;
use scene::PathtfindScene;

use std::rc::Rc;
use std::cell::RefCell;

#[macroquad::main("PathfindDemo")]
async fn main() {
    let grid = Grid::<bool>::new(20, 20);
    let scene = Rc::new(RefCell::new(PathtfindScene::new(grid, (0, 0), (19, 19))));

    #[cfg(target_arch = "wasm32")]
    ui::init(scene.clone());

    let mut runner = Runner::new(scene);
    runner.run().await
}
