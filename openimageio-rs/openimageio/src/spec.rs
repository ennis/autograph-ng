//! ImageSpec
use crate::TypeDesc;
use itertools::Itertools;
use openimageio_sys as sys;
use openimageio_sys::AsStringRef;
use std::ffi::CStr;
use std::ops::Deref;
use std::ops::Range;
use std::os::raw::c_int;

/// Describes a color channel of an image.
#[derive(Copy, Clone, Debug)]
pub struct Channel<'a> {
    /// Format of the channel data.
    pub format: TypeDesc,
    /// Name of the channel.
    pub name: &'a str,
}

impl<'a> Channel<'a> {
    pub fn to_channel_desc(&self) -> ChannelDesc {
        ChannelDesc {
            format: self.format,
            name: self.name.to_string(),
        }
    }
}

/// Version of [Channel] that owns its contents.
#[derive(Clone, Debug)]
pub struct ChannelDesc {
    /// Format of the channel data.
    pub format: TypeDesc,
    /// Name of the channel.
    pub name: String,
}

/// Image specification: contains metadata about an image.
pub struct ImageSpec(pub(crate) sys::OIIO_ImageSpec); // ImageSpec is zero-sized

/// Represents a rectangular window in some coordinate space.
pub struct Window {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
}

impl ImageSpec {
    /// Returns the _data window_ of the image, containing:
    /// - the origin `(x, y, z)` of the pixel data of the image.
    /// - the size `(width, height, depth)` of the data of this image.
    ///
    /// (OpenImageIO:) A depth greater than 1 indicates a 3D "volumetric" image.
    ///
    /// (OpenImageIO:) `x,y,z` default to (0,0,0), but setting them differently may indicate
    /// that this image is offset from the usual origin.
    ///
    /// (OpenImageIO:) Pixel data are defined over pixel coordinates \[x ... x+width-1\] horizontally,
    /// \[y ... y+height-1\] vertically, and \[z ... z+depth-1\] in depth.
    pub fn data_window(&self) -> Window {
        Window {
            x: self.x(),
            y: self.y(),
            z: self.z(),
            width: self.width(),
            height: self.height(),
            depth: self.depth(),
        }
    }

    /// Equivalent to `self.data_window().x`.
    pub fn x(&self) -> i32 {
        unsafe { sys::OIIO_ImageSpec_x(&self.0) }
    }

    /// Equivalent to `self.data_window().y`.
    pub fn y(&self) -> i32 {
        unsafe { sys::OIIO_ImageSpec_y(&self.0) }
    }

    /// Equivalent to `self.data_window().z`.
    pub fn z(&self) -> i32 {
        unsafe { sys::OIIO_ImageSpec_z(&self.0) }
    }

    /// Returns the size of the image data `(width, height, depth)`.
    ///
    /// Equivalent to `(self.width(), self.height(), self.depth())`
    pub fn size(&self) -> (u32, u32, u32) {
        (self.width(), self.height(), self.depth())
    }

    /// Returns the width of this image.
    ///
    /// Equivalent to `self.data_window().width`.
    pub fn width(&self) -> u32 {
        unsafe { sys::OIIO_ImageSpec_width(&self.0) as u32 }
    }

    /// Returns the height of this image.
    ///
    /// Equivalent to `self.data_window().height`.
    pub fn height(&self) -> u32 {
        unsafe { sys::OIIO_ImageSpec_height(&self.0) as u32 }
    }

    /// Returns the depth of this image.
    ///
    /// Equivalent to `self.data_window().depth`.
    pub fn depth(&self) -> u32 {
        unsafe { sys::OIIO_ImageSpec_depth(&self.0) as u32 }
    }

    /// Returns the 2D size `(width,height)` of the image data, or `None` if this is not a 2D image
    /// (i.e. `depth != 1`)
    pub fn width_height(&self) -> Option<(u32, u32)> {
        if self.depth() == 1 {
            Some((self.width(), self.height()))
        } else {
            None
        }
    }

