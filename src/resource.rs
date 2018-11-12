//! Common resource trait.

/// Trait representing the shared functionality and properties of resources (buffers and images).
pub trait Resource {
    /// Gets the name of the resource.
    /// Note that the name does not uniquely identifies a resource,
    /// as it does not need to be unique among all resources.
    fn name(&self) -> &str;

    /*/// The frame in which the resource was last used.
    fn last_used_frame(&self) -> FrameNumber;*/
}
