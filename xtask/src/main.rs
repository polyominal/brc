use std::fs::File;
use std::time::Instant;
use std::{io, time};

use anyhow::Context;
use liblzma::read::XzDecoder;
use liblzma::stream::MtStreamBuilder;
use xshell::{Shell, cmd};

fn main() -> anyhow::Result<()> {
    use flags::XtaskCmd;
    let flags = flags::Xtask::from_env()?;
    let sh = &Shell::new()?;
    match flags.subcommand {
        XtaskCmd::Decompress(_) => decompress(sh),
        XtaskCmd::InstallTools(_) => install_tools(sh),
        XtaskCmd::Bench(_) => bench(sh),
        XtaskCmd::Fmt(_) => fmt(sh),
        XtaskCmd::Smoke(_) => smoke(sh),
        XtaskCmd::Verify(_) => verify(sh),
        XtaskCmd::Profile(_) => profile(sh),
    }
}

fn decompress(sh: &Shell) -> anyhow::Result<()> {
    const INPUT: &str = "measurements.txt.xz";
    const OUTPUT: &str = "measurements.txt";
    const EXPECTED_OUTPUT_SIZE: u64 = 1_379_536_818;

    let check_output = || -> io::Result<bool> {
        match std::fs::metadata(OUTPUT)? {
            meta if meta.len() == EXPECTED_OUTPUT_SIZE => Ok(true),
            _ => {
                eprintln!("{OUTPUT} exists with unexpected size");
                Ok(false)
            }
        }
    };

    match check_output() {
        Ok(true) => {
            eprintln!("{OUTPUT} already exists with expected size, skipping decompression");
            return Ok(());
        }
        Err(e) if e.kind() != io::ErrorKind::NotFound => {
            return Err(e).context("check output file metadata");
        }
        _ => {
            // need to decompress
        }
    }

    eprintln!("starting fetching and decompressing");

    {
        let start = time::Instant::now();

        cmd!(sh, "git lfs pull").run()?;

        eprintln!(
            "fetched {INPUT}. took {elapsed:?}",
            elapsed = start.elapsed()
        );
    }

    {
        let start = time::Instant::now();

        let thread_count = std::thread::available_parallelism()?.get() as u32;
        let input = File::open(INPUT)?;
        let mut output = File::create(OUTPUT)?;
        let stream = MtStreamBuilder::new()
            .threads(thread_count)
            .memlimit_stop(u64::MAX)
            .memlimit_threading(u64::MAX)
            .decoder()?;

        let mut decoder = XzDecoder::new_stream(input, stream);
        io::copy(&mut decoder, &mut output).context("copy decoded bytes to output file")?;

        if !matches!(check_output(), Ok(true)) {
            anyhow::bail!("something wrong happened with input file decompression");
        }

        eprintln!(
            "fetched and decompressed {INPUT} to {OUTPUT} with thread count {thread_count}. took {elapsed:?}",
            elapsed = start.elapsed()
        );
    }

    Ok(())
}

fn install_tools(sh: &Shell) -> anyhow::Result<()> {
    cmd!(sh, "cargo install --locked samply").run()?;

    Ok(())
}

fn build(sh: &Shell, profile: &str) -> anyhow::Result<()> {
    decompress(sh)?;
    cmd!(sh, "cargo build --profile {profile} --bin brc")
        .run()
        .context("build brc binary")?;
    Ok(())
}

fn bench(sh: &Shell) -> anyhow::Result<()> {
    const ITERATIONS: usize = 10;

    build(sh, "release")?;

    let mut times = Vec::with_capacity(ITERATIONS);

    for i in 0..ITERATIONS {
        let start = Instant::now();
        cmd!(sh, "./target/release/brc")
            .ignore_stderr()
            .ignore_stdout()
            .run()
            .context("run brc binary")?;
        let elapsed = start.elapsed();
        times.push(elapsed);

        eprintln!("run #{i}: {elapsed:?}");
    }

    times.sort();
    eprintln!(
        "min/p50/max: {min:?}/{p50:?}/{max:?}",
        min = times.first().unwrap(),
        max = times.last().unwrap(),
        p50 = times[times.len() / 2],
    );

    Ok(())
}

fn fmt(sh: &Shell) -> anyhow::Result<()> {
    cmd!(sh, "cargo fmt").run()?;

    Ok(())
}

fn smoke(sh: &Shell) -> anyhow::Result<()> {
    cmd!(sh, "cargo fmt --check").run()?;
    cmd!(sh, "cargo clippy --all-targets").run()?;

    Ok(())
}

fn verify(sh: &Shell) -> anyhow::Result<()> {
    const REFERENCE: &str = "reference_answer.txt";

    build(sh, "release")?;

    let output = cmd!(sh, "./target/release/brc")
        .read()
        .context("run brc binary")?;

    let reference =
        std::fs::read_to_string(REFERENCE).with_context(|| format!("read {REFERENCE}"))?;

    if output == reference.trim_end() {
        eprintln!("output matches {REFERENCE}");
        Ok(())
    } else {
        eprintln!("output differs from {REFERENCE}");
        anyhow::bail!("verification failed");
    }
}

fn profile(sh: &Shell) -> anyhow::Result<()> {
    install_tools(sh)?;
    build(sh, "profiling")?;

    eprintln!("warming up target/profiling/brc before profiling");
    cmd!(sh, "./target/profiling/brc")
        .ignore_stdout()
        .ignore_stderr()
        .run()
        .context("warm up brc binary")?;

    sh.create_dir("tmp/")?;
    let id = cmd!(sh, "git rev-parse --short=8 HEAD")
        .read()
        .context("get commit ID")?;
    cmd!(
        sh,
        "samply record --port 2333 --no-open --output tmp/{id}.profile.json.gz ./target/profiling/brc"
    )
    .ignore_stdout()
    .run()
    .context("run samply record")?;

    Ok(())
}

mod flags {
    xflags::xflags! {
        cmd xtask {
            /// Decompress `measurements.txt.xz`
            cmd decompress {}
            /// Install tools required for the challenge
            cmd install-tools {}
            /// Benchmark `brc` and report min, max, and p50 times
            cmd bench {}
            /// Format the codebase
            cmd fmt {}
            /// Run smoke tests (fmt check + clippy)
            cmd smoke {}
            /// Verify brc output matches the reference answer
            cmd verify {}
            /// Profile `brc` using samply (opens Firefox Profiler)
            cmd profile {}
        }
    }
}
