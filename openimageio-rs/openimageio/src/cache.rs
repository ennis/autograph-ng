use crate::attribute::AttributeType;
use crate::cstring_to_owned;
use crate::input::ImageChannelsInput;
use crate::roi::Roi;
use crate::spec::channel_descs_from_index_ranges;
use crate::spec::coalesce_channels;
use crate::spec::ChannelRanges;
use crate::spec::ChannelSelect;
use crate::typedesc::ImageData;
use crate::ChannelDesc;
use crate::Error;
use crate::ImageBuffer;
use crate::ImageSpec;
use crate::ImageSpecOwned;
use crate::TypeDesc;
use openimageio_sys as sys;
use openimageio_sys::AsStringRef;
use std::ffi::c_void;
use std::ffi::CStr;
use std::ffi::CString;
use std::mem;
use std::ops::Bound;
use std::ops::Range;
use std::ops::RangeBounds;
use std::os::raw::c_char;
use std::path::Path;
use std::ptr;

pub struct ImageCache(*mut sys::OIIO_ImageCache);

impl ImageCache {
    fn get_last_error(&self) -> String {
        unsafe { cstring_to_owned(sys::OIIO_ImageCache_geterror(self.0)) }
    }

    pub fn new() -> ImageCache {
        let ptr = unsafe { sys::OIIO_ImageCache_create(false) };
        ImageCache(ptr)
    }

    pub fn new_shared() -> ImageCache {
        let ptr = unsafe { sys::OIIO_ImageCache_create(true) };
        ImageCache(ptr)
    }

    pub fn invalidate<P: AsRef<Path>>(&self, filename: P) {
        let filename_str = filename.as_ref().to_str().unwrap();
        unsafe {
            sys::OIIO_ImageCache_invalidate(self.0, filename_str.as_stringref());
        }
    }

    pub fn get_attribute<A: AttributeType>(&self, attr_name: &str) -> Result<A, Error> {
        unsafe {
            A::get(|ptr| {
                let success = sys::OIIO_ImageCache_getattribute(
                    self.0,
                    attr_name.as_stringref(),
                    A::TYPEDESC.0,
                    ptr,
                );
                if success {
                    Ok(())
                } else {
                    Err(Error::InvalidAttributeNameOrType)
                }
            })
        }
    }

    pub fn set_attribute<A: AttributeType>(&self, attr_name: &str, attr: A) -> Result<(), Error> {
        unsafe {
            attr.set(|ptr| {
                let success = sys::OIIO_ImageCache_attribute(
                    self.0,
                    attr_name.as_stringref(),
                    A::TYPEDESC.0,
                    ptr,
                );
                if success {
                    Ok(())
                } else {
                    Err(Error::InvalidAttributeNameOrType)
                }
            })
        }
    }

    pub fn max_memory_mb(&self) -> f32 {
        self.get_attribute("max_memory_MB").unwrap()
    }

    pub fn set_max_memory_mb(&self, megabytes: f32) {
        self.set_attribute("max_memory_MB", megabytes).unwrap();
    }

    pub fn max_open_files(&self) -> u32 {
        self.get_attribute::<i32>("max_open_files").unwrap() as u32
    }

    pub fn set_max_open_files(&self, max_open_files: u32) {
        self.set_attribute("max_open_files", max_open_files as i32)
            .unwrap();
    }

    pub fn total_files(&self) -> usize {
        self.get_attribute::<i32>("total_files").unwrap() as usize
    }

    pub fn all_filenames(&self) -> Vec<&str> {
        let n = self.total_files();
        let mut out_raw: Vec<*const c_char> = vec![ptr::null(); n];
        let tydesc = sys::OIIO_TypeDesc {
            arraylen: n as i32,
            ..TypeDesc::STRING.0
        };

        unsafe {
            let success = sys::OIIO_ImageCache_getattribute(
                self.0,
                "all_filenames".as_stringref(),
                tydesc,
                out_raw.as_mut_ptr() as *mut c_void,
            );
            assert!(success);
        }

        out_raw
            .into_iter()
            .map(|ptr| {
                unsafe { CStr::from_ptr(ptr) }
                    .to_str()
                    .expect("invalid UTF-8")
            })
            .collect()
    }

    pub fn image<P: AsRef<Path>>(&self, path: P) -> Result<CachedImage, Error> {
        let path_stringref = path
            .as_ref()
            .to_str()
            .expect("invalid UTF-8")
            .as_stringref();

        let handle = unsafe { sys::OIIO_ImageCache_get_image_handle(self.0, path_stringref) };

        if handle.is_null() {
            return Err(Error::OpenError(self.get_last_error()));
        }

        let mut img = CachedImage {
            cache: self,
            spec: self.get_image_spec(handle, 0, 0)?,
            handle,
        };

        Ok(img)
    }

