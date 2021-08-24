use std::cell::{Cell, UnsafeCell};
use std::mem;
use std::ptr;

struct OnDrop<F: FnMut()>(F);

impl<F: FnMut()> Drop for OnDrop<F> {
    fn drop(&mut self) {
        self.0();
    }
}

thread_local! {
    static WITH_STACK: Cell<*const WithStackEntry> = Cell::new(ptr::null());
}

#[derive(Copy, Clone)]
struct WithStackEntry {
    cell_address: usize,
    next: *const WithStackEntry,
}

#[repr(transparent)]
pub struct WithCell<T>(UnsafeCell<T>);

impl<T> WithCell<T> {}

impl<T> WithCell<T> {
    pub fn new(t: T) -> Self {
        Self(UnsafeCell::new(t))
    }

    pub fn with<U>(&self, f: impl FnOnce(&T) -> U) -> U {
        WITH_STACK.with(|stack| {
            let previous_head = stack.get();
            let new_head = WithStackEntry {
                cell_address: self as *const Self as usize,
                next: previous_head,
            };
            stack.set(&new_head);
            let _on_drop = OnDrop(|| stack.set(previous_head));
            unsafe { f(&*self.0.get()) }
        })
    }

    fn assert_not_in_stack(&self) {
        let self_address = self as *const Self as usize;
        WITH_STACK.with(|stack| {
            let mut entry_ptr = stack.get();
            while let Some(entry) = unsafe { entry_ptr.as_ref() } {
                assert_ne!(self_address, entry.cell_address);
                entry_ptr = entry.next;
            }
        });
    }

    pub fn replace(&self, t: T) -> T {
        self.assert_not_in_stack();
        unsafe { mem::replace(&mut *self.0.get(), t) }
    }

    pub fn set(&self, t: T) {
        self.replace(t);
    }

    pub fn swap(&self, other: &Self) {
        if ptr::eq(self, other) {
            return;
        }
        self.assert_not_in_stack();
        other.assert_not_in_stack();
        unsafe {
            mem::swap(&mut *self.0.get(), &mut *other.0.get());
        }
    }
}

impl<T: Default> WithCell<T> {
    pub fn take(&self) -> T {
        self.replace(T::default())
    }
}
