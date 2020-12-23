use rustpython_vm::{VirtualMachine, pyobject::{IntoPyObject, PyObjectRef}};

#[derive(Clone)]
pub struct Grid<T> {
    width: usize,
    height: usize,
    values: Vec<T>,
}

impl<T: Default + Copy> Grid<T> {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            values: vec![T::default(); width * height],
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn set(&mut self, x: usize, y: usize, value: T) {
        self.values[y * self.width + x] = value;
    }

    pub fn get(&self, x: usize, y: usize) -> T {
        self.values[y * self.width + x]
    }

    pub fn try_get<U>(&self, x: U, y: U) -> Option<T>
    where
        i64: From<U>,
        U: Copy,
    {
        if self.are_coordinates_valid(x, y) {
            Some(self.get(i64::from(x) as usize, i64::from(y) as usize))
        } else {
            None
        }
    }

    pub fn are_coordinates_valid<U>(&self, x: U, y: U) -> bool
    where
        i64: From<U>,
    {
        let x: i64 = x.into();
        let y: i64 = y.into();
        x >= 0 && (x as usize) < self.width && y >= 0 && (y as usize) < self.height
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = (usize, usize, T)> + 'a {
        self.keys().map(move |(x, y)| (x, y, self.get(x, y)))
    }

    pub fn keys<'a>(&'a self) -> impl Iterator<Item = (usize, usize)> + 'a {
        (0..self.width)
            .map(move |x| (0..self.height).map(move |y| (x, y)))
            .flatten()
    }

    pub fn neighbors<'a>(
        &'a self,
        x: usize,
        y: usize,
    ) -> impl Iterator<Item = (usize, usize, T)> + 'a {
        const DELTAS: [(i64, i64); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];

        DELTAS.iter().filter_map(move |(dx, dy)| {
            let (nx, ny) = (x as i64 + dx, y as i64 + dy);
            self.try_get(nx, ny)
                .map(|value| (nx as usize, ny as usize, value))
        })
    }
}

impl<T: IntoPyObject + Default + Copy> IntoPyObject for Grid<T> {
    fn into_pyobject(self, vm: &VirtualMachine) -> PyObjectRef {
        vm.ctx.new_list(
            (0..self.width).map(|x| {
                vm.ctx.new_list(
                    (0..self.height)
                        .map(|y| self.get(x, y).into_pyobject(vm))
                        .collect()
                )
            }).collect()
        )
    }
}
