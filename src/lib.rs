//! Just like [`Cell`] but with [volatile] read / write operations
//!
//! [`Cell`]: https://doc.rust-lang.org/std/cell/struct.Cell.html
//! [volatile]: https://doc.rust-lang.org/std/ptr/fn.read_volatile.html

#![deny(missing_docs)]
#![deny(warnings)]
#![cfg_attr(feature = "const-fn", feature(const_fn))]
#![no_std]

use core::cell::UnsafeCell;
use core::ptr;

/// Just like [`Cell`] but with [volatile] read / write operations
///
/// [`Cell`]: https://doc.rust-lang.org/std/cell/struct.Cell.html
/// [volatile]: https://doc.rust-lang.org/std/ptr/fn.read_volatile.html
pub struct VolatileCell<T> {
    value: UnsafeCell<T>,
}

impl<T> VolatileCell<T> {
    /// Creates a new `VolatileCell` containing the given value
    #[cfg(feature = "const-fn")]
    pub const fn new(value: T) -> Self {
        VolatileCell { value: UnsafeCell::new(value) }
    }

    /// Creates a new `VolatileCell` containing the given value
    ///
    /// NOTE A `const fn` variant is available under the "const-fn" Cargo
    /// feature
    #[cfg(not(feature = "const-fn"))]
    pub fn new(value: T) -> Self {
        VolatileCell { value: UnsafeCell::new(value) }
    }

    /// Returns a copy of the contained value
    #[inline(always)]
    pub fn get(&self) -> T
        where T: Copy
    {
        unsafe { ptr::read_volatile(self.value.get()) }
    }

    /// Sets the contained value
    #[inline(always)]
    pub fn set(&self, value: T)
        where T: Copy
    {
        unsafe { ptr::write_volatile(self.value.get(), value) }
    }
}

// NOTE implicit because of `UnsafeCell`
// unsafe impl<T> !Sync for VolatileCell<T> {}
