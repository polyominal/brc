#![feature(portable_simd)]

use std::simd::Simd;
use std::simd::cmp::SimdOrd;

use memchr::memchr;

type Key = u64;

type Value = i32;

fn slice_to_value(mut s: &[u8]) -> Value {
    let is_negative = if unsafe { *s.get_unchecked(0) } == b'-' {
        s = unsafe { s.get_unchecked(1..) };
        true
    } else {
        false
    };

    let len = s.len();

    let (a, b, c) = unsafe {
        (
            *s.get_unchecked(len - 4) - b'0',
            *s.get_unchecked(len - 3) - b'0',
            *s.get_unchecked(len - 1) - b'0',
        )
    };
    let value = if len == 4 { 100 * (a as Value) } else { 0 } + 10 * (b as Value) + (c as Value);

    if !is_negative { value } else { -value }
}

struct Record<'a> {
    key: &'a [u8],
    count_sum: Simd<i32, 2>,
    min_max: Simd<i32, 2>,
}

impl<'a> Record<'a> {
    fn add(&mut self, value: Value) {
        self.count_sum += Simd::from_array([1, value]);
        self.min_max = self.min_max.simd_min(Simd::from_array([value, -value]));
    }

    fn new(key: &'a [u8], value: Value) -> Self {
        Self {
            key,
            count_sum: Simd::from_array([1, value]),
            min_max: Simd::from_array([value, -value]),
        }
    }
}

// I assure you that this gives zero collision...
fn slice_to_key(s: &[u8]) -> u64 {
    let leading_dirty = unsafe { s.as_ptr().cast::<u64>().read_unaligned() };

    // clean up garbage bytes. note that from_le should be a no-op for little-endian systems
    let shift = 64_usize.saturating_sub(8 * s.len());
    let leading = u64::from_le(leading_dirty) << shift;

    leading ^ (s.len() as u64)
}

fn main() -> std::io::Result<()> {
    let data = std::fs::read("measurements.txt")?;
    let mut data = data.as_slice();

    let mut map = rustc_hash::FxHashMap::<Key, Record>::default();
    while let Some(sep) = memchr(b';', data) {
        let value_len = memchr(b'\n', unsafe { data.get_unchecked(sep + 1..) }).unwrap();
        let (key, value) = unsafe {
            (
                data.get_unchecked(..sep),
                data.get_unchecked(sep + 1..sep + 1 + value_len),
            )
        };
        let value = slice_to_value(value);
        map.entry(slice_to_key(key))
            .and_modify(|r| r.add(value))
            .or_insert_with(|| Record::new(key, value));
        data = unsafe { data.get_unchecked(sep + 1 + value_len + 1..) };
    }

    let mut list: Vec<_> = map.values().collect();
    list.sort_by_key(|r| r.key);
    for r in list {
        println!(
            "{key}: {min:.1}/{avg:.1}/{max:.1}",
            key = unsafe { str::from_utf8_unchecked(r.key) },
            min = r.min_max[0] as f32 / 10_f32,
            avg = (r.count_sum[1] as f32 / r.count_sum[0] as f32) / 10_f32,
            max = -r.min_max[1] as f32 / 10_f32,
        );
    }

    Ok(())
}
