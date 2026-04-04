use std::io::Read;

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
            max: TEMPERATURE_RANGE_ABS,
        }
    }
}

fn main() -> std::io::Result<()> {
    let mut data = String::new();
    {
        let mut file = std::fs::File::open("measurements.txt")?;
        file.read_to_string(&mut data)?;
    }

    let mut map = std::collections::HashMap::<_, Record>::new();
    for line in data.trim().split("\n") {
        let (city, temperature) = line.split_once(';').unwrap();
        let value = temperature.parse::<f32>().unwrap();
        map.entry(city).or_default().add(value);
    }

    let mut list: Vec<_> = map.into_iter().collect();
    list.sort_by_key(|p| p.0);
    for (city, record) in list {
        println!(
            "{city}: {min:.1}/{avg:.1}/{max:1}",
            min = record.min,
            avg = record.sum / record.count as f32,
            max = record.max
        );
    }

    Ok(())
}
