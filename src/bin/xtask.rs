use liblzma::read::XzDecoder;
use liblzma::stream::MtStreamBuilder;
use std::fs::File;
use std::io;

fn main() -> anyhow::Result<()> {
    const INPUT: &str = "measurements.txt.xz";
    const OUTPUT: &str = "measurements.txt";

    let thread_count = std::thread::available_parallelism()?.get() as u32;
    let input = File::open(INPUT)?;
    let mut output = File::create(OUTPUT)?;

    let stream = MtStreamBuilder::new()
        .threads(thread_count)
        .memlimit_stop(u64::MAX)
        .memlimit_threading(u64::MAX)
        .decoder()?;

    let mut decoder = XzDecoder::new_stream(input, stream);
    io::copy(&mut decoder, &mut output)?;

    println!("decompressed {INPUT} to {OUTPUT} with thread count {thread_count}");

    Ok(())
}
