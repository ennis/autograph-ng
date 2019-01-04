use gfx2_extension_runtime::{load_dev_dylib, load_module};
use std::env;
use std::thread::sleep;
use std::time::Duration;
use test_dylib;

#[test]
fn test_reload() {
    env::set_current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/..")).unwrap();

    let mut leak = &0;

    for _ in 0..10 {
        let lib = load_dev_dylib!(test_dylib).unwrap();
        let hot = load_module!(&lib, test_dylib::hot).unwrap();

        let mut test_vec = Vec::new();
        (hot.push)(&mut test_vec);
        // assert_eq!(&test_vec[..], &[&42]);
        leak = test_vec[0];

        //sleep(Duration::from_secs());
        eprintln!("reloading...");
    }

    eprintln!("invalid: {}", leak);
}
