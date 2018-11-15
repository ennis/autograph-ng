use sid_vec::Id;

pub struct ImageHandleTag;
pub type ImageHandle = Id<ImageHandleTag>;

pub struct BufferHandleTag;
pub type BufferHandle = Id<BufferHandleTag>;

pub struct SwapchainHandleTag;
pub type SwapchainHandle = Id<SwapchainHandleTag>;
