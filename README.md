# `cell_utils`

This Rust crate contains two utilities for working with
[`std::cell::Cell`](https://doc.rust-lang.org/std/cell/struct.Cell.html):
`array_of_cells` and `project!`.

The `array_of_cells` function converts between `&Cell<[T; N]>` and `&[Cell<T>;
N]`. This is very similar to `Cell::as_slice_of_cells`, and I've [proposed
adding it to the standard
library](https://github.com/rust-lang/rust/pull/87944).

```rust
use cell_utils::array_of_cells;

let cell: Cell<[i32; 3]> = Cell::new([1, 2, 3]);
let array: &[Cell<i32>; 3] = array_of_cells(&cell);
array[0].set(99);
assert_eq!(cell.into_inner(), [99, 2, 3]);
```

The `project!` macro lets you access the fields of a struct or the elements of
a tuple, given a reference to a containing cell.

```rust
use cell_utils::project;

struct Foo {
    bar: Bar,
}

struct Bar {
    baz: i32,
}

let mut foo = Foo { bar: Bar { baz: 42 } };
let foo_cell: &Cell<Foo> = Cell::from_mut(&mut foo);
let baz_cell: &Cell<i32> = project!(foo_cell.bar.baz);
baz_cell.set(99);
assert_eq!(foo.bar.baz, 99);
```

The `project!` macro was inspired by the
[`cell-project`](https://crates.io/crates/cell-project) crate.
