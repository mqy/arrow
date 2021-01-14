// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

use crate::{
    array::OffsetSizeTrait, buffer::MutableBuffer, datatypes::ToByteSlice, util::bit_util,
};

/// extends the `buffer` to be able to hold `len` bits, setting all bits of the new size to zero.
#[inline]
pub(super) fn reserve_for_bits(buffer: &mut MutableBuffer, len: usize) {
    let needed_bytes = bit_util::ceil(len, 8);
    if buffer.len() < needed_bytes {
        buffer.extend(needed_bytes - buffer.len());
    }
}

/// sets all bits on `write_data` on the range `[offset_write..offset_write+len]` to be equal to the
/// bits on `data` on the range `[offset_read..offset_read+len]`
/// Panics when slice index out of bounds.
pub(super) fn set_bits(
    write_data: &mut [u8],
    data: &[u8],
    offset_write: usize,
    offset_read: usize,
    len: usize,
) -> usize {
    let mut count = 0;
    (0..len).for_each(|i| {
        if bit_util::get_bit(data, offset_read + i) {
            bit_util::set_bit(write_data, offset_write + i);
            count += 1;
        } else {
            bit_util::unset_bit(write_data, offset_write + i);
        }
    });
    count
}

pub(super) fn extend_offsets<T: OffsetSizeTrait>(
    buffer: &mut MutableBuffer,
    mut last_offset: T,
    offsets: &[T],
) {
    buffer.reserve(buffer.len() + offsets.len() * std::mem::size_of::<T>());
    offsets.windows(2).for_each(|offsets| {
        // compute the new offset
        let length = offsets[1] - offsets[0];
        last_offset += length;
        buffer.extend_from_slice(last_offset.to_byte_slice());
    });
}

#[inline]
pub(super) unsafe fn get_last_offset<T: OffsetSizeTrait>(
    offset_buffer: &MutableBuffer,
) -> T {
    // JUSTIFICATION
    //  Benefit
    //      20% performance improvement extend of variable sized arrays (see bench `mutable_array`)
    //  Soundness
    //      * offset buffer is always extended in slices of T and aligned accordingly.
    //      * Buffer[0] is initialized with one element, 0, and thus `mutable_offsets.len() - 1` is always valid.
    let (prefix, offsets, suffix) = offset_buffer.as_slice().align_to::<T>();
    debug_assert!(prefix.is_empty() && suffix.is_empty());
    *offsets.get_unchecked(offsets.len() - 1)
}
#[cfg(test)]
mod tests {
    use super::set_bits;

    #[test]
    fn test_set_bits() {
        let byte_0 = 0_u8;
        let byte_3 = 3_u8;
        let start = 0;
        let len = 2;

        let mut dst = [byte_0; 1];
        let src = [byte_3; 1];
        let count = set_bits(&mut dst[..], &src[..], start, start, len);
        assert_eq!(count, 2);
        assert_eq!(dst[0], byte_3);

        let mut dst = [byte_3; 1];
        let src = [byte_0; 1];
        let count = set_bits(&mut dst[..], &src[..], start, start, len);
        assert_eq!(count, 0);
        assert_eq!(dst[0], byte_0);
    }

    #[test]
    #[should_panic(expected = "index out of bounds: the len is 1 but the index is 1")]
    fn test_set_bits_panic() {
        let byte_0 = 0_u8;
        let byte_3 = 3_u8;
        let mut dst = [byte_0; 1];
        let src = [byte_3; 1];
        let start = 7;

        let _ = set_bits(&mut dst[..], &src[..], start, start, 1);
        let _ = set_bits(&mut dst[..], &src[..], start, start, 2);
    }
}
