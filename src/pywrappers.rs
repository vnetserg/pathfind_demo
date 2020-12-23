use py::builtins::tuple::PyTupleRef;
use py::pyobject::{BorrowValue, PyIterable, PyObjectRef, PyResult, TryFromObject};
use rustpython_vm as py;

////////////////////////////////////////////////////////////////////////////////

pub struct PyVecWrapper<T: TryFromObject>(pub Vec<T>);

impl<T: TryFromObject> TryFromObject for PyVecWrapper<T> {
    fn try_from_object(vm: &py::VirtualMachine, obj: PyObjectRef) -> PyResult<Self> {
        let mut vec = vec![];
        for maybe_item in PyIterable::try_from_object(vm, obj)?.iter(vm)? {
            vec.push(T::try_from_object(vm, maybe_item?)?);
        }
        Ok(Self(vec))
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct PyTuple2Wrapper<T: TryFromObject, U: TryFromObject>(pub T, pub U);

impl<T: TryFromObject, U: TryFromObject> TryFromObject for PyTuple2Wrapper<T, U> {
    fn try_from_object(vm: &py::VirtualMachine, obj: PyObjectRef) -> PyResult<Self> {
        let tuple = PyTupleRef::try_from_object(vm, obj)?;
        if tuple.borrow_value().len() != 2 {
            Err(vm.new_type_error("Expected tuple of length 2".to_owned()))
        } else {
            Ok(Self(
                T::try_from_object(vm, tuple.borrow_value()[0].clone())?,
                U::try_from_object(vm, tuple.borrow_value()[1].clone())?,
            ))
        }
    }
}
