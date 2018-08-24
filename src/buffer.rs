
pub enum BufferUsage {
    Upload,
    Default,
    Readback,
    Unspecified
}

#[derive(Debug)]
pub struct BufferDesc
{
    pub offset: usize,
    pub size: usize,
    pub usage: BufferUsage,
}