//! This crate provides two utilities for working with
//! [`std::cell::Cell`](https://doc.rust-lang.org/std/cell/struct.Cell.html), the `array_of_cells`
//! function and the `project!` macro.
//!
//! # Examples
//!
//! ```
//! # use cell_utils::{array_of_cells, project};
//! # use core::cell::Cell;
//! ///
//! /// Convert a reference to a cell-of-array into a reference to an array-of-cells.
//! ///
//!
//! let cell: Cell<[i32; 3]> = Cell::new([1, 2, 3]);
//! let array: &[Cell<i32>; 3] = array_of_cells(&cell);
//! array[0].set(99);
//! assert_eq!(cell.into_inner(), [99, 2, 3]);
//!
//! ///
//! /// Extract references to struct fields inside a cell.
//! /// Note that the extracted references are also cells.
//! ///
//!
//! struct Foo {
//!     bar: Bar,
//! }
//! struct Bar {
//!     baz: i32,
//! }
//! let mut foo = Foo { bar: Bar { baz: 42 } };
//! let foo_cell: &Cell<Foo> = Cell::from_mut(&mut foo);
//! let baz_cell: &Cell<i32> = project!(foo_cell.bar.baz);
//! baz_cell.set(99);
//! assert_eq!(foo.bar.baz, 99);
//! ```

#![no_std]

use core::cell::Cell;

/// Given a reference to a [`Cell`](https://doc.rust-lang.org/std/cell/struct.Cell.html) containing
/// an array, return a reference to an array of cells.
///
/// Internally this is a pointer cast, with no runtime cost.
///
/// This is very similar to the standard
/// [`Cell::as_slice_of_cells`](https://doc.rust-lang.org/std/cell/struct.Cell.html#method.as_slice_of_cells)
/// method.
///
/// # Example
///
/// ```
/// # use cell_utils::array_of_cells;
/// # use core::cell::Cell;
/// let cell: Cell<[i32; 3]> = Cell::new([1, 2, 3]);
/// let array: &[Cell<i32>; 3] = array_of_cells(&cell);
/// array[0].set(99);
/// assert_eq!(cell.into_inner(), [99, 2, 3]);
/// ```
pub fn array_of_cells<T, const N: usize>(cell: &Cell<[T; N]>) -> &[Cell<T>; N] {
    // SAFETY: `Cell<T>` has the same memory layout as `T`.
    unsafe { &*(cell as *const Cell<[T; N]> as *const [Cell<T>; N]) }
}

/// Given a reference to a [`Cell`](https://doc.rust-lang.org/std/cell/struct.Cell.html) containing
/// a struct or a tuple, return a reference one of the fields or elements of that object.
///
/// # Example
///
/// ```
/// # use cell_utils::project;
/// # use core::cell::Cell;
/// struct Foo {
///     bar: Bar,
/// }
/// struct Bar {
///     baz: i32,
/// }
/// let mut foo = Foo { bar: Bar { baz: 42 } };
/// let foo_cell: &Cell<Foo> = Cell::from_mut(&mut foo);
/// let baz_cell: &Cell<i32> = project!(foo_cell.bar.baz);
/// baz_cell.set(99);
/// assert_eq!(foo.bar.baz, 99);
/// ```
///
/// # Example
///
/// This also works with tuples:
///
/// ```
/// # use cell_utils::project;
/// # use core::cell::Cell;
/// let mut tuple = ("foo", "bar", "baz");
/// let tuple_cell: &Cell<_> = Cell::from_mut(&mut tuple);
/// project!(tuple_cell.0).set("hello");
/// assert_eq!(tuple.0, "hello");
/// ```
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
