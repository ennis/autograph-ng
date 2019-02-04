use crate::api as gl;
use crate::api::types::*;
use crate::api::Gl;
use autograph_render::pipeline::ShaderStageFlags;
use autograph_spirv::TypeDesc;
use std::error::Error;
use std::ffi::CString;
use std::fmt;
use std::mem;
use std::os::raw::c_void;
use std::ptr;

//--------------------------------------------------------------------------------------------------
#[derive(Debug)]
pub struct GlShaderModule {
    pub(crate) obj: GLuint,
    pub(crate) stage: ShaderStageFlags,
    /// SPIR-V bytecode of this shader. If this is not None, then obj is ignored
    /// (the shader is created during program creation).
    pub(crate) spirv: Option<Vec<u32>>,
}

impl GlShaderModule {
    pub(crate) fn from_glsl(
        gl: &Gl,
        stage: ShaderStageFlags,
        source: &[u8],
    ) -> Result<GlShaderModule, ShaderCreationError> {
        let obj = create_shader_from_glsl(gl, stage, source)?;
        Ok(GlShaderModule {
            obj,
            spirv: None,
            stage,
        })
    }
}

//--------------------------------------------------------------------------------------------------
#[derive(Debug)]
pub(crate) struct ShaderCreationError(pub(crate) String);

impl fmt::Display for ShaderCreationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        fmt::Display::fmt(&self.0, f)
    }
}

impl Error for ShaderCreationError {}

//--------------------------------------------------------------------------------------------------
pub(crate) fn shader_stage_flags_to_glenum(stage: ShaderStageFlags) -> GLenum {
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

fn get_shader_info_log(gl: &Gl, obj: GLuint) -> String {
    unsafe {
        let mut log_size = 0;
        let mut log_buf = Vec::with_capacity(log_size as usize);
        gl.GetShaderInfoLog(
            obj,
            log_size,
            &mut log_size,
            log_buf.as_mut_ptr() as *mut i8,
        );
        log_buf.set_len(log_size as usize);
        String::from_utf8(log_buf).unwrap()
    }
}

pub(crate) fn create_shader_from_glsl(
    gl: &Gl,
    stage: ShaderStageFlags,
    source: &[u8],
) -> Result<GLuint, ShaderCreationError> {
    let stage = shader_stage_flags_to_glenum(stage);
    unsafe {
        let obj = gl.CreateShader(stage);
        let sources = [source.as_ptr() as *const i8];
        let lengths = [source.len() as GLint];
        gl.ShaderSource(
            obj,
            1,
            &sources[0] as *const *const i8,
            &lengths[0] as *const GLint,
        );
        gl.CompileShader(obj);
        let mut status = 0;
        gl.GetShaderiv(obj, gl::COMPILE_STATUS, &mut status);
        if status != gl::TRUE as GLint {
            let log = get_shader_info_log(gl, obj);
            gl.DeleteShader(obj);
            Err(ShaderCreationError(log))
        } else {
            Ok(obj)
        }
    }
}

pub(crate) fn create_specialized_spirv_shader(
    gl: &Gl,
    stage: ShaderStageFlags,
    entry_point: &str,
    bytecode: &[u32],
) -> Result<GLuint, ShaderCreationError> {
    let stage = shader_stage_flags_to_glenum(stage);
    let entry_point = CString::new(entry_point).unwrap();

    unsafe {
        let shader = gl.CreateShader(stage);
        gl.ShaderBinary(
            1,
            &shader,
            gl::SHADER_BINARY_FORMAT_SPIR_V,
            bytecode.as_ptr() as *const c_void,
            mem::size_of_val(bytecode) as i32,
        );

        gl.SpecializeShader(shader, entry_point.as_ptr(), 0, ptr::null(), ptr::null());
        let mut status = 0;
        gl.GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);
        if status != gl::TRUE as GLint {
            gl.DeleteShader(shader);
            let log = get_shader_info_log(gl, shader);
            Err(ShaderCreationError(log))
        } else {
            Ok(shader)
        }
    }
}

