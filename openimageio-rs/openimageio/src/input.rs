use crate::cstring_to_owned;
use crate::error::get_last_error;
use crate::spec::ChannelSelect;
use crate::typedesc::ImageData;
use crate::ChannelDesc;
use crate::Error;
use crate::ImageSpec;
use crate::ImageSpecOwned;
use core::mem;
use openimageio_sys as sys;
use openimageio_sys::AsStringRef;
use std::ffi::c_void;
use std::path::Path;
use std::ptr;
use std::slice;

/// Image file opened for input.
///
/// Use [ImageInput::open] to open an image file.
///
/// Images may contain multiple _subimages_ (e.g. the faces of a cube map)
/// and/or _mip maps_. You must select which subimage to read from with the
/// [ImageInput::subimage_0] or [ImageInput::subimage] methods, and use
/// the returned [SubimageInput] object to read image data.
/// These methods exclusively borrow the `ImageInput` object, so it's impossible to read multiple
/// subimages at once.
pub struct ImageInput(*mut sys::OIIO_ImageInput);

impl ImageInput {
    /// Opens the image file at the specified path.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<ImageInput, Error> {
        let path = path.as_ref().to_str().unwrap();
        let ptr = unsafe { sys::OIIO_ImageInput_open(path.as_stringref(), ptr::null()) };
        if ptr.is_null() {
            Err(Error::OpenError(get_last_error()))
        } else {
            Ok(ImageInput(ptr))
        }
    }

    /// Returns the first subimage/miplevel from the file, which is guaranteed to exist.
    pub fn subimage_0(&mut self) -> SubimageInput {
        self.subimage(0, 0).unwrap()
    }

    /// Returns the subimage corresponding to the given index and mip level.
    ///
    /// Returns None if the subimage/miplevel pair does not exist.
    pub fn subimage(&mut self, subimage: usize, miplevel: usize) -> Option<SubimageInput> {
        let newspec = ImageSpecOwned::new();

        let exists = unsafe {
            sys::OIIO_ImageInput_seek_subimage_miplevel(
                self.0,
                subimage as i32,
                miplevel as i32,
                newspec.0,
            )
        };

        if exists {
            Some(SubimageInput {
                img: self,
                spec: newspec,
                subimage,
                miplevel,
            })
        } else {
            None
        }
    }

    fn get_last_error(&self) -> String {
        unsafe { cstring_to_owned(sys::OIIO_ImageInput_geterror(self.0)) }
    }
}

/// Subimages.
///
/// This represents an individual subimage and mip level from an image file.
pub struct SubimageInput<'a> {
    img: &'a ImageInput,
    spec: ImageSpecOwned,
    subimage: usize,
    miplevel: usize,
}

impl<'a> SubimageInput<'a> {
    pub fn spec(&self) -> &ImageSpec {
        &self.spec
    }

    pub fn width(&self) -> u32 {
        self.spec().width()
    }

    pub fn height(&self) -> u32 {
        self.spec().height()
    }

    pub fn depth(&self) -> u32 {
        self.spec().depth()
    }

    pub fn subimage_index(&self) -> usize {
        self.subimage
    }

    pub fn mip_level(&self) -> usize {
        self.miplevel
    }

    /// Reads channels to an [ImageBuffer].
    ///
    /// #### Example usages:
    /// - read all channels into a floating-point image buffer:
    /// ```
    /// imagein.read::<f32>(AllChannels)
    /// ```
    /// - read channels R,G,B and A, selected using a regular expression:
    /// ```
    /// imagein.read::<f32>("[RGBA]")
    /// ```
    ///
    /// #### Outstanding issues:
    /// There is currently no way to guarantee with a regexp that the specified channels are all read,
    /// or that they are read in the correct order.
    /// (i.e. with the selector `"[RGBA]"` the function may read the R,G,B,A channels in that order,
    /// or just RGB, or ABGR, or BGR, or nothing at all)
    pub fn read<T: ImageData, C: ChannelSelect>(
        &self,
        channels: C,
    ) -> Result<ImageBuffer<T>, Error> {
        let spec = self.spec();
        // calculate necessary size
        let (nch, chans) = channels.into_channel_ranges(spec);
        let n = (spec.width() * spec.height() * spec.depth()) as usize * nch;
        let mut data: Vec<T> = Vec::with_capacity(n);
        let mut channels = Vec::new();

        // read all channel ranges
        let mut success = true;
        let mut ich = 0;
        for r in chans.iter() {
            for ch in r.clone() {
                channels.push(self.spec.channel_by_index(ch).unwrap().to_channel_desc());
            }
            success &= unsafe {
                sys::OIIO_ImageInput_read_image_format2(
                    self.img.0,
                    r.start as i32,
                    r.end as i32,
                    T::DESC.0,
                    data.as_mut_ptr().offset(ich) as *mut c_void,
                    (nch * mem::size_of::<T>()) as isize,
                    sys::OIIO_AutoStride,
                    sys::OIIO_AutoStride,
                    ptr::null_mut(),
                )
            };

            ich += r.len() as isize;
        }

        if success {
            unsafe {
                data.set_len(n);
            }
            Ok(ImageBuffer {
                width: self.width() as usize,
                height: self.height() as usize,
                depth: self.depth() as usize,
                data,
                channels,
            })
        } else {
            Err(Error::ReadError(self.img.get_last_error()))
        }
    }
}

impl Drop for ImageInput {
    fn drop(&mut self) {
        unsafe {
            sys::OIIO_ImageInput_delete(self.0);
        }
    }
}

/// Memory buffer containing image data.
///
/// The image data is stored in a `Vec`, which you can extract with [into_vec].
pub struct ImageBuffer<T: ImageData> {
    width: usize,
    height: usize,
    depth: usize,
    channels: Vec<ChannelDesc>,
    data: Vec<T>,
}

impl<T: ImageData> ImageBuffer<T> {
    /// Returns the width of this image.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Returns the height of this image.
    pub fn height(&self) -> usize {
        self.height
    }

    /// Returns the depth of this image.
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Returns the image data reinterpreted as a slice of bytes.
    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(
                self.data.as_ptr() as *const u8,
                self.data.len() * mem::size_of::<T>(),
            )
        }
    }

    /// Returns the number of channels of this image.
    pub fn num_channels(&self) -> usize {
        self.channels.len()
    }

    /// Returns the descriptions of all channels of this image.
    pub fn channels(&self) -> &[ChannelDesc] {
        &self.channels
    }

    /// Consumes this object and returns the `Vec` containing the image data.
    pub fn into_vec(self) -> Vec<T> {
        self.data
    }
}