    pub fn get_image_spec(
        &self,
        handle: *mut sys::OIIO_ImageCache_ImageHandle,
        subimage: usize,
        miplevel: usize,
    ) -> Result<ImageSpecOwned, Error> {
        let spec = ImageSpecOwned::new();
        let success = unsafe {
            sys::OIIO_ImageCache_get_imagespec_by_handle(
                self.0,
                handle,
                ptr::null_mut(),
                spec.0,
                0,
                0,
                false,
            )
        };
        if success {
            Ok(spec)
        } else {
            Err(Error::SubimageNotFound)
        }
    }
}

impl Drop for ImageCache {
    fn drop(&mut self) {
        unsafe {
            sys::OIIO_ImageCache_destroy(self.0, false);
        }
    }
}

/// Handle to a cached image.
///
/// It's safe to clone since the image cache is thread safe.
#[derive(Clone)]
pub struct CachedImage<'a> {
    cache: &'a ImageCache,
    spec: ImageSpecOwned,
    handle: *mut sys::OIIO_ImageCache_ImageHandle,
}

impl<'a> CachedImage<'a> {
    pub fn subimage(self, subimage: usize) -> Result<CachedSubimageMipmap<'a>, Error> {
        self.subimage_mipmap(subimage, 0)
    }

    pub fn subimage_mipmap(
        self,
        subimage: usize,
        miplevel: usize,
    ) -> Result<CachedSubimageMipmap<'a>, Error> {
        let spec = self.cache.get_image_spec(self.handle, subimage, miplevel)?;
        Ok(CachedSubimageMipmap {
            cache: self.cache,
            spec,
            handle: self.handle,
            subimage,
            miplevel,
        })
    }

    /// Returns the metadata of this subimage.
    pub fn spec(&self) -> &ImageSpec {
        &self.spec
    }

    /// Returns the width of this image.
    ///
    /// Equivalent to `spec().width()`.
    pub fn width(&self) -> u32 {
        self.spec().width()
    }

    /// Returns the width of this image.
    ///
    /// Equivalent to `spec().height()`.
    pub fn height(&self) -> u32 {
        self.spec().height()
    }

    /// Returns the width of this image.
    ///
    /// Equivalent to `spec().depth()`.
    pub fn depth(&self) -> u32 {
        self.spec().depth()
    }

    /// Selects channels.
    pub fn channels_by_name(
        self,
        channel_names: &[&str],
    ) -> Result<CachedSubimageMipmapChannels<'a>, Error> {
        self.subimage(0).unwrap().channels_by_name(channel_names)
    }

    /// Selects channels.
    pub fn channels(self, channels: &[usize]) -> Result<CachedSubimageMipmapChannels<'a>, Error> {
        self.subimage(0).unwrap().channels(channels)
    }

    /// Selects channels.
    pub fn all_channels(self) -> CachedSubimageMipmapChannels<'a> {
        self.subimage(0).unwrap().all_channels()
    }

    /// Selects channels.
    pub fn channels_rgba(self) -> Result<CachedSubimageMipmapChannels<'a>, Error> {
        self.subimage(0).unwrap().channels_rgba()
    }

    /// Selects channels.
    pub fn channel_alpha(self) -> Result<CachedSubimageMipmapChannels<'a>, Error> {
        self.subimage(0).unwrap().channel_alpha()
    }

    /// Shorthand to read all the channels of the top mip level of the first subimage
    /// into an [ImageBuffer].
    pub fn read<I: ImageData>(self) -> Result<ImageBuffer<I>, Error> {
        self.subimage(0).unwrap().all_channels().read()
    }
}

#[derive(Clone)]
pub struct CachedSubimageMipmap<'a> {
    cache: &'a ImageCache,
    spec: ImageSpecOwned,
    handle: *mut sys::OIIO_ImageCache_ImageHandle,
    subimage: usize,
    miplevel: usize,
}

impl<'a> CachedSubimageMipmap<'a> {
    /// Returns the metadata of this subimage.
    pub fn spec(&self) -> &ImageSpec {
        &self.spec
    }

    /// Returns the width of this image.
    ///
    /// Equivalent to `spec().width()`.
    pub fn width(&self) -> u32 {
        self.spec().width()
    }

    /// Returns the width of this image.
    ///
    /// Equivalent to `spec().height()`.
    pub fn height(&self) -> u32 {
        self.spec().height()
    }

    /// Returns the width of this image.
    ///
    /// Equivalent to `spec().depth()`.
    pub fn depth(&self) -> u32 {
        self.spec().depth()
    }

