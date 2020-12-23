use crate::grid::Grid;
use crate::scene::{colors, Shape, DrawCommand};

use rustpython_vm as py;
use py::pyobject::{TryFromObject, IntoPyObject, ItemProtocol};
use py::function::IntoFuncArgs;

use std::collections::HashMap;

////////////////////////////////////////////////////////////////////////////////

pub trait PathfindAlgorithm {
    fn find_path(
        &self,
        grid: &Grid<bool>,
        start: (usize, usize),
        finish: (usize, usize),
    ) -> (Option<Vec<(usize, usize)>>, Vec<DrawCommand>);
}

////////////////////////////////////////////////////////////////////////////////

pub fn find_and_render_path<T: PathfindAlgorithm>(
    algo: &T,
    grid: &Grid<bool>,
    start: (usize, usize),
    finish: (usize, usize),
) -> Vec<DrawCommand> {
    let (maybe_path, mut draw_commands) = algo.find_path(grid, start, finish);
    maybe_path.map(|path| {
        draw_commands.push(DrawCommand::Clear);
        draw_commands.push(
            DrawCommand::AddShape(Shape::SegmentedLine {
                points: path,
                width: 5.,
                color: colors::LIME,
            }),
        );
    });
    draw_commands
}

////////////////////////////////////////////////////////////////////////////////

pub struct Dfs {}

impl PathfindAlgorithm for Dfs {
    fn find_path(
        &self,
        grid: &Grid<bool>,
        start: (usize, usize),
        finish: (usize, usize),
    ) -> (Option<Vec<(usize, usize)>>, Vec<DrawCommand>) {
        let tree_color = colors::DARKGREEN;

        if start == finish {
            return (Some(vec![start]), vec![]);
        }

        let mut draw_commands = vec![];
        let mut stack = vec![start];
        let mut prev = HashMap::new();
        prev.insert(start, start);

        'main_loop: while let Some((x, y)) = stack.pop() {
            for (nx, ny, is_occupied) in grid.neighbors(x, y) {
                if is_occupied || prev.contains_key(&(nx, ny)) {
                    continue;
                }

                prev.insert((nx, ny), (x, y));
                draw_commands.push(DrawCommand::AddShape(Shape::Line {
                    from: (x, y),
                    to: (nx, ny),
                    width: 5.,
                    color: tree_color,
                }));

                if (nx, ny) == finish {
                    break 'main_loop;
                }
                stack.push((nx, ny));
            }
        }

        let maybe_path = if prev.contains_key(&finish) {
            let mut path = vec![finish];
            while path.last().unwrap() != &start {
                let parent = prev.get(path.last().unwrap()).unwrap();
                path.push(*parent);
            }
            (&mut path).reverse();
            Some(path)
        } else {
            None
        };

        (maybe_path, draw_commands)
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct PythonPathfind {
    code_obj: py::bytecode::CodeObject,
}

impl PythonPathfind {
    pub fn new(py_source: &str) -> Result<Self, py::compile::CompileError> {
        let compile_res = py::compile::compile(
            py_source,
            py::compile::Mode::Exec,
            "<embedded>".to_owned(),
            py::compile::CompileOpts::default(),
        );
        compile_res.map(|code_obj| Self { code_obj })
    }
}

impl PathfindAlgorithm for PythonPathfind {
    fn find_path(
        &self,
        grid: &Grid<bool>,
        start: (usize, usize),
        finish: (usize, usize),
    ) -> (Option<Vec<(usize, usize)>>, Vec<DrawCommand>) {
        py::Interpreter::default().enter(|vm| {
            let scope = vm.new_scope_with_builtins();

            let code_obj = vm.new_code_object(self.code_obj.clone());
            vm.run_code_obj(code_obj, scope.clone()).unwrap();

            let py_grid = grid.clone().into_pyobject(vm);

            let find_path_item = scope.globals.get_item("find_path", vm).unwrap();
            let find_path_func = find_path_item.downcast::<py::builtins::PyFunction>().unwrap();

            let py_path = py::slots::Callable::call(&find_path_func, (py_grid,).into_args(vm), vm).unwrap();

            // TODO: fix
            let path: usize = TryFromObject::try_from_object(vm, py_path).unwrap();

            (Some(vec![start, (path, path)]), vec![])
        })
    }
}

impl Default for PythonPathfind {
    fn default() -> Self {
        Self::new("def find_path(grid): return 3").unwrap()
    }
}
