extern crate clap;
extern crate ecpool;
#[macro_use]
extern crate trackable;

use ecpool::BuildCoder;
use std::time::Instant;

#[cfg(not(unix))]
fn main() {
    panic!("Unsupported platform");
}

#[cfg(unix)]
fn main() -> Result<(), trackable::error::MainError> {
    use clap::{App, Arg};
    use ecpool::ErasureCode;
    use std::fs;
    use std::io::Read;
    use std::num::NonZeroUsize;
    use trackable::error::Failed;

    let matches = App::new("decode")
        .arg(
            Arg::with_name("INPUT_FILES")
                .short("i")
                .takes_value(true)
                .required(true)
                .multiple(true),
        )
        .arg(
            Arg::with_name("DATA_FRAGMENTS")
                .short("k")
                .takes_value(true)
                .default_value("6"),
        )
        .arg(
            Arg::with_name("PARITY_FRAGMENTS")
                .short("m")
                .takes_value(true)
                .default_value("3"),
        )
        .arg(
            Arg::with_name("CHECKSUM")
                .short("c")
                .takes_value(true)
                .possible_values(&["none", "crc32", "md5"])
                .default_value("none"),
        )
        .get_matches();
    let mut inputs = Vec::new();
    for file in matches.values_of("INPUT_FILES").unwrap() {
        println!("# INPUT FILE: {}", file);
        let mut buf = Vec::new();
        let mut f = track_any_err!(fs::File::open(file))?;
        track_any_err!(f.read_to_end(&mut buf))?;
        inputs.push(buf);
    }

    let k = track_any_err!(matches.value_of("DATA_FRAGMENTS").unwrap().parse())?;
    let m = track_any_err!(matches.value_of("PARITY_FRAGMENTS").unwrap().parse())?;
    let checksum = match matches.value_of("CHECKSUM").unwrap() {
        "none" => ecpool::liberasurecode::Checksum::None,
        "crc32" => ecpool::liberasurecode::Checksum::Crc32,
        "md5" => ecpool::liberasurecode::Checksum::Md5,
        _ => unreachable!(),
    };

    let k = track_assert_some!(NonZeroUsize::new(k), Failed);
    let m = track_assert_some!(NonZeroUsize::new(m), Failed);
    let mut ec = track!(ecpool::liberasurecode::LibErasureCoderBuilder::new(k, m)
        .checksum(checksum)
        .build_coder())?;

    let inputs2 = inputs.iter().map(|i| &i[..]).collect::<Vec<_>>();
    let start_time = Instant::now();
    let data = track!(ec.decode(&inputs2[..]))?;
    let elapsed = start_time.elapsed();
    println!(
        "# DECODE TIME: {} sec",
        elapsed.as_secs() as f64 + (elapsed.subsec_nanos() as f64 / 1_000_000_000.0)
    );
    println!("# DECODED SIZE: {} ", data.len());

    Ok(())
}
