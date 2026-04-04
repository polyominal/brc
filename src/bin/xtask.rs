use std::fs::File;
use std::io;
use std::time::Instant;

use liblzma::read::XzDecoder;
use liblzma::stream::MtStreamBuilder;

use xshell::{Shell, cmd};

fn main() -> anyhow::Result<()> {
    use flags::XtaskCmd;
    let flags = flags::Xtask::from_env()?;
    let sh = &Shell::new()?;
    match flags.subcommand {
        XtaskCmd::Decompress(_) => decompress(),
        XtaskCmd::InstallTools(_) => install_tools(sh),
        XtaskCmd::Bench(_) => bench(sh),
    }
}

fn decompress() -> anyhow::Result<()> {
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

fn install_tools(sh: &Shell) -> anyhow::Result<()> {
    cmd!(sh, "cargo install flamegraph").run()?;

    Ok(())
}

fn bench(sh: &Shell) -> anyhow::Result<()> {
    const ITERATIONS: usize = 10;

    cmd!(sh, "cargo build --bin brc --release").run()?;

    let mut times = Vec::with_capacity(ITERATIONS);

    for i in 0..ITERATIONS {
        let start = Instant::now();
        cmd!(sh, "./target/release/brc")
            .ignore_stderr()
            .ignore_stdout()
            .run()?;
        let elapsed = start.elapsed();
        times.push(elapsed);

        eprintln!("run #{i}: {elapsed:?}");
    }

    times.sort();
    println!("p50: {p50:?}", p50 = times[times.len() / 2]);

    Ok(())
}

mod flags {
    xflags::xflags! {
        cmd xtask {
            /// Decompress `measurements.txt.xz`
            cmd decompress {}
            /// Install tools required for the challenge
            cmd install-tools {}
            /// Benchmark `brc` and report p50 time
            cmd bench {}
        }
    }
}
