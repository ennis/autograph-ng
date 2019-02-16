/*
pub struct ImageCache(*mut sys::OIIO_ImageCache);

impl ImageCache
{
    pub fn new() -> ImageCache {
        let ptr = unsafe {
            sys::OIIO_ImageCache_create(false);
        };
        ImageCache(ptr)
    }

    pub fn new_shared() -> ImageCache {
        let ptr = unsafe {
            sys::OIIO_ImageCache_create(true);
        };
        ImageCache(ptr)
    }

    pub fn invalidate<P: AsRef<Path>>(&self, filename: P) {
        let filename_str = filename.as_ref().to_str().unwrap();
        unsafe {
            sys::OIIO_ImageCache_invalidate(self.0, filename_str.as_stringref());
        }
    }


    pub fn get_pixels<P: AsRef<Path>, I: ImageData, C: ChannelSelect>(
        filename: P,
        subimage: usize,
        miplevel: usize,
        roi: Roi,
        channels: C,
        out: &mut [I],
    )
    {

    }

}

impl Drop for ImageCache
{
    fn drop(&mut self) {
        unsafe {
            sys::OIIO_ImageCache_destroy(self.0);
        }
    }
}

pub struct ImageHandle<'a> {
    cache: &'a ImageCache,
    handle: *mut sys::OIIO_ImageCache_ImageHandle
}

impl ImageHandle
{

}
*/
