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
                if *bit_count == 0 || *bit_count > 16 {
                    panic!("bit_count {} out of range; must be between 1 and 16 inclusive", *bit_count);
                }
                if *first_bit > 31 {
                    panic!("first_bit {} out of range; must be <32", *first_bit);
                }
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

#[cfg(all(test, feature = "bit-manipulation"))]
mod test_bme {
    use super::*;

    #[test]
    fn test_set_field_bits() {
        for first_bit in 0..32 {
            for bit_count in 1..=16 {
                let op = BmeOperation::SetField{first_bit: first_bit, bit_count: bit_count};
                let val = op.bits() & 0xf007ffff;
                assert_eq!(val, 0x10000000, "0x{:X} != 0x10000000 with first_bit={} bit_count={}", val, first_bit, bit_count);
            }
        }
    }

    #[test]
    #[should_panic]
    fn test_set_field_zero_bits() {
        let op = BmeOperation::SetField{first_bit: 0, bit_count: 0};
        op.bits();
    }
    #[test]
    #[should_panic]
    fn test_set_field_too_many_bits() {
        let op = BmeOperation::SetField{first_bit: 0, bit_count: 42};
        op.bits();
    }
    #[test]
    #[should_panic]
    fn test_set_field_wrong_first_bit() {
        let op = BmeOperation::SetField{first_bit: 32, bit_count: 0};
        op.bits();
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
        // Shift the bit_to_modify left 2 bits, since the output addresses must be 32-bit-aligned
        // We can't overwrite bits because incoming addresses are aligned to T, and bit_to_modify
        // is already range-checked against the size of T.  (That is, if the bit number is, say,
        // 27, such that the top 2 bits might collide with lower bits in addr, then those 2 bits
        // in addr must already be clear because of its alignment.)
        let bb_offset = (addr & 0xfffff) << 5 | (u32::from(bit_to_modify & 0x1f)) << 2;
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

#[cfg(all(test, feature = "bit-banding"))]
mod test_bb {
    use super::*;

    #[test]
    fn exhaustively_test_alignment() {
        for addr in 0x20000000..0x20100000 {
            for bit in 0..32 {
                let out = VolatileCell::bitband_pointer(addr as usize as *mut u32, bit) as usize;
                // All possible bitband outputs must be word-aligned
                assert!((out & 0x3) == 0);
            }
        }
        for addr in 0x40000000..0x40100000 {
            for bit in 0..32 {
                let out = VolatileCell::bitband_pointer(addr as usize as *mut u32, bit) as usize;
                // All possible bitband outputs must be word-aligned
                assert!((out & 0x3) == 0);
            }
        }
    }

    #[test]
    #[should_panic]
    fn test_invalid_address_below() {
        VolatileCell::bitband_pointer(0x1fffffffusize as *mut u32, 1);
    }
    #[test]
    #[should_panic]
    fn test_invalid_address_lower_mid() {
        VolatileCell::bitband_pointer(0x20100000usize as *mut u32, 1);
    }
    #[test]
    #[should_panic]
    fn test_invalid_address_upper_mid() {
        VolatileCell::bitband_pointer(0x3fffffffusize as *mut u32, 1);
    }
    #[test]
    #[should_panic]
    fn test_invalid_address_above() {
        VolatileCell::bitband_pointer(0x40100000usize as *mut u32, 1);
    }

    #[test]
    #[should_panic]
    fn test_invalid_bit() {
        VolatileCell::bitband_pointer(0x20080000usize as *mut u8, 12);
    }
}
