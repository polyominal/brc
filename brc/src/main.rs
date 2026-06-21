#![feature(portable_simd)]

use std::simd::Simd;
use std::simd::cmp::SimdOrd;

use memchr::memchr;

type Key = u64;
type Value = i32;

fn main() -> std::io::Result<()> {
    let data = std::fs::read("measurements.txt")?;
    let mut data = data.as_slice();

    let mut map = rustc_hash::FxHashMap::<Key, Record>::default();
    while let Some(sep) = memchr(b';', data) {
        let key = unsafe { data.get_unchecked(..sep) };
        let (value, rest) = parse_value(unsafe { data.get_unchecked(sep + 1..) });
        map.entry(slice_to_key(key))
            .and_modify(|r| r.add(value))
            .or_insert_with(|| Record::new(key, value));
        data = rest;
    }

    let mut list: Vec<_> = map.values().collect();
    list.sort_by_key(|r| r.key);
    for r in list {
        let r = DisplayedRecord::from(r);
        println!(
            "{key}: {min:.1}/{avg:.1}/{max:.1}",
            key = r.key,
            min = r.min,
            avg = r.avg,
            max = r.max
        );
    }

    Ok(())
}

// I assure you that this gives zero collision...
fn slice_to_key(s: &[u8]) -> u64 {
    let leading_dirty = unsafe { s.as_ptr().cast::<u64>().read_unaligned() };

    // clean up garbage bytes. note that from_le should be a no-op for little-endian systems
    let shift = 64_usize.saturating_sub(8 * s.len());
    let leading = u64::from_le(leading_dirty) << shift;

    leading ^ (s.len() as u64)
}

fn parse_value(s: &[u8]) -> (Value, &[u8]) {
    let is_negative = unsafe { *s.get_unchecked(0) == b'-' };
    let offset = is_negative as usize;
    let unsigned = unsafe { s.get_unchecked(offset..) };
    let has_tens = unsafe { (*unsigned.get_unchecked(1) != b'.') as usize };

    let (tens, ones, tenths) = unsafe {
        (
            *unsigned.get_unchecked(0) - b'0',
            *unsigned.get_unchecked(has_tens) - b'0',
            *unsigned.get_unchecked(2 + has_tens) - b'0',
        )
    };
    let value =
        (has_tens as Value) * 100 * (tens as Value) + 10 * (ones as Value) + (tenths as Value);
    let value = if !is_negative { value } else { -value };
    let rest = unsafe { s.get_unchecked(offset + 4 + has_tens..) };

    (value, rest)
}

struct DisplayedRecord<'a> {
    key: &'a str,
    min: f32,
    avg: f32,
    max: f32,
}

impl<'a> From<&Record<'a>> for DisplayedRecord<'a> {
    fn from(r: &Record<'a>) -> Self {
        Self {
            key: unsafe { str::from_utf8_unchecked(r.key) },
            min: r.min_max[0] as f32 / 10_f32,
            avg: (r.count_sum[1] as f32 / r.count_sum[0] as f32) / 10_f32,
            max: -r.min_max[1] as f32 / 10_f32,
        }
    }
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
