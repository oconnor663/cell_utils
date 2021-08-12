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
pub fn array_of_cells<T, const N: usize>(
    cell: &core::cell::Cell<[T; N]>,
) -> &[core::cell::Cell<T>; N] {
    // SAFETY: `Cell<T>` has the same memory layout as `T`.
    unsafe { &*(cell as *const core::cell::Cell<[T; N]> as *const [core::cell::Cell<T>; N]) }
}

/// Given a reference to a [`Cell`](https://doc.rust-lang.org/std/cell/struct.Cell.html) containing
/// a struct or a tuple, return a reference one of the fields or elements of that object:
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
/// If you want to use any expression other than a bare variable name, you need to surround it with
/// an extra set of parentheses:
///
/// ```
/// # use cell_utils::project;
/// # use core::cell::Cell;
/// # struct Foo {
/// #     bar: Bar,
/// # }
/// # struct Bar {
/// #     baz: i32,
/// # }
/// let mut foo = Foo { bar: Bar { baz: 42 } };
/// let baz_cell: &Cell<i32> = project!((Cell::from_mut(&mut foo)).bar.baz);
/// baz_cell.set(99);
/// assert_eq!(foo.bar.baz, 99);
/// ```
///
/// `project!` also supports tuples:
///
/// ```
/// # use cell_utils::project;
/// # use core::cell::Cell;
/// let mut tuple = Cell::new(("foo", "bar", "baz"));
/// project!((&tuple).0).set("hello");
/// assert_eq!(tuple.into_inner().0, "hello");
/// ```
#[macro_export]
macro_rules! project {
    ($e:ident $(. $field:tt)* ) => {
        project!(($e) $(. $field)*)
    };
    (( $e:expr ) $(. $field:tt)* ) => {{
        let cell: &core::cell::Cell<_> = $e;
        // SAFETY: We need this helper function to bind the lifetime of the reference.
        unsafe fn get_mut<T>(cell: &core::cell::Cell<T>) -> &mut T { &mut *cell.as_ptr() }
        let reference = unsafe { get_mut(cell) };
        $( let reference = &mut reference.$field; )*
        core::cell::Cell::from_mut(reference)
    }};
}

// This is a hacky way of using doctests to guarantee that something fails to compile, as described
// in https://stackoverflow.com/a/55327334/823869. Note that these tests tends to be fragile: We
// want them to fail for a specific reason, but new errors might get introduced that make them fail
// for different reasons, and then we'd no longer be testing what we thought we were. I'm not sure
// there's much we can do about that other than occasionally removing these `compile_fail`
// annotations and sanity checking that the errors look like what we expect.
//
// In this case, we're testing lifetime errors. This exercises the lifetime bindings commented on
// in the macro code above.
/// ```compile_fail
/// use std::cell::Cell;
/// use cell_utils::project;
/// let y = {
///     let x = Cell::new(5);
///     // FAIL: This reference outlives x.
///     project!((&x))
/// };
/// y.set(6)
/// ```
fn _compile_fail_test() {}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::Cell;

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
    fn test_project_self() {
        let x = &Cell::new(5);
        // This is the case where we don't actually include any field names.
        let y = project!(x);
        y.set(6);
        assert_eq!(x.get(), 6);
    }

    #[test]
    fn test_project_expr() {
        struct Foo {
            x: i32,
        }
        let mut tuple = (Foo { x: 0 }, Foo { x: 1 });
        project!((Cell::from_mut(&mut tuple)).0.x).set(99);
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
