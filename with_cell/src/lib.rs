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
    static BORROW_STACK: Cell<*const BorrowEntry> = Cell::new(ptr::null());
}

#[derive(Copy, Clone)]
struct BorrowEntry {
    cell_address: usize,
    next: *const BorrowEntry,
}

#[repr(transparent)]
pub struct WithCell<T>(UnsafeCell<T>);

impl<T> WithCell<T> {
    pub fn new(t: T) -> Self {
        Self(UnsafeCell::new(t))
    }

    pub fn from_mut(t: &mut T) -> &Self {
        unsafe { &*(t as *mut T as *mut Self) }
    }

    pub fn into_inner(self) -> T {
        self.0.into_inner()
    }

    pub fn with<U>(&self, f: impl FnOnce(&T) -> U) -> U {
        BORROW_STACK.with(|stack| {
            let previous_head = stack.get();
            let new_head = BorrowEntry {
                cell_address: self as *const Self as usize,
                next: previous_head,
            };
            stack.set(&new_head);
            let _on_drop = OnDrop(|| stack.set(previous_head));
            unsafe { f(&*self.0.get()) }
        })
    }

    fn assert_not_borrowed(&self) {
        let self_address = self as *const Self as usize;
        BORROW_STACK.with(|stack| {
            let mut entry_ptr = stack.get();
            while let Some(entry) = unsafe { entry_ptr.as_ref() } {
                assert_ne!(self_address, entry.cell_address, "address is borrowed");
                entry_ptr = entry.next;
            }
        });
    }

    pub fn replace(&self, t: T) -> T {
        self.assert_not_borrowed();
        unsafe { mem::replace(&mut *self.0.get(), t) }
    }

    pub fn set(&self, t: T) {
        self.replace(t);
    }

    pub fn swap(&self, other: &Self) {
        if ptr::eq(self, other) {
            return;
        }
        self.assert_not_borrowed();
        other.assert_not_borrowed();
        unsafe {
            mem::swap(&mut *self.0.get(), &mut *other.0.get());
        }
    }
}

impl<T: Copy> WithCell<T> {
    pub fn get(&self) -> T {
        unsafe { *self.0.get() }
    }
}

impl<T: Clone> WithCell<T> {
    // It seems more useful to return T than to actually implement Clone and return WithCell<T>?
    // Callers can convert between T and WithCell<T> freely, though, so it's not a huge deal either
    // way. Feedback needed.
    pub fn clone(&self) -> T {
        self.with(|t| t.clone())
    }
}

impl<T: Default> WithCell<T> {
    pub fn take(&self) -> T {
        self.replace(T::default())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_set_get() {
        let x = WithCell::new(0);
        assert_eq!(0, x.get());
        x.set(1);
        assert_eq!(1, x.get());
    }

    #[test]
    fn test_replace() {
        let x = WithCell::new(0);
        assert_eq!(0, x.replace(1));
        assert_eq!(1, x.replace(2));
        assert_eq!(2, x.into_inner());
    }

    #[test]
    fn test_swap() {
        let x = WithCell::new(0);
        let y = WithCell::new(1);
        x.swap(&y);
        assert_eq!(1, x.get());
        assert_eq!(0, y.get());
    }

    #[test]
    #[should_panic]
    fn test_replace_panic() {
        let x = WithCell::new(0);
        x.with(|_| {
            x.set(1);
        });
    }

    #[test]
    #[should_panic]
    fn test_replace_panic_nested() {
        let x = WithCell::new(0);
        let y = WithCell::new(0);
        let z = WithCell::new(0);
        x.with(|_| {
            y.with(|_| {
                z.with(|_| {
                    y.set(1);
                });
            });
        });
    }

    #[test]
    #[should_panic]
    fn test_swap_panic_left() {
        let x = WithCell::new(0);
        x.with(|_| {
            x.swap(&WithCell::new(1));
        });
    }

    #[test]
    #[should_panic]
    fn test_swap_panic_right() {
        let x = WithCell::new(0);
        x.with(|_| {
            WithCell::new(0).swap(&x);
        });
    }

    #[test]
    fn test_swap_self_doesnt_panic() {
        let x = WithCell::new(0);
        x.with(|_| {
            x.swap(&x);
            assert_eq!(0, x.get());
        });
    }

    #[test]
    fn test_take() {
        let x = WithCell::new(String::from("foo"));
        x.with(|s| assert_eq!(s, "foo"));
        assert_eq!(x.take(), "foo");
        x.with(|s| assert_eq!(s, ""));
    }

    #[test]
    fn test_from_mut() {
        let mut s = String::from("foo");
        let c1 = WithCell::from_mut(&mut s);
        let c2 = c1;
        c1.with(|s| assert_eq!(s, "foo"));
        c2.set(String::from("bar"));
        c1.with(|s| assert_eq!(s, "bar"));
        assert_eq!(s, "bar");
    }
}
