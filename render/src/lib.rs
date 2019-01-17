//! Renderer manifesto:
//! * Easy to use
//! * Flexible
//! * Not too verbose
//! * Dynamic
//!
//! Based on global command reordering with sort keys.
//! (see https://ourmachinery.com/post/a-modern-rendering-architecture/)
//! Submission order is fully independant from the execution
//! order on the GPU. The necessary barriers and synchronization
//! is determined once the list of commands is sorted.
//! Thus, adding a post-proc effect is as easy as adding a command buffer with the correct resource
//! names and sequence IDs so that it happens after main rendering.
//! This means that any component in the engine can modify
//! the render pipeline 'non-locally' by submitting a command buffer.
//! This might not be a good thing per se, but at least it's flexible.
//!
//! The `Renderer` instances should be usable across threads
//! (e.g. can allocate and upload from different threads at once).
//!
//! `CommandBuffers` are renderer-agnostic.
//! They contain commands with a sort key that indicates their relative execution order.
//!
#![feature(const_transmute)]

extern crate log;

// Reexport nalgebra_glm types if requested
#[cfg(feature = "glm-types")]
pub use nalgebra_glm as glm;

pub mod arena;
pub mod buffer;
pub mod cmd;
pub mod descriptor;
mod format;
pub mod image;
pub mod interface;
pub mod pipeline;
pub mod shader;
mod sync;
pub mod traits;
mod util;

pub use self::arena::*;
pub use self::buffer::*;
pub use self::cmd::*;
pub use self::descriptor::*;
pub use self::format::*;
pub use self::image::*;
pub use self::pipeline::*;
pub use self::shader::*;
pub use self::traits::RendererBackend;
pub use self::util::*;
// re-export macros
pub use autograph_render_macros::{BufferLayout, DescriptorSetInterface, VertexData};
pub use autograph_shader_macros::{
    glsl_compute, glsl_fragment, glsl_geometry, glsl_tess_control, glsl_tess_eval, glsl_vertex,
    include_combined_shader, include_shader,
};

//--------------------------------------------------------------------------------------------------

/// Currently unused.
#[derive(Copy, Clone, Debug)]
pub enum MemoryType {
    DeviceLocal,
    HostUpload,
    HostReadback,
}

/// Currently unused.
pub enum Queue {
    Graphics,
    Compute,
    Transfer,
}

/// Describes the type of indices contained in an index buffer.
#[derive(Copy, Clone, Debug)]
pub enum IndexFormat {
    /// 16-bit unsigned integer indices
    U16,
    /// 32-bit unsigned integer indices
    U32,
}

//--------------------------------------------------------------------------------------------------

/// A contiguous range in the sorted command stream inside which a resource should not be aliased.
///
/// An AliasScope is defined using a mask and a value (similarly to IP subnet masks, for example):
/// a command with sortkey `s` is inside the range if `s & mask == value`.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct AliasScope {
    pub value: u64,
    pub mask: u64,
}

impl AliasScope {
    /// Returns an AliasScope encompassing the whole command stream.
    pub fn no_alias() -> AliasScope {
        AliasScope { value: 0, mask: 0 }
    }

    /// Returns true if this scope overlaps the other.
    pub fn overlaps(&self, other: &AliasScope) -> bool {
        let m = self.mask & other.mask;
        (self.value & m) == (other.value & m)
    }
}

//--------------------------------------------------------------------------------------------------

/// Renderer
pub struct Renderer<R: RendererBackend> {
    backend: R,
}

impl<R: RendererBackend> Renderer<R> {
    /// Creates a new renderer with the specified backend.
    pub fn new(backend: R) -> Renderer<R> {
        Renderer { backend }
    }

    /// Returns the default swapchain if there is one.
    pub fn default_swapchain(&self) -> Option<Swapchain<R>> {
        self.backend.default_swapchain().map(|s| Swapchain(s))
    }

    /// Creates a command buffer.
    pub fn create_command_buffer<'cmd>(&self) -> CommandBuffer<'cmd, R> {
        CommandBuffer::new()
    }

    /// Submits the given command buffers for rendering and ends the current frame.
    ///
    /// Frame-granularity synchronization points happen in this call.
    /// A new frame is implicitly started after this call.
    pub fn submit_frame(&self, command_buffers: Vec<CommandBuffer<R>>) {
        let commands = sort_command_buffers(command_buffers);
        self.backend.submit_frame(&commands)
    }
}