    /// Returns the "full" or "display" window of the image.
    ///
    /// (OpenImageIO) Having the full display window different from the pixel data window can be helpful in
    /// cases where you want to indicate that your image is a crop window of a larger image (if
    /// the pixel data window is a subset of the full display window), or that the pixels include
    /// overscan (if the pixel data is a superset of the full display window), or may simply indicate
    /// how different non-overlapping images piece together.
    pub fn display_window(&self) -> Window {
        Window {
            x: self.display_x(),
            y: self.display_y(),
            z: self.display_z(),
            width: self.display_width(),
            height: self.display_height(),
            depth: self.display_depth(),
        }
    }

    /// Equivalent to `self.display_window().x`.
    pub fn display_x(&self) -> i32 {
        unsafe { sys::OIIO_ImageSpec_full_x(&self.0) }
    }

    /// Equivalent to `self.display_window().y`.
    pub fn display_y(&self) -> i32 {
        unsafe { sys::OIIO_ImageSpec_full_y(&self.0) }
    }

    /// Equivalent to `self.display_window().z`.
    pub fn display_z(&self) -> i32 {
        unsafe { sys::OIIO_ImageSpec_full_z(&self.0) }
    }

    /// Returns the origin of the display window.
    ///
    /// Equivalent to `(self.display_x(),self.display_y(),self.display_z())`.
    pub fn display_origin(&self) -> (i32, i32, i32) {
        (self.display_x(), self.display_y(), self.display_z())
    }

    /// Equivalent to `self.display_window().width`.
    pub fn display_width(&self) -> u32 {
        unsafe { sys::OIIO_ImageSpec_full_width(&self.0) as u32 }
    }

    /// Equivalent to `self.display_window().height`.
    pub fn display_height(&self) -> u32 {
        unsafe { sys::OIIO_ImageSpec_full_height(&self.0) as u32 }
    }

    /// Equivalent to `self.display_window().depth`.
    pub fn display_depth(&self) -> u32 {
        unsafe { sys::OIIO_ImageSpec_full_depth(&self.0) as u32 }
    }

    /// Returns the size of the display window.
    ///
    /// Equivalent to `(self.display_width(),self.display_height(),self.display_depth())`.
    pub fn display_size(&self) -> (u32, u32, u32) {
        (
            self.display_width(),
            self.display_height(),
            self.display_depth(),
        )
    }

    /// (OpenImageIO:) The number of channels (color values) present in each pixel of the image.
    ///
    /// For example, an RGB image has 3 channels.
    pub fn num_channels(&self) -> usize {
        unsafe { sys::OIIO_ImageSpec_nchannels(&self.0) as usize }
    }

    /// Returns an iterator over the descriptions of the channels of the image.
    pub fn channels<'a>(&'a self) -> impl Iterator<Item = Channel> + 'a {
        let nch = self.num_channels();
        (0..nch).map(move |i| self.channel_by_index(i).unwrap())
    }

    /// Returns the description of the channel at index `index`.
    pub fn channel_by_index(&self, index: usize) -> Option<Channel> {
        let nch = self.num_channels();
        if index >= nch {
            return None;
        }
        let i = index as i32;

        let name = unsafe {
            CStr::from_ptr(sys::OIIO_ImageSpec_channelname(&self.0, i))
                .to_str()
                .unwrap()
        };

        let format = unsafe { TypeDesc(sys::OIIO_ImageSpec_channelformat(&self.0, i)) };

        Some(Channel {
            format,
            name,
            //pixel_bytes,
        })
    }

    /// Finds every channel whose name match the specified regular expression.
    pub fn find_channels<'a>(&'a self, re: &str) -> impl Iterator<Item = usize> + 'a {
        let re = regex::Regex::new(re).expect("invalid regular expression");
        self.channels()
            .enumerate()
            .filter(move |(_, ch)| re.is_match(ch.name))
            .map(|(i, _)| i)
    }
}

/// Version of [ImageSpec] that owns its data.
pub struct ImageSpecOwned(pub(crate) *mut sys::OIIO_ImageSpec);

impl ImageSpecOwned {
    /// Creates the metadata of a zero-sized image with unknown format.
    pub fn new() -> ImageSpecOwned {
        let ptr = unsafe { sys::OIIO_ImageSpec_new(TypeDesc::UNKNOWN.0) };
        ImageSpecOwned(ptr)
    }

