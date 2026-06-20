use memchr::memchr;

type Value = i32;
type Sum = i64;

const TEMPERATURE_RANGE_ABS: Value = 1000;

fn slice_to_value(mut s: &[u8]) -> Value {
    let is_negative = if unsafe { *s.get_unchecked(0) } == b'-' {
        s = unsafe { s.get_unchecked(1..) };
        true
    } else {
        false
    };

    let value: Value = match s {
        [a, b, b'.', c] => {
            100 * ((*a - b'0') as Value) + 10 * ((*b - b'0') as Value) + ((*c - b'0') as Value)
        }
        [a, b'.', b] => 10 * ((*a - b'0') as Value) + ((*b - b'0') as Value),
        _ => panic!("unrecognized value: {s:?}"),
    };
    if !is_negative { value } else { -value }
}

struct Record {
    count: u32,
    min: Value,
    sum: Sum,
    max: Value,
}

impl Record {
    fn add(&mut self, value: Value) {
        self.count += 1;
        self.min = self.min.min(value);
        self.sum += value as Sum;
        self.max = self.max.max(value);
    }
}

impl Default for Record {
    fn default() -> Self {
        Self {
            count: 0,
            min: TEMPERATURE_RANGE_ABS,
            sum: 0,
            max: -TEMPERATURE_RANGE_ABS,
        }
    }
}

fn main() -> std::io::Result<()> {
    let data = std::fs::read("measurements.txt")?;
    let mut data = data.as_slice();

    let mut map = std::collections::HashMap::<&[u8], Record>::new();
    while let Some(sep) = memchr(b';', data) {
        let value_len = memchr(b'\n', unsafe { data.get_unchecked(sep + 1..) }).unwrap();
        let (key, value) = unsafe {
            (
                data.get_unchecked(..sep),
                data.get_unchecked(sep + 1..sep + 1 + value_len),
            )
        };
        let value = slice_to_value(value);
        map.entry(key).or_default().add(value);
        data = unsafe { data.get_unchecked(sep + 1 + value_len + 1..) };
    }

    let mut list: Vec<_> = map.into_iter().collect();
    list.sort_by_key(|p| p.0);
    for (key, value) in list {
        println!(
            "{key}: {min:.1}/{avg:.1}/{max:.1}",
            key = unsafe { str::from_utf8_unchecked(key) },
            min = value.min as f32 / 10_f32,
            avg = (value.sum as f32 / value.count as f32) / 10_f32,
            max = value.max as f32 / 10_f32
        );
    }

    Ok(())
}
