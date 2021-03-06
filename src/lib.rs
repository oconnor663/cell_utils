//! This crate provides two utilities for working with
//! [`std::cell::Cell`](https://doc.rust-lang.org/std/cell/struct.Cell.html), the `array_of_cells`
//! function and the `project!` macro.
//!
//! # Examples
//!
//! Convert a reference to a cell-of-array into a reference to an array-of-cells:
//! ```
//! # use cell_utils::array_of_cells;
//! # use core::cell::Cell;
//! let cell: Cell<[i32; 3]> = Cell::new([1, 2, 3]);
//! let array: &[Cell<i32>; 3] = array_of_cells(&cell);
//! array[0].set(99);
//! assert_eq!(cell.into_inner(), [99, 2, 3]);
//! ```
//!
//! Extract references to struct fields inside a cell. Note that the extracted references are also
//! cells:
//!
//! ```
//! # use cell_utils::project;
//! # use core::cell::Cell;
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

use core::cell::{Cell, UnsafeCell};

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
/// `project!` will automatically take a reference if needed:
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
/// let foo_cell: Cell<_> = Cell::new(Foo { bar: Bar { baz: 42 } });
/// // Note that foo_cell is not a reference. That's ok.
/// let baz_cell: &Cell<i32> = project!(foo_cell.bar.baz);
/// baz_cell.set(99);
/// assert_eq!(foo_cell.into_inner().bar.baz, 99);
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
        // Add a & automatically. If $e is already a reference, that's fine, because a double
        // reference will get automatically dereferenced below.
        project!((&$e) $(. $field)*)
    };
    (( $e:expr ) $(. $field:tt)* ) => {{
        // If cell is a double reference, this automatically dereferences it.
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

#[repr(transparent)]
pub struct ReadOnlyCell<T>(UnsafeCell<T>);

impl<T> ReadOnlyCell<T> {
    pub fn new(val: T) -> Self {
        Self(UnsafeCell::new(val))
    }

    pub fn from_ref(t: &T) -> &Self {
        // SAFETY: &ReadOnlyCell<T> is strictly less capable than &T
        unsafe { &*(t as *const T as *const Self) }
    }

    pub fn from_cell_ref(t: &Cell<T>) -> &Self {
        // SAFETY: &ReadOnlyCell<T> is strictly less capable than &Cell<T>
        unsafe { &*(t as *const Cell<T> as *const Self) }
    }
}

impl<T: Copy> ReadOnlyCell<T> {
    pub fn get(&self) -> T {
        unsafe { *self.0.get() }
    }
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
    fn test_project_self() {
        let x = &Cell::new(5);
        // This is the case where we don't actually include any field names.
        let y = project!(x);
        y.set(6);
        assert_eq!(x.get(), 6);
    }

    #[test]
    fn test_project_autoref() {
        struct Foo {
            x: i32,
        }
        let tuple = Cell::new((Foo { x: 0 }, Foo { x: 1 }));
        // This automatically takes a reference.
        project!(tuple.0.x).set(99);
        assert_eq!(tuple.into_inner().0.x, 99);
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

    #[test]
    fn test_read_only_cell() {
        let my_int = &mut 42;

        let shared_ref = &*my_int;
        let read_only_cell = ReadOnlyCell::from_ref(shared_ref);
        assert_eq!(read_only_cell.get(), 42);

        *my_int += 1;

        let cell_ref = Cell::from_mut(my_int);
        let read_only_cell = ReadOnlyCell::from_cell_ref(cell_ref);
        assert_eq!(read_only_cell.get(), 43);
        cell_ref.set(44);
        assert_eq!(read_only_cell.get(), 44);
    }
}