    fn with_channels(self, channels: ChannelRanges) -> CachedSubimageMipmapChannels<'a> {
        CachedSubimageMipmapChannels {
            cache: self.cache,
            spec: self.spec,
            handle: self.handle,
            subimage: self.subimage,
            miplevel: self.miplevel,
            channels,
        }
    }

    /// Selects channels.
    pub fn channels_by_name(
        self,
        channel_names: &[&str],
    ) -> Result<CachedSubimageMipmapChannels<'a>, Error> {
        let channels = self.spec.channels_by_name(channel_names)?;
        Ok(self.with_channels(channels))
    }

    /// Selects channels.
    pub fn channels(self, channels: &[usize]) -> Result<CachedSubimageMipmapChannels<'a>, Error> {
        let channels = self.spec.channels_by_index(channels)?;
        Ok(self.with_channels(channels))
    }

    /// Selects channels.
    pub fn all_channels(self) -> CachedSubimageMipmapChannels<'a> {
        let channels = self.spec.all_channels();
        self.with_channels(channels)
    }

    /// Selects channels.
    pub fn channels_rgba(self) -> Result<CachedSubimageMipmapChannels<'a>, Error> {
        self.channels_by_name(&["R", "G", "B", "A"])
    }

    /// Selects channels.
    pub fn channel_alpha(self) -> Result<CachedSubimageMipmapChannels<'a>, Error> {
        self.channels_by_name(&["A"])
    }

    /// Shorthand to read all the channels into an [ImageBuffer].
    pub fn read<I: ImageData>(self) -> Result<ImageBuffer<I>, Error> {
        self.all_channels().read()
    }
}

pub struct CachedSubimageMipmapChannels<'a> {
    cache: &'a ImageCache,
    spec: ImageSpecOwned,
    handle: *mut sys::OIIO_ImageCache_ImageHandle,
    subimage: usize,
    miplevel: usize,
    channels: ChannelRanges,
}

impl<'a> CachedSubimageMipmapChannels<'a> {
    /// Returns the metadata of this subimage.
    pub fn spec(&self) -> &ImageSpec {
        &self.spec
    }

    /// Returns the width of this image.
    ///
    /// Equivalent to `spec().width()`.
    pub fn width(&self) -> u32 {
        self.spec().width()
    }

    /// Returns the width of this image.
    ///
    /// Equivalent to `spec().height()`.
    pub fn height(&self) -> u32 {
        self.spec().height()
    }

    /// Returns the width of this image.
    ///
    /// Equivalent to `spec().depth()`.
    pub fn depth(&self) -> u32 {
        self.spec().depth()
    }

    /// Reads channels from the entire image.
    pub fn read<I: ImageData>(&self) -> Result<ImageBuffer<I>, Error> {
        self.read_region(.., .., ..)
    }

    /// Reads channels from a region of an image.
    pub fn read_region<I: ImageData>(
        &self,
        xs: impl RangeBounds<i32>,
        ys: impl RangeBounds<i32>,
        zs: impl RangeBounds<i32>,
    ) -> Result<ImageBuffer<I>, Error> {
        let spec = self.spec();
        let (xs, ys, zs) = spec.calculate_bounds(xs, ys, zs);
        let (width, height, depth) = (xs.len(), ys.len(), zs.len());
        let n = width * height * depth  * self.channels.count;
        let mut data = Vec::with_capacity(n);

        unsafe {
            self.read_region_unchecked(xs, ys, zs, data.as_mut_ptr())?;
            data.set_len(n);
        }

        Ok(ImageBuffer {
            width,
            height,
            depth,
            data,
            channels: channel_descs_from_index_ranges(&spec, &self.channels.ranges),
        })
    }

    unsafe fn read_region_unchecked<I: ImageData>(
        &self,
        xs: Range<i32>,
        ys: Range<i32>,
        zs: Range<i32>,
        out: *mut I,
    ) -> Result<(), Error> {
        let mut success = true;
        let mut ich = 0;
        for r in self.channels.ranges.iter() {
            success &= sys::OIIO_ImageCache_get_pixels_stride_by_handle(
                self.cache.0,
                // filename
                self.handle,
                ptr::null_mut(),
                // subimage
                self.subimage as i32,
                // miplevel
                self.miplevel as i32,
                // xbegin, xend
                xs.start,
                xs.end,
                // ybegin, yend
                ys.start,
                ys.end,
                // zbegin, zend
                zs.start,
                zs.end,
                // chbegin, chend
                r.start as i32,
                r.end as i32,
                // format
                I::DESC.0,
                // result
                out.offset(ich) as *mut c_void,
                // xstride
                (self.channels.count * mem::size_of::<I>()) as isize,
                // ystride
                sys::OIIO_AutoStride,
                // zstride
                sys::OIIO_AutoStride,
                // cache_chbegin, cache_chend
                0,
                -1,
            );

            ich += r.len() as isize;
        }

        if success {
            Ok(())
        } else {
            Err(Error::ReadError(self.cache.get_last_error()))
        }
    }
}

/*
pub struct ImageHandle<'a> {
    cache: &'a ImageCache,
    handle: *mut sys::OIIO_ImageCache_ImageHandle,
}
*/
