extern crate clap;
extern crate ecpool;
#[macro_use]
extern crate trackable;

#[cfg(not(unix))]
fn main() {
    panic!("Unsupported platform");
}

#[cfg(unix)]
fn main() -> Result<(), trackable::error::MainError> {
    use clap::{App, Arg};
    use ecpool::{BuildCoder, ErasureCode};
    use std::fs;
    use std::io::{Read, Write};
    use std::num::NonZeroUsize;
    use std::time::Instant;
    use trackable::error::Failed;

    let matches = App::new("encode")
        .arg(Arg::with_name("INPUT_FILE").index(1).required(true))
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
    let input_file = matches.value_of("INPUT_FILE").unwrap();
    let mut input_data = Vec::new();
    let mut file = track_any_err!(fs::File::open(input_file))?;
    track_any_err!(file.read_to_end(&mut input_data))?;
    println!("# READ INPUT FILE: bytes={}", input_data.len());

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

    let start_time = Instant::now();
    let fragments = track!(ec.encode(&input_data[..]))?;
    let elapsed = start_time.elapsed();
    println!(
        "# ENCODE TIME: {} sec",
        elapsed.as_secs() as f64 + (elapsed.subsec_nanos() as f64 / 1_000_000_000.0)
    );
    let encoded_size: usize = fragments.iter().map(|f| f.len()).sum();
    println!(
        "# ENCODED SIZE: bytes={} ({}/{} = {})",
        encoded_size,
        encoded_size,
        input_data.len(),
        encoded_size as f64 / input_data.len() as f64
    );
    for (i, fragment) in fragments.iter().enumerate().take(k.get()) {
        let path = format!("{}.data_{}", input_file, i);
        println!(
            "# DATA FRAGMENT({}): bytes={}, file={:?}",
            i,
            fragment.len(),
            path
        );
        let mut file = track_any_err!(fs::File::create(path))?;
        track_any_err!(file.write_all(&fragment[..]))?;
    }
    for i in 0..m.get() {
        let path = format!("{}.parity_{}", input_file, i);
        println!(
            "# PARITY FRAGMENT({}): bytes={}, file={:?}",
            i,
            fragments[k.get() + i].len(),
            path
        );
        let mut file = track_any_err!(fs::File::create(path))?;
        track_any_err!(file.write_all(&fragments[k.get() + i][..]))?;
    }
    Ok(())
}
