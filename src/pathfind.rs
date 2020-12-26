use crate::grid::Grid;
use crate::pywrappers::{PyTuple2Wrapper, PyVecWrapper};
use crate::scene::{colors, DrawCommand, Shape};

use py::function::IntoFuncArgs;
use py::pyobject::{IntoPyObject, ItemProtocol, TryFromObject, PyResult};
use rustpython_vm as py;

use std::cell::RefCell;
use std::rc::Rc;

////////////////////////////////////////////////////////////////////////////////

pub fn find_and_render_path(
    code: &str,
    grid: &Grid<bool>,
    start: (usize, usize),
    finish: (usize, usize),
) -> Result<Vec<DrawCommand>, String> {
    let (maybe_path, mut draw_commands) = find_path(code, grid, start, finish)?;
    maybe_path.map(|path| {
        draw_commands.push(DrawCommand::Clear);
        draw_commands.push(DrawCommand::AddShape(Shape::SegmentedLine {
            points: path,
            width: 5.,
            color: colors::LIME,
        }));
    });
    Ok(draw_commands)
}

////////////////////////////////////////////////////////////////////////////////

pub fn find_path(
    code: &str,
    grid: &Grid<bool>,
    start: (usize, usize),
    finish: (usize, usize),
) -> Result<(Option<Vec<(usize, usize)>>, Vec<DrawCommand>), String> {
    py::Interpreter::default().enter(|vm| {
        try_find_path(vm, code, grid, start, finish)
            .map_err(|err| {
                let mut traceback = Vec::<u8>::new();
                py::exceptions::write_exception(&mut traceback, vm, &err)
                    .expect("failed to write exception");
                String::from_utf8(traceback).expect("traceback is not utf-8")
            })
    })
}

fn try_find_path(
    vm: &py::VirtualMachine,
    code: &str,
    grid: &Grid<bool>,
    start: (usize, usize),
    finish: (usize, usize),
) -> PyResult<(Option<Vec<(usize, usize)>>, Vec<DrawCommand>)> {
    let code_obj = py::compile::compile(
        code,
        py::compile::Mode::Exec,
        "<embedded>".to_owned(),
        py::compile::CompileOpts::default(),
    ).map_err(|err| vm.new_syntax_error(&err))?;

    let (scope, commands) = prepare_scope(vm)?;

    let py_path = run_python_code(code_obj, vm, grid, scope, start, finish)?;

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

    Ok((maybe_path, Rc::try_unwrap(commands).unwrap().into_inner()))
}

fn prepare_scope(vm: &py::VirtualMachine) -> PyResult<(py::scope::Scope, Rc<RefCell<Vec<DrawCommand>>>)> {
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
    )?;

    Ok((scope, commands))
}

fn run_python_code(
    code_obj: py::bytecode::CodeObject,
    vm: &py::VirtualMachine,
    grid: &Grid<bool>,
    scope: py::scope::Scope,
    start: (usize, usize),
    finish: (usize, usize),
) -> py::pyobject::PyResult {
    let code_obj = vm.new_code_object(code_obj.clone());
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
