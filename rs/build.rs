use std::fs::File;
use std::io::Write;

fn main() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let mut file =
        File::create(format!("{manifest_dir}/../src/eggmock.h")).expect("could not create eggmock.h");
    write!(file, "{}", eggmock::ffi_header()).expect("could not write eggmock.h");
}