//--------------------------------------------------------------------------------------------------
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub(crate) enum BindingSpace {
    UniformBuffer,
    ShaderStorageBuffer,
    AtomicCounterBuffer,
    Texture,
    Image,
    Empty,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub(crate) struct FlatBinding {
    pub(crate) space: BindingSpace,
    pub(crate) location: u32,
}

/*impl FlatBinding {
    pub(crate) fn new(space: BindingSpace, location: u32) -> FlatBinding {
        FlatBinding { space, location }
    }
}*/

#[derive(Clone, Debug)]
pub(crate) struct DescriptorMap(pub(crate) Vec<Vec<FlatBinding>>);

impl DescriptorMap {
    pub fn get_binding_location(&self, set: u32, binding: u32) -> Option<FlatBinding> {
        self.0.get(set as usize).and_then(|s| {
            s.get(binding as usize).and_then(|loc| {
                if loc.space == BindingSpace::Empty {
                    None
                } else {
                    //eprintln!("(set={},binding={}) -> {:?}", set, binding, loc);
                    Some(*loc)
                }
            })
        })
    }
}

#[derive(Clone, Debug)]
pub struct DescriptorMapBuilder {
    sets: Vec<Vec<FlatBinding>>,
    next_tex: u32,
    next_img: u32,
    next_ssbo: u32,
    next_ubo: u32,
}

impl DescriptorMapBuilder {
    fn get_or_insert(&mut self, set: u32, binding: u32, space: BindingSpace) -> FlatBinding {
        let set = set as usize;
        if set >= self.sets.len() {
            self.sets.resize(set + 1, Vec::new());
        }

        let set = &mut self.sets[set];
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

        if set[binding].space != BindingSpace::Empty {
            set[binding]
        } else {
            let next = match space {
                BindingSpace::UniformBuffer => &mut self.next_ubo,
                BindingSpace::ShaderStorageBuffer => &mut self.next_ssbo,
                BindingSpace::AtomicCounterBuffer => unimplemented!(),
                BindingSpace::Texture => &mut self.next_tex,
                BindingSpace::Image => &mut self.next_img,
                _ => panic!("invalid binding space"),
            };
            let new = FlatBinding {
                space,
                location: *next,
            };
            *next += 1;
            set[binding] = new;
            new
        }
    }

    pub fn new() -> DescriptorMapBuilder {
        DescriptorMapBuilder {
            sets: Vec::new(),
            next_tex: 0,
            next_img: 0,
            next_ssbo: 0,
            next_ubo: 0,
        }
    }
}

impl From<DescriptorMapBuilder> for DescriptorMap {
    fn from(builder: DescriptorMapBuilder) -> Self {
        DescriptorMap(builder.sets)
    }
}

//--------------------------------------------------------------------------------------------------

/// Translate SPIR-V bytecode into something that OpenGL can understand.
///
/// Does two things:
/// * 'Flattens' descriptor sets and bindings into a single binding number
/// * Builds image+sampler combinations (unimplemented)
///
/// Ported from gfx-rs
pub fn translate_spirv_to_gl_flavor(
    spv: &[u32],
    _stage: ShaderStageFlags,
    desc_map: &mut DescriptorMapBuilder,
) -> Vec<u32> {
    use autograph_spirv as spirv;
    use spirv_headers::*;

    let m = spirv::Module::from_words(spv).expect("failed to load SPIR-V module");

    struct RemapEntry<'m> {
        space: BindingSpace,
        set: u32,
        binding: u32,
        iptr_set: spirv::IPtr<'m>,
        iptr_binding: spirv::IPtr<'m>,
        var_id: u32,
    }

    let mut to_remap = Vec::new();

    {
        // parse spirv
        let a = spirv::ast::Arenas::new();
        let ast = spirv::ast::Ast::new(&a, &m);

        for (_, v) in ast.variables() {
            debug!("{:?}", v);
            //let has_block_deco = v.has_block_decoration().is_some();
            let has_buffer_block_deco = v.has_buffer_block_decoration().is_some();

            let space = if v.storage == StorageClass::Uniform
            /*&& has_block_deco*/
            {
                BindingSpace::UniformBuffer
            } else if (v.storage == StorageClass::Uniform && has_buffer_block_deco)
                || (v.storage == StorageClass::StorageBuffer)
            {
                BindingSpace::ShaderStorageBuffer
            } else if v.storage == StorageClass::UniformConstant {
                if let &TypeDesc::Pointer(&TypeDesc::Image(_, _)) = v.ty {
                    BindingSpace::Image
                } else if let &TypeDesc::Pointer(&TypeDesc::SampledImage(_, _)) = v.ty {
                    BindingSpace::Texture
                } else {
                    continue;
                }
            } else {
                continue;
            };

            let (iptr_set, set) = v
                .descriptor_set_decoration()
                .expect("expected descriptor set decoration");
            let (iptr_binding, binding) =
                v.binding_decoration().expect("expected binding decoration");
            to_remap.push(RemapEntry {
                space,
                set,
                binding,
                iptr_set,
                iptr_binding,
                var_id: v.id,
            });
        }
        // drop AST
    }

    // sort descriptors by (set,binding)
    to_remap.sort_by(|a, b| (a.set, a.binding).cmp(&(b.set, b.binding)));

    // edit SPIR-V
    for RemapEntry {
        space,
        set,
        binding,
        iptr_set,
        iptr_binding,
        var_id,
    } in to_remap
    {
        let new_binding = desc_map.get_or_insert(set, binding, space);

        // remove descriptor set and binding, replace with GL binding
        m.edit_remove_instruction(iptr_set);
        m.edit_remove_instruction(iptr_binding);

        // Place the new decoration where the old was before.
        // We can't just append it to the end of the instruction stream,
        // as this violates the logical layout of instructions mandated by the SPIR-V spec.
        // (2.4. Logical Layout of a Module)
        m.edit_write_instruction(
            iptr_set,
            &spirv::inst::IDecorate {
                decoration: Decoration::Binding,
                params: &[new_binding.location],
                target_id: var_id,
            },
        );
    }

    // apply modifications
    let data = m.into_vec_and_apply_edits();
    /*let mut f = File::create("dump.spv").unwrap();
    let mut bw = BufWriter::new(f);
    for w in data.iter() {
        bw.write_u32::<byteorder::LE>(*w);
    }*/
    data
}