    /// Creates the metadata of a 2D image with the specified format, resolution, and channels.
    ///
    /// All channels share the same format.
    pub fn new_2d(format: TypeDesc, xres: u32, yres: u32, channels: &[&str]) -> ImageSpecOwned {
        let channels = channels
            .iter()
            .map(|s| s.as_stringref())
            .collect::<Vec<_>>();

        let formatptr = &format.0;

        let ptr = unsafe {
            sys::OIIO_ImageSpec_new_2d(
                xres as c_int,
                yres as c_int,
                channels.len() as c_int,
                false,
                formatptr,
                channels.as_ptr(),
            )
        };

        ImageSpecOwned(ptr)
    }

    /*pub fn new_2d_0(xres: u32, yres: u32, channel_formats: ChannelFormats, channel_names: &[&str]) -> ImageSpecOwned {
        let channel_names = channel_names.iter().map(|s| s.as_stringref()).collect::<Vec<_>>();

        let (sepchannels, formatptr) = match channel_formats {
            ChannelFormats::Single(ref typedesc) => {
                (false, typedesc)
            }
            ChannelFormats::PerChannel(typedescs) => {
                (true, typedescs)
            }
        };

        let ptr = unsafe {
            sys::OIIO_ImageSpec_new_2d(
                xres as c_int,
                yres as c_int,
                channel_names.len() as c_int,
                sepchannels,
                formatptr,
                channel_names.as_ptr()
            )
        };

        ImageSpecOwned(ptr)
    }*/

    //pub fn new()
}

impl Drop for ImageSpecOwned {
    fn drop(&mut self) {
        unsafe {
            sys::OIIO_ImageSpec_delete(self.0);
        }
    }
}

impl Deref for ImageSpecOwned {
    type Target = ImageSpec;

    fn deref(&self) -> &ImageSpec {
        unsafe { &*(self.0 as *const ImageSpec) }
    }
}

/// Helper function to turn a sequence of channel indices into contiguous ranges of indices.
///
/// This is done to optimize the number of reads necessary to load a set of channels in memory.
fn coalesce_channels(channels: impl Iterator<Item = usize>) -> (usize, Vec<Range<usize>>) {
    let mut count = 0;
    // optimize this
    let r = channels
        .map(|i| {
            count += 1;
            i..i + 1
        })
        .coalesce(|a, b| {
            if a.end == b.start {
                Ok(a.start..b.end)
            } else {
                Err((a, b))
            }
        })
        .collect::<Vec<_>>();
    (count, r)
}

/// Objects that describe a set of channels in an image.
pub trait ChannelSelect {
    /// Turns this object into a set of contiguous channel ranges.
    fn into_channel_ranges(self, spec: &ImageSpec) -> (usize, Vec<Range<usize>>);
}

/// Single-channel selection via index.
impl ChannelSelect for usize {
    fn into_channel_ranges(self, _spec: &ImageSpec) -> (usize, Vec<Range<usize>>) {
        (1, vec![self..self])
    }
}

/// Multi-channel selection via regular expression.
impl<'a> ChannelSelect for &'a str {
    fn into_channel_ranges(self, spec: &ImageSpec) -> (usize, Vec<Range<usize>>) {
        coalesce_channels(spec.find_channels(self))
    }
}

//pub struct ChannelRange<T: IntoIterator<Item = usize>>(pub T);

/*/// select by range
impl<'a> ChannelSelect for &'a [usize] {
    fn into_channel_ranges(self, _spec: &ImageSpec) -> (usize, Vec<Range<usize>>) {
        coalesce_channels(self.iter().cloned())
    }
}*/

/// A dummy type implementing `ChannelSelect` that means "select all channels of the image".
///
/// Example usage:
/// ```
/// let buf: ImageBuffer<f32> = subimage.read(AllChannels);
/// ```
pub struct AllChannels;

impl ChannelSelect for AllChannels {
    fn into_channel_ranges(self, spec: &ImageSpec) -> (usize, Vec<Range<usize>>) {
        (spec.num_channels(), vec![0..spec.num_channels()])
    }
}
