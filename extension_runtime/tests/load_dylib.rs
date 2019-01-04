use gfx2_extension_runtime::{load_dev_dylib, load_module};
use std::env;
use test_dylib;

#[test]
fn test_compile() {
    env::set_current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/..")).unwrap();

    let lib = load_dev_dylib!(test_dylib).unwrap();
    let hot = load_module!(&lib, test_dylib::hot).unwrap();
    let mut test_vec = Vec::new();
    (hot.push)(&mut test_vec);
    assert_eq!(&test_vec[..], &[&42]);
    let r = (hot.simple)(42);
    assert_eq!(r, 43);
}
