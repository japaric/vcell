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

#[cfg(feature = "bit-manipulation")]
#[derive(Debug)]
enum BmeOperation {
    And,
    Or,
    Xor,
    SetField{first_bit: u8, bit_count: u8},
}
#[cfg(feature = "bit-manipulation")]
impl BmeOperation {
    #[inline(always)]
    fn bits(&self) -> usize {
        match self {
            BmeOperation::And => 0x04000000,
            BmeOperation::Or => 0x08000000,
            BmeOperation::Xor => 0x0c000000,
            BmeOperation::SetField{first_bit, bit_count} => {
                0x10000000 |
                    (usize::from(first_bit & 0x1f) << 23) |
                    (usize::from((bit_count-1) & 0xf) << 19)
            },
        }
    }
    #[inline(always)]
    pub fn wrap_pointer<T>(&self, ptr: *mut T) -> *mut T {
        let addr = ptr as usize;
        let mask = match self {
            BmeOperation::SetField{first_bit: _, bit_count: _} => 0x6007ffff,
            _ => 0x600fffff,
        };
        if addr & mask != addr {
            panic!("Tried to use BME on address 0x{:x?}, which operation {:?} does not support", addr, self);
        }
        (addr | self.bits()) as *mut T
    }
}

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

    /// Sets a sub-field of the contained value with the bit-manipulation-engine, if enabled.
    /// See [NXP documentation] on the BME. This is a "BFI" operation.
    ///
    /// [NXP documentation]: https://www.nxp.com/docs/en/application-note/AN4838.pdf
    #[inline(always)]
    #[cfg(feature = "bit-manipulation")]
    pub fn set_field(&self, first_bit: u8, bit_count: u8, value: T)
        where T: Copy
    {
        unsafe {
            let op = BmeOperation::SetField{first_bit: first_bit, bit_count: bit_count};
            let bfi_ptr = op.wrap_pointer(self.value.get());
            ptr::write_volatile(bfi_ptr, value)
        }
    }

    /// Sets a collection of bits of the contained value with the bit-manipulation-engine, if
    /// enabled.
    /// See [NXP documentation] on the BME. This is an "OR" operation.
    ///
    /// [NXP documentation]: https://www.nxp.com/docs/en/application-note/AN4838.pdf
    #[inline(always)]
    #[cfg(feature = "bit-manipulation")]
    pub fn set_bits(&self, bits_to_set: T)
        where T: Copy
    {
        unsafe {
            let or_ptr = BmeOperation::Or.wrap_pointer(self.value.get());
            ptr::write_volatile(or_ptr, bits_to_set)
        }
    }

    /// Clears a collection of bits of the contained value with the bit-manipulation-engine, if
    /// enabled.
    /// See [NXP documentation] on the BME. This is an "AND" operation.
    /// Note that the bits set in bits_to_clear get *cleared* in the register.
    ///
    /// [NXP documentation]: https://www.nxp.com/docs/en/application-note/AN4838.pdf
    #[inline(always)]
    #[cfg(feature = "bit-manipulation")]
    pub fn clear_bits(&self, bits_to_clear: T)
        where T: Copy + core::ops::Not<Output = T>
    {
        unsafe {
            let and_ptr = BmeOperation::And.wrap_pointer(self.value.get());
            ptr::write_volatile(and_ptr, !bits_to_clear)
        }
    }

    /// Inverts a collection of bits of the contained value with the bit-manipulation-engine, if
    /// enabled.
    /// See [NXP documentation] on the BME. This is an "XOR" operation.
    ///
    /// [NXP documentation]: https://www.nxp.com/docs/en/application-note/AN4838.pdf
    #[inline(always)]
    #[cfg(feature = "bit-manipulation")]
    pub fn invert_bits(&self, bits_to_invert: T)
        where T: Copy
    {
        unsafe {
            let xor_ptr = BmeOperation::Xor.wrap_pointer(self.value.get());
            ptr::write_volatile(xor_ptr, bits_to_invert)
        }
    }

    /// See [ARM documentation] and [ST documentation] on bit-banding.
    ///
    /// [ARM documentation]: http://infocenter.arm.com/help/index.jsp?topic=/com.arm.doc.ddi0337h/Behcjiic.html
    /// [ST documentation]: https://www.st.com/content/ccc/resource/technical/document/programming_manual/5b/ca/8d/83/56/7f/40/08/CD00228163.pdf/files/CD00228163.pdf/jcr:content/translations/en.CD00228163.pdf
    #[inline(always)]
    #[cfg(feature = "bit-banding")]
    fn bitband_pointer(addr: *mut T, bit_to_modify: u8) -> *mut u32 {
        let addr = addr as usize as u32;
        if addr < 0x20000000 || (addr > 0x200fffff && addr < 0x40000000) || addr > 0x400fffff {
            panic!("Tried to use bit-banding on address 0x{:x?}, which is outside the bit-banded region");
        }
        if usize::from(bit_to_modify) > core::mem::size_of::<T>()*8 {
            panic!("Tried to change bit {} of value whose size is {}", bit_to_modify, core::mem::size_of::<T>());
        }
        // Shift left 5 bits, since each "normal" bit expands to a 32-bit word in the alias region
        let bb_offset = (addr & 0xfffff) << 5 | u32::from(bit_to_modify & 0x1f);
        (((addr & 0xf0000000) | 0x02000000) | bb_offset) as usize as *mut u32
    }

    /// Sets a single bit of the contained value with bit-banding, if enabled.
    /// See [ARM documentation] and [ST documentation] on bit-banding.
    ///
    /// [ARM documentation]: http://infocenter.arm.com/help/index.jsp?topic=/com.arm.doc.ddi0337h/Behcjiic.html
    /// [ST documentation]: https://www.st.com/content/ccc/resource/technical/document/programming_manual/5b/ca/8d/83/56/7f/40/08/CD00228163.pdf/files/CD00228163.pdf/jcr:content/translations/en.CD00228163.pdf
    #[inline(always)]
    #[cfg(feature = "bit-banding")]
    pub fn set_bit(&self, bit_to_set: u8, value: T)
        where T: core::convert::Into<u32>
    {
        let value = value.into();
        if value > 1 {
            panic!("value {:?} out of range", value)
        }
        unsafe {
            ptr::write_volatile(Self::bitband_pointer(self.value.get(), bit_to_set), value)
        }
    }
}

// NOTE implicit because of `UnsafeCell`
// unsafe impl<T> !Sync for VolatileCell<T> {}
