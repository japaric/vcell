//! Just like [`Cell`] but with [volatile] read / write operations
//!
//! [`Cell`]: https://doc.rust-lang.org/std/cell/struct.Cell.html
//! [volatile]: https://doc.rust-lang.org/std/ptr/fn.read_volatile.html

#![deny(missing_docs)]
#![deny(warnings)]
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

    /// Returns a raw pointer to the underlying data in the cell
    #[inline(always)]
    pub fn as_ptr(&self) -> *mut T {
        self.value.get()
    }
}

// NOTE implicit because of `UnsafeCell`
// unsafe impl<T> !Sync for VolatileCell<T> {}

/// Reset value of the register
pub trait ResetValue {
    /// Reset value of the register
    const RESET_VALUE: Self;
}

/// Reads the contents of the register
pub trait ReadRegister<R, T>: core::ops::Deref<Target = VolatileCell<T>>
where
    T: Copy + Into<R>,
{
    /// Reads the contents of the register
    #[inline]
    fn read(&self) -> R {
        (*self).get().into()
    }
}

/// Writes to the register using `RESET_VALUE` as basis
pub trait WriteRegisterWithReset<W, T>: core::ops::Deref<Target = VolatileCell<T>>
where
    W: ResetValue + core::ops::Deref<Target = T>,
    T: Copy,
{
    /// Writes to the register
    #[inline]
    fn write<F>(&self, f: F)
    where
        F: FnOnce(&mut W) -> &mut W,
    {
        let mut w = W::RESET_VALUE;
        f(&mut w);
        (*self).set(*w);
    }
}

/// Writes to the register
pub trait WriteRegisterWithZero<W, T>: core::ops::Deref<Target = VolatileCell<T>>
where
    W: core::ops::Deref<Target = T>,
    T: Copy + Default + Into<W>,
{
    /// Writes to the register
    #[inline]
    fn write<F>(&self, f: F)
    where
        F: FnOnce(&mut W) -> &mut W,
    {
        let mut w = T::default().into();
        f(&mut w);
        (*self).set(*w);
    }
}

/// Writes the reset value to the register
pub trait ResetRegister<W, T>: WriteRegisterWithReset<W, T>
where
    W: ResetValue + core::ops::Deref<Target = T>,
    T: Copy,
{
    /// Writes the reset value to the register
    #[inline]
    fn reset(&self) {
        self.write(|w| w)
    }
}

/// Writes Zero to the register
pub trait ResetRegisterWithZero<W, T>: WriteRegisterWithZero<W, T>
where
    W: core::ops::Deref<Target = T>,
    T: Copy + Default + Into<W>,
{
    /// Writes Zero to the register
    #[inline]
    fn reset(&self) {
        self.write(|w| w)
    }
}

/// Modifies the contents of the register
pub trait ModifyRegister<R, W, T>: core::ops::Deref<Target = VolatileCell<T>>
where
    W:  core::ops::Deref<Target = T>,
    T: Copy + Into<R> + Into<W>,
{
    /// Modifies the contents of the register
    #[inline]
    fn modify<F>(&self, f: F)
    where
        for<'w> F: FnOnce(&R, &'w mut W) -> &'w mut W,
    {
        let bits = (*self).get();
        let r: R = bits.into();
        let mut w: W = bits.into();
        f(&r, &mut w);
        (*self).set(*w);
    }
}

/// Single bit read access proxy
pub trait BitR {
    /// Returns `true` if the bit is clear (0)
    #[inline]
    fn bit_is_clear(&self) -> bool {
        !self.bit()
    }
    /// Returns `true` if the bit is set (1)
    #[inline]
    fn bit_is_set(&self) -> bool {
        self.bit()
    }
    /// Returns the current state of the bit as boolean
    fn bit(&self) -> bool;
}

/// Single bit write access proxy
pub trait BitW<'a, W> {
    /// Sets the field bit
    #[inline]
    fn set_bit(self) -> &'a mut W
    where
        Self: core::marker::Sized,
    {
        self.bit(true)
    }
    /// Clears the field bit
    #[inline]
    fn clear_bit(self) -> &'a mut W
    where
        Self: core::marker::Sized,
    {
        self.bit(false)
    }
    /// Writes raw bit(s) to the field
    fn bit(self, value: bool) -> &'a mut W
    where
        Self: core::marker::Sized;
}
