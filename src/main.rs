pub mod grid;
pub mod pathfind;
pub mod runner;
pub mod scene;

use grid::Grid;
use runner::Runner;
use scene::PathtfindScene;

#[macroquad::main("PathfindDemo")]
async fn main() {
    let grid = Grid::<bool>::new(20, 20);
    let scene = PathtfindScene::new(grid, (0, 0), (19, 19));
    let mut runner = Runner::new(scene);
    runner.run().await
}
