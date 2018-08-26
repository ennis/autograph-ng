use buffer_data::BufferData;
use std::mem;
use std::os::raw::c_void;

#[derive(Copy, Clone, Debug)]
pub enum BufferUsage {
    Upload,
    Default,
    Readback,
    Unspecified
}

/// Describes a buffer.
#[derive(Copy, Clone, Debug)]
pub struct BufferDesc
{
    //pub offset: usize,
    pub size: usize,
    pub usage: BufferUsage,
}

/// A slice into a buffer.
#[derive(Copy, Clone, Debug)]
pub struct BufferSlice
{
    pub offset: usize,
    pub size: usize
}

/// Raw OpenGL buffer object.
pub(crate) struct BufferStorage
{
    //pub(crate) obj: GLuint,
    pub(crate) size: usize,
    pub(crate) usage: BufferUsage,
    //pub(crate) flags: GLuint,
}

impl Drop for BufferStorage
{
    fn drop(&mut self) {
        unimplemented!()
    }
}

/*fn usage_to_creation_flags(usage: BufferUsage) -> GLuint {
    match usage {
        BufferUsage::Readback => gl::MAP_READ_BIT | gl::MAP_PERSISTENT_BIT | gl::MAP_COHERENT_BIT,
        // Upload buffers are persistent/coherent
        BufferUsage::Upload => gl::MAP_WRITE_BIT | gl::MAP_PERSISTENT_BIT | gl::MAP_COHERENT_BIT,
        BufferUsage::Default => 0,
        BufferUsage::Unspecified => 0
    }
}*/
/*
unsafe fn create_buffer<T: BufferData + ?Sized>(
    byte_size: usize,
    flags: GLuint,
    initial_data: Option<&T>,
) -> GLuint {
    let mut obj: GLuint = 0;
    gl::CreateBuffers(1, &mut obj);
    gl::NamedBufferStorage(
        obj,
        byte_size as isize,
        if let Some(data) = initial_data {
            data as *const T as *const GLvoid
        } else {
            0 as *const GLvoid
        },
        flags,
    );

    obj
}*/

impl BufferStorage
{
    pub(crate) fn new(size: usize, usage: BufferUsage) -> BufferStorage {
        //unimplemented!()
        //let flags = usage_to_creation_flags(usage);
        BufferStorage {
            //obj: unsafe { create_buffer::<u8>(size, flags, None) },
            size,
            usage,
            //flags,
        }
    }

    pub(crate) fn with_data<T: BufferData + ?Sized>(
        usage: BufferUsage,
        data: &T,
    ) -> BufferStorage {
        let size = mem::size_of_val(data);
        unimplemented!()
        //let flags = usage_to_creation_flags(usage);
        /*BufferStorage {
            obj: unsafe { create_buffer(size, flags, Some(data)) },
            size,
            usage,
            flags
        }*/
    }

    pub(crate) unsafe fn map_all(&self) -> *mut c_void {
        /*let flags = match self.usage {
            BufferUsage::READBACK => {
                gl::MAP_UNSYNCHRONIZED_BIT
                    | gl::MAP_READ_BIT
                    | gl::MAP_PERSISTENT_BIT
                    | gl::MAP_COHERENT_BIT
            }
            BufferUsage::UPLOAD => {
                gl::MAP_UNSYNCHRONIZED_BIT
                    | gl::MAP_WRITE_BIT
                    | gl::MAP_PERSISTENT_BIT
                    | gl::MAP_COHERENT_BIT
            }
            BufferUsage::DEFAULT => {
                panic!("Cannot map a buffer allocated with BufferUsage::DEFAULT")
            }
        };*/

        //gl::MapNamedBufferRange(self.obj, 0, self.byte_size() as isize, flags)
        unimplemented!()
    }
}