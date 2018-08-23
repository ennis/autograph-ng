pub enum BufferUsage {
    UPLOAD,
    DEFAULT,
    READBACK,
}

#[derive(Debug)]
pub struct BufferDesc
{
    pub offset: usize,
    pub size: usize,
}