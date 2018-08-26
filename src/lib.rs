//#![feature(rust_2018_preview, uniform_paths)]
extern crate petgraph;
#[macro_use] extern crate bitflags;
extern crate toml;
extern crate config;
#[macro_use]
extern crate ash;
extern crate winit;
extern crate winapi;
extern crate pretty_env_logger;
#[macro_use]
extern crate log;

mod texture;
pub mod format;
mod buffer;
mod buffer_data;
mod upload_buffer;
mod fence;
pub mod window;
pub mod context;
pub mod frame;


// TODO: design a low-level layer for resources and commands (in the eventuality of switching to Vulkan at some point)
// Safe write access to persistent resources:
// -> update reference each frame (texture = texture_next;)
// -> update automatically, but track revisions within a frame.

// subsystems:
// - frame graph (resource management)
// - pipeline (pipeline state management: loading shaders and verifying interfaces)
// - draw command interface (binding of vertex buffers, index buffers...)
// - high-level: meshes, geometry helpers

// Resource table:
// Texture { Rc<storage (raw GL object, no drop impl)>, is_transient (reclaimed at end of frame) }
// Buffer { Rc<storage (raw GL object, no drop impl)>, offset + size, is_transient }
//
// Persistent resource table:
// Each frame, delete resources that have no refs (if deemed necessary: can also keep them allocated for caching)
// For buffers, resources has associated allocators
//
// Persistent resource:
// Texture { Rc<storage (raw GL object, no drop impl)>, Rc<suballocation>, last frame accessed }
//
// Layers of resources: revision -> resource (suballocation) -> (RC) memory object
//
// Scope:
// - can load (internal) resources: fonts, textures
// - can create its own window and GL/Vulkan/D3D context, invisible to the user.
//      - has another function to pass Context creation options.
// - supports multi-window.
//
// ISSUE: openGL and vulkan handling of resources is different:
// - openGL cannot alias memory for textures
// - vulkan can
// - openGL can alias buffer memory by using offsets inside a bigger buffer
// - vulkan's buffer objects can already be suballocations of memory
// Q: How to design the current system so that the transition from GL to vulkan is smooth?
// A**: use vulkan directly
// A?: use gfx-rs
// A*: multi-backend design
//
// Practical considerations:
// - we will be targeting many different configurations: Windows + Linux
// Q: What's the status of vulkan on windows? should Direct3D12 be preferred?
// A: Some people like using windows tools to debug/profile graphics code.
// Q: What about DirectX RayTracing?
// A?: Should be ported to vulkan with NV extensions "soon"
//

// STEP 0: window and event loop.
// STEP 1: clear the framebuffer.
// STEP 2: create a persistent texture.
