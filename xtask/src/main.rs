use std::fs::File;
use std::io;
use std::time::Instant;

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
    cmd!(sh, "git lfs pull").run()?;

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
    io::copy(&mut decoder, &mut output).context("copy decoded bytes to output file")?;

    eprintln!("decompressed {INPUT} to {OUTPUT} with thread count {thread_count}");

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

    sh.create_dir("tmp/")?;
    cmd!(
        sh,
        "samply record --port 2333 --output tmp/profile.json.gz ./target/profiling/brc"
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
