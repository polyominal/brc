type Key = u64;
type Value = i32;

fn main() -> std::io::Result<()> {
    let data = std::fs::read("measurements.txt")?;
    let mut data = data.as_slice();

    let mut map = rustc_hash::FxHashMap::<Key, Record>::default();
    while !data.is_empty() {
        let (key, key_bytes, after_key) = parse_key(data);
        let (value, rest) = parse_value(after_key);
        map.entry(key)
            .and_modify(|r| r.add(value))
            .or_insert_with(|| Record::new(key_bytes, value));
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

fn parse_key(s: &[u8]) -> (Key, &[u8], &[u8]) {
    if s.len() < 8 {
        return parse_key_scalar(s);
    }

    let first_word = read_word(s, 0);
    let mut matches = word_byte_matches(first_word, b';');
    if matches != 0 {
        return parsed_key(s, first_word, matches.trailing_zeros() as usize / 8);
    }

    let mut offset = 8;
    while offset + 8 <= s.len() {
        matches = word_byte_matches(read_word(s, offset), b';');
        if matches != 0 {
            return parsed_key(
                s,
                first_word,
                offset + matches.trailing_zeros() as usize / 8,
            );
        }
        offset += 8;
    }

    let mut len = offset;
    loop {
        if unsafe { *s.get_unchecked(len) } == b';' {
            return parsed_key(s, first_word, len);
        }
        len += 1;
    }
}

fn parse_key_scalar(s: &[u8]) -> (Key, &[u8], &[u8]) {
    let mut leading = 0_u64;
    let mut len = 0;

    loop {
        let byte = unsafe { *s.get_unchecked(len) };
        if byte == b';' {
            break;
        }
        if len < 8 {
            leading |= (byte as u64) << (8 * len);
        }
        len += 1;
    }

    parsed_key(s, leading, len)
}

fn parsed_key(s: &[u8], first_word: u64, len: usize) -> (Key, &[u8], &[u8]) {
    let shift = 64_usize.saturating_sub(8 * len);
    let key = (first_word << shift) ^ (len as u64);
    let key_bytes = unsafe { s.get_unchecked(..len) };
    let rest = unsafe { s.get_unchecked(len + 1..) };

    (key, key_bytes, rest)
}

fn read_word(s: &[u8], offset: usize) -> u64 {
    unsafe { u64::from_le(s.as_ptr().add(offset).cast::<u64>().read_unaligned()) }
}

fn word_byte_matches(word: u64, byte: u8) -> u64 {
    let repeated = u64::from_le_bytes([byte; 8]);
    let diff = word ^ repeated;

    (diff.wrapping_sub(0x0101_0101_0101_0101) & !diff) & 0x8080_8080_8080_8080
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
            min: r.min as f32 / 10_f32,
            avg: (r.sum as f32 / r.count as f32) / 10_f32,
            max: r.max as f32 / 10_f32,
        }
    }
}

struct Record<'a> {
    key: &'a [u8],
    count: u32,
    min: Value,
    sum: Value,
    max: Value,
}

impl<'a> Record<'a> {
    fn add(&mut self, value: Value) {
        self.count += 1;
        self.min = self.min.min(value);
        self.sum += value;
        self.max = self.max.max(value);
    }

    fn new(key: &'a [u8], value: Value) -> Self {
        Self {
            key,
            count: 1,
            min: value,
            sum: value,
            max: value,
        }
    }
}
