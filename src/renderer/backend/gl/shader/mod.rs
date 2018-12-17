use std::error::Error;
use std::ffi::CString;
use std::fmt;
use std::mem;
use std::os::raw::c_void;
use std::ptr;

//pub mod interface;
pub mod preprocessor;

pub use self::preprocessor::*;
use super::pipeline::{BindingSpace, FlatBinding};
use crate::renderer::backend::gl::api as gl;
use crate::renderer::backend::gl::api::types::*;
use crate::renderer::ShaderStageFlags;
use spirv_cross::{glsl, spirv};

//--------------------------------------------------------------------------------------------------
#[derive(Debug)]
pub struct ShaderModule {
    pub obj: GLuint,
    pub stage: ShaderStageFlags,
    /// SPIR-V bytecode of this shader. If this is not None, then obj is ignored
    /// (the shader is created during program creation).
    pub spirv: Option<Vec<u32>>,
}

//--------------------------------------------------------------------------------------------------
#[derive(Debug)]
pub struct ShaderCreationError(pub String);

impl fmt::Display for ShaderCreationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        fmt::Display::fmt(&self.0, f)
    }
}

impl Error for ShaderCreationError {}

//--------------------------------------------------------------------------------------------------
pub fn shader_stage_flags_to_glenum(stage: ShaderStageFlags) -> GLenum {
    match stage {
        ShaderStageFlags::VERTEX => gl::VERTEX_SHADER,
        ShaderStageFlags::FRAGMENT => gl::FRAGMENT_SHADER,
        ShaderStageFlags::GEOMETRY => gl::GEOMETRY_SHADER,
        ShaderStageFlags::TESS_CONTROL => gl::TESS_CONTROL_SHADER,
        ShaderStageFlags::TESS_EVAL => gl::TESS_EVALUATION_SHADER,
        ShaderStageFlags::COMPUTE => gl::COMPUTE_SHADER,
        _ => panic!("invalid shader stage"),
    }
}

fn get_shader_info_log(obj: GLuint) -> String {
    unsafe {
        let mut log_size = 0;
        let mut log_buf = Vec::with_capacity(log_size as usize);
        gl::GetShaderInfoLog(
            obj,
            log_size,
            &mut log_size,
            log_buf.as_mut_ptr() as *mut i8,
        );
        log_buf.set_len(log_size as usize);
        String::from_utf8(log_buf).unwrap()
    }
}

pub fn create_shader_from_glsl(
    stage: ShaderStageFlags,
    source: &[u8],
) -> Result<GLuint, ShaderCreationError> {
    let stage = shader_stage_flags_to_glenum(stage);
    unsafe {
        let obj = gl::CreateShader(stage);
        let sources = [source.as_ptr() as *const i8];
        let lengths = [source.len() as GLint];
        gl::ShaderSource(
            obj,
            1,
            &sources[0] as *const *const i8,
            &lengths[0] as *const GLint,
        );
        gl::CompileShader(obj);
        let mut status = 0;
        gl::GetShaderiv(obj, gl::COMPILE_STATUS, &mut status);
        if status != gl::TRUE as GLint {
            let log = get_shader_info_log(obj);
            gl::DeleteShader(obj);
            Err(ShaderCreationError(log))
        } else {
            Ok(obj)
        }
    }
}

pub fn create_specialized_spirv_shader(
    stage: ShaderStageFlags,
    entry_point: &str,
    bytecode: &[u32],
) -> Result<GLuint, ShaderCreationError> {
    let stage = shader_stage_flags_to_glenum(stage);
    let entry_point = CString::new(entry_point).unwrap();

    unsafe {
        let shader = gl::CreateShader(stage);
        gl::ShaderBinary(
            1,
            &shader,
            gl::SHADER_BINARY_FORMAT_SPIR_V,
            bytecode.as_ptr() as *const c_void,
            mem::size_of_val(bytecode) as i32,
        );

        gl::SpecializeShader(shader, entry_point.as_ptr(), 0, ptr::null(), ptr::null());
        let mut status = 0;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);
        if status != gl::TRUE as GLint {
            gl::DeleteShader(shader);
            let log = get_shader_info_log(shader);
            Err(ShaderCreationError(log))
        } else {
            Ok(shader)
        }
    }
}
/*
#[derive(Clone, Debug)]
pub struct DescriptorMap {
    pub sets: Vec<Vec<FlatBinding>>,
}

impl DescriptorMap {
    pub fn get_binding_location(&self, set: u32, binding: u32) -> Option<FlatBinding> {
        self.sets.get(set as usize).and_then(|set| {
            set.get(binding as usize).and_then(|loc| {
                if loc.space == BindingSpace::Empty {
                    None
                } else {
                    Some(*loc)
                }
            })
        })
    }

    fn insert(&mut self, set: u32, binding: u32, new_binding: FlatBinding) {
        let set = set as usize;
        if set >= self.sets.len() {
            self.sets.resize(set + 1, Vec::new());
        }

        let set = &mut sets[set];
        let binding = binding as usize;
        if binding >= set.len() {
            set.resize(
                binding + 1,
                FlatBinding {
                    space: BindingSpace::Empty,
                    location: 0,
                },
            );
        }

        set[binding] = new_binding;
    }

    fn new() -> DescriptorMap {
        DescriptorMap { sets: Vec::new() }
    }
}


/// Ported from gfx-rs
///
/// Translate SPIR-V bytecode into something that OpenGL can understand.
/// Does two things:
/// * 'Flattens' descriptor sets and bindings into a single binding number
/// * Builds image+sampler combinations (unimplemented)
pub fn translate_vulkan_spirv(
    spv: &[u32],
    stage: ShaderStageFlags,
    desc_map: &mut DescriptorMap
) -> Vec<u32>
{
    let module = spirv::Module::from_words(spv);
    // parse spirv
    let mut ast = spirv::Ast::parse(&module).unwrap();
    // translate into something that OpenGL can understand
    let res = ast.get_shader_resources().unwrap();
    remap_bindings(
        &mut ast,
        BindingSpace::UniformBuffer,
        &res.uniform_buffers,
        desc_map,
    );
    remap_bindings(
        &mut ast,
        BindingSpace::ShaderStorageBuffer,
        &res.storage_buffers,
        desc_map,
    );
    remap_bindings(&mut ast, BindingSpace::Image, &res.storage_images, desc_map);
    remap_bindings(
        &mut ast,
        BindingSpace::Texture,
        &res.sampled_images,
        desc_map,
    );


}

fn translate_spirv(ast: &mut spirv::Ast<glsl::Target>, desc_map: &mut DescriptorMap) {
    // flatten descriptor sets
    // TODO samplers (harder)
    // TODO atomic buffers
}

fn remap_bindings(
    ast: &mut spirv::Ast<glsl::Target>,
    space: BindingSpace,
    res: &[spirv::Resource],
    desc_map: &mut DescriptorMap,
) {
    let mut flat_binding = 0;
    for r in res.iter() {
        // must have set and binding decorations
        let set = ast
            .get_decoration(r.id, spirv::Decoration::DescriptorSet)
            .unwrap();
        let binding = ast
            .get_decoration(r.id, spirv::Decoration::Binding)
            .unwrap();
        desc_map.insert(
            set,
            binding,
            FlatBinding::new(BindingSpace::UniformBuffer, flat_binding),
        );
        flat_binding += 1;
        // remove set decoration, change binding
        ast.unset_decoration(r.id, spirv::Decoration::DescriptorSet);
        ast.set_decoration(r.id, spirv::Decoration::Binding, flat_binding);
    }
}
*/
