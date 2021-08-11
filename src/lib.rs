#![no_std]

use core::cell::Cell;

#[macro_export]
macro_rules! project {
    ($e:ident $(. $field:tt)* ) => {{
        let cell: &core::cell::Cell<_> = $e;
        $(
        let field_mut = unsafe { &mut (*cell.as_ptr()).$field };
        let cell = core::cell::Cell::from_mut(field_mut);
        )*
        cell
    }};
}

pub fn array_of_cells<T, const N: usize>(cell: &Cell<[T; N]>) -> &[Cell<T>; N] {
    // SAFETY: `Cell<T>` has the same memory layout as `T`.
    unsafe { &*(cell as *const Cell<[T; N]> as *const [Cell<T>; N]) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project() {
        struct Foo {
            x: i32,
        }
        let mut tuple = (Foo { x: 0 }, Foo { x: 1 });
        let tuple_cell = Cell::from_mut(&mut tuple);
        project!(tuple_cell.0.x).set(99);
        assert_eq!(tuple.0.x, 99);
    }

    #[test]
    fn test_array_of_cells() {
        let cell = Cell::new([1, 2, 3]);
        let array_of_cells: &[Cell<i32>; 3] = array_of_cells(&cell);
        array_of_cells[0].set(99);
        assert_eq!(cell.into_inner(), [99, 2, 3]);
    }
}
