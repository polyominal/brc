use memchr::memchr;

const TEMPERATURE_RANGE_ABS: f32 = 100.;

struct Record {
    count: u32,
    min: f32,
    sum: f32,
    max: f32,
}

impl Record {
    fn add(&mut self, temperature: f32) {
        self.count += 1;
        self.min = self.min.min(temperature);
        self.sum += temperature;
        self.max = self.max.max(temperature);
    }
}

impl Default for Record {
    fn default() -> Self {
        Self {
            count: 0,
            min: TEMPERATURE_RANGE_ABS,
            sum: 0.,
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
        let value = unsafe { str::from_utf8_unchecked(value) }
            .parse::<f32>()
            .unwrap();
        map.entry(key).or_default().add(value);
        data = unsafe { data.get_unchecked(sep + 1 + value_len + 1..) };
    }

    let mut list: Vec<_> = map.into_iter().collect();
    list.sort_by_key(|p| p.0);
    for (key, value) in list {
        println!(
            "{key}: {min:.1}/{avg:.1}/{max:.1}",
            key = unsafe { str::from_utf8_unchecked(key) },
            min = value.min,
            avg = value.sum / value.count as f32,
            max = value.max
        );
    }

    Ok(())
}
