use std::fs::File;
use std::io;

fn main() -> io::Result<()> {
    let input_path = "measurements.txt.xz";
    let output_path = "measurements.txt";

    let num_workers = std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(4)
        .clamp(1, 256);

    let input = File::open(input_path)?;
    let mut decoder = lzma_rust2::XzReaderMt::new(input, true, num_workers)?;
    let mut output = File::create(output_path)?;

    io::copy(&mut decoder, &mut output)?;

    println!(
        "decompressed {} to {} (using {} workers)",
        input_path, output_path, num_workers
    );
    Ok(())
}
