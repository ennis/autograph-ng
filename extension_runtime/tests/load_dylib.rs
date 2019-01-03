use gfx2_extension_runtime::load_module;
use std::env;
use test_dylib;

#[test]
fn test_compile() {
    env::set_current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/..")).unwrap();
    let lib = libloading::Library::new("target/debug/deps/test_dylib.dll").unwrap();

    let mut test_vec = Vec::new();

    let escape = {
        let hot = load_module!(&lib, test_dylib::hot).unwrap();
        (hot.push)(&mut test_vec);
        hot.push
    };

    escape(&mut test_vec);

    /*assert_eq!(&test_vec[..], &[&42]);
    let r = hot.simple(42);
    assert_eq!(r, 43);*/

    // ideally:
    // auto-resolve path to dylib in development context:
    //let hot = load_dev_module!(test_dylib::hot).unwrap();
    // find target directory? debug/release?
    // then it's deps/module-name
}
