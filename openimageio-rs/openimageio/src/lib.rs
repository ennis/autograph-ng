use std::ffi::CStr;
use std::os::raw::c_char;

mod error;
mod imagecache;
mod input;
mod output;
mod roi;
mod spec;
mod typedesc;

pub use error::Error;
pub use spec::AllChannels;
pub use spec::Channel;
pub use spec::ChannelDesc;
pub use spec::ImageSpec;
pub use spec::ImageSpecOwned;
//pub use spec::ChannelFormats;
//pub use spec::ChannelRange;
pub use input::ImageBuffer;
pub use input::ImageInput;
pub use output::ImageOutput;
pub use output::MultiImageOutput;
pub use output::SingleImageOutput;
pub use typedesc::Aggregate;
pub use typedesc::BaseType;
pub use typedesc::TypeDesc;
pub use typedesc::VecSemantics;

unsafe fn cstring_to_owned(cstr: *const c_char) -> String {
    // assume utf8 input
    let msg = CStr::from_ptr(cstr).to_str().unwrap().to_owned();
    openimageio_sys::OIIO_freeString(cstr);
    msg
}

// use-case
// open file with specs, write one image, close
// open file with specs, write multiple images, close
// open file, append image, close
// An imageoutput must be ready to write
//
// Open existing image for appending or modification
//   open(Existing), append_subimage(spec) -> SubimageWriter, modify_subimage(spec)
// OR create(path), open(spec) -> SubimageWriter,
// Create new (empty) image, append images
//   create(path), open(spec, Create)
#[cfg(test)]
mod tests {
    use super::*;
    use crate::imageio::ChannelRange;
    use std::mem;
    use std::slice;

    #[test]
    fn open_image() {
        let img = super::ImageInput::open("../test_images/tonberry.jpg");
        assert!(img.is_ok());
    }

    #[test]
    fn open_image_exr() {
        let mut img = super::ImageInput::open("../test_images/output0013.exr").unwrap();

        for ch in img.subimage_0().spec().channels() {
            println!("channel {:?}", ch);
        }

        let chans = img
            .subimage_0()
            .spec()
            .find_channels(r"RenderLayer\.DiffCol\..*")
            .collect::<Vec<_>>();
        println!("selected channels {:?}", chans);
        let size = (
            img.subimage_0().spec().width(),
            img.subimage_0().spec().height(),
        );

        let data: Vec<f32> = img.subimage_0().read_to_vec(&chans[..]).unwrap();

        let spec = ImageSpecOwned::new_2d(TypeDesc::FLOAT, size.0, size.1, &["R", "G", "B"]);
        let mut out = ImageOutput::create("output.exr").unwrap();
        let mut out0 = out.open(&spec).unwrap();
        out0.write_image(&data).unwrap();
    }

    #[test]
    fn open_image_psd() {
        let mut img = super::ImageInput::open("../test_images/cup.psd").unwrap();
        for ch in img.subimage_0().spec().channels() {
            println!("channel {:?}", ch);
        }
    }

    #[test]
    fn open_image_tif() {
        let mut img = super::ImageInput::open("../test_images/cup.tif").unwrap();
        for ch in img.subimage_0().spec().channels() {
            println!("channel {:?}", ch);
        }
    }

    #[test]
    fn open_nonexistent_image() {
        let img = super::ImageInput::open("../test_images/nonexistent.png");
        if let Err(ref e) = img {
            println!("{}", e);
        }
        assert!(img.is_err());
    }

}
