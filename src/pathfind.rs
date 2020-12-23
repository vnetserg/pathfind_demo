use crate::grid::Grid;
use crate::pywrappers::{PyTuple2Wrapper, PyVecWrapper};
use crate::scene::{colors, DrawCommand, Shape};

use py::function::IntoFuncArgs;
use py::pyobject::{IntoPyObject, ItemProtocol, TryFromObject};
use rustpython_vm as py;

use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;

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
        draw_commands.push(DrawCommand::AddShape(Shape::SegmentedLine {
            points: path,
            width: 5.,
            color: colors::LIME,
        }));
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

    fn prepare_scope(&self, vm: &py::VirtualMachine) -> (py::scope::Scope, Rc<RefCell<Vec<DrawCommand>>>) {
        let scope = vm.new_scope_with_builtins();

        let commands = Rc::new(RefCell::new(vec![]));

        let commands_inner = Rc::downgrade(&commands);
        scope.globals.set_item(
            "draw_line",
            vm.ctx.new_function(
                "draw_line",
                move |from: PyTuple2Wrapper<usize, usize>, to: PyTuple2Wrapper<usize, usize>| {
                    let PyTuple2Wrapper(x0, y0) = from;
                    let PyTuple2Wrapper(x1, y1) = to;
                    commands_inner
                        .upgrade()
                        .unwrap()
                        .borrow_mut()
                        .push(DrawCommand::AddShape(Shape::Line {
                            from: (x0, y0),
                            to: (x1, y1),
                            width: 5.,
                            color: colors::DARKGREEN,
                        }));
                }
            ),
            vm,
        ).unwrap();

        (scope, commands)
    }

    fn run_python_code(
        &self,
        vm: &py::VirtualMachine,
        grid: &Grid<bool>,
        scope: py::scope::Scope,
        start: (usize, usize),
        finish: (usize, usize),
    ) -> py::pyobject::PyResult {
        let code_obj = vm.new_code_object(self.code_obj.clone());
        vm.run_code_obj(code_obj, scope.clone())?;

        let py_grid = grid.clone().into_pyobject(vm);

        let find_path_item = scope.globals.get_item("find_path", vm)?;
        let find_path_func = find_path_item.downcast::<py::builtins::PyFunction>()
            .map_err(|_| vm.new_type_error("Expected 'find_path' to be a function".to_owned()))?;

        py::slots::Callable::call(
            &find_path_func,
            (py_grid, start, finish).into_args(vm),
            vm,
        )
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
            let (scope, commands) = self.prepare_scope(vm);

            let py_path = self.run_python_code(vm, grid, scope, start, finish).unwrap();

            let maybe_path = Option::<PyVecWrapper::<PyTuple2Wrapper<usize, usize>>>::try_from_object(vm, py_path)
                .map(|maybe_vec| {
                    maybe_vec.map(|vec_wrapper| {
                        vec_wrapper
                            .0
                            .into_iter()
                            .map(|tuple_wrapper| (tuple_wrapper.0, tuple_wrapper.1))
                            .collect()
                    })
                })
                .unwrap();

            (maybe_path, Rc::try_unwrap(commands).unwrap().into_inner())
        })
    }
}

impl Default for PythonPathfind {
    fn default() -> Self {
        Self::new(r#"\

from collections import deque

def find_path(grid, start, finish):
    if start == finish:
        return [start]

    width = len(grid)
    height = len(grid[0])

    prev = {start: start}
    queue = deque([start])

    deltas = [
        (1, 0),
        (0, 1),
        (-1, 0),
        (0, -1),
    ]

    while queue:
        x, y = queue.popleft()
        for (dx, dy) in deltas:
            nx, ny = (x + dx, y + dy)
            if 0 <= nx < width and 0 <= ny < height and not grid[nx][ny] and (nx, ny) not in prev:
                prev[(nx, ny)] = (x, y)
                draw_line((x, y), (nx, ny))

                if (nx, ny) == finish:
                    path = [(nx, ny)]
                    while path[-1] != start:
                        path.append(prev[path[-1]])
                    return list(reversed(path))

                queue.append((nx, ny))

    return None
"#

        ).unwrap()
    }
}
