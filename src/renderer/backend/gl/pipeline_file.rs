//! Shader pipeline files.

use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::renderer;
use crate::renderer::backend::gl::shader::preprocessor::*;
use crate::renderer::backend::gl::{
    pipeline::{BindingSpace, DescriptorMap, FlatBinding},
    shader::ShaderModule,
    OpenGlBackend,
};
use crate::renderer::{
    Arena, Renderer, RendererBackend, ShaderStageFlags, VertexInputBindingDescription,
};

//--------------------------------------------------------------------------------------------------
struct ShaderSources {
    vert: Option<String>,
    frag: Option<String>,
    geom: Option<String>,
    tesseval: Option<String>,
    tessctl: Option<String>,
    comp: Option<String>,
}

struct SpirvModules {
    vert: Option<Vec<u32>>,
    frag: Option<Vec<u32>>,
    geom: Option<Vec<u32>>,
    tesseval: Option<Vec<u32>>,
    tessctl: Option<Vec<u32>>,
    comp: Option<Vec<u32>>,
}

//--------------------------------------------------------------------------------------------------
#[derive(Copy, Clone, Debug)]
struct SourceWithFileName<'a> {
    src: &'a str,
    file: &'a str,
}

impl<'a> SourceWithFileName<'a> {
    fn new(src: &'a str, file: &'a str) -> SourceWithFileName<'a> {
        SourceWithFileName { src, file }
    }
}

/// Compile a bunch of GLSL files to SPIR-V. File names are for better error reporting.
fn compile_glsl_to_spirv<'a>(
    version: u32,
    vert: SourceWithFileName<'a>,
    frag: SourceWithFileName<'a>,
    geom: Option<SourceWithFileName<'a>>,
    tessctl: Option<SourceWithFileName<'a>>,
    tesseval: Option<SourceWithFileName<'a>>,
) -> Result<SpirvModules, Box<Error>> {
    use shaderc::{CompileOptions, Compiler, GlslProfile, OptimizationLevel, TargetEnv};

    let mut c = Compiler::new().unwrap();
    let mut opt = CompileOptions::new().unwrap();
    opt.set_target_env(TargetEnv::Vulkan, 0);
    opt.set_forced_version_profile(version, GlslProfile::None);
    opt.set_optimization_level(OptimizationLevel::Zero);

    let print_debug_info = |src: SourceWithFileName<'a>,
                            stage: ShaderStageFlags,
                            ca: &shaderc::CompilationArtifact| {
        debug!(
            "Successfully compiled shader to SPIR-V: {}, stage {:?}",
            src.file, stage
        );
        let nw = ca.get_num_warnings();
        if nw != 0 {
            debug!("warnings({}): ", nw);
            debug!("{}", ca.get_warning_messages());
        }
    };

    //debug!("==== Preprocessed ====\n\n{}", pp.vertex.as_ref().unwrap());

    let cvert = c.compile_into_spirv(
        vert.src,
        shaderc::ShaderKind::Vertex,
        vert.file,
        "main",
        Some(&opt),
    )?;
    print_debug_info(vert, ShaderStageFlags::VERTEX, &cvert);

    let cfrag = c.compile_into_spirv(
        frag.src,
        shaderc::ShaderKind::Fragment,
        frag.file,
        "main",
        Some(&opt),
    )?;
    print_debug_info(frag, ShaderStageFlags::FRAGMENT, &cfrag);

    let cgeom = if let Some(geom) = geom {
        let ca = c.compile_into_spirv(
            geom.src,
            shaderc::ShaderKind::Geometry,
            geom.file,
            "main",
            Some(&opt),
        )?;
        print_debug_info(geom, ShaderStageFlags::GEOMETRY, &ca);
        Some(ca)
    } else {
        None
    };
    let ctessctl = if let Some(tessctl) = tessctl {
        let ca = c.compile_into_spirv(
            tessctl.src,
            shaderc::ShaderKind::TessControl,
            tessctl.file,
            "main",
            Some(&opt),
        )?;
        print_debug_info(tessctl, ShaderStageFlags::TESS_CONTROL, &ca);
        Some(ca)
    } else {
        None
    };
    let ctesseval = if let Some(tesseval) = tesseval {
        let ca = c.compile_into_spirv(
            tesseval.src,
            shaderc::ShaderKind::TessEvaluation,
            tesseval.file,
            "main",
            Some(&opt),
        )?;
        print_debug_info(tesseval, ShaderStageFlags::TESS_EVAL, &ca);
        Some(ca)
    } else {
        None
    };

    Ok(SpirvModules {
        vert: Some(cvert.as_binary().into()),
        frag: Some(cfrag.as_binary().into()),
        geom: cgeom.map(|gs| gs.as_binary().into()),
        tessctl: ctessctl.map(|tcs| tcs.as_binary().into()),
        tesseval: ctesseval.map(|tes| tes.as_binary().into()),
        comp: None,
    })
}

fn as_bytes(buf: &[u32]) -> &[u8] {
    unsafe { ::std::slice::from_raw_parts(buf.as_ptr() as *const u8, buf.len() * 4) }
}

//--------------------------------------------------------------------------------------------------
pub struct ShaderModules<'rcx> {
    pub vert: Option<renderer::ShaderModule<'rcx, OpenGlBackend>>,
    pub frag: Option<renderer::ShaderModule<'rcx, OpenGlBackend>>,
    pub geom: Option<renderer::ShaderModule<'rcx, OpenGlBackend>>,
    pub tesseval: Option<renderer::ShaderModule<'rcx, OpenGlBackend>>,
    pub tessctl: Option<renderer::ShaderModule<'rcx, OpenGlBackend>>,
    pub comp: Option<renderer::ShaderModule<'rcx, OpenGlBackend>>,
}

pub struct PipelineDescriptionFile<'rcx> {
    pub src: String,
    pub path: Option<PathBuf>,
    pub pp: PreprocessResult,
    pub desc_map: DescriptorMap,
    pub sep: SeparateShaderSources,
    pub modules: ShaderModules<'rcx>,
    pub vtx_bindings: Vec<VertexInputBindingDescription>,
}

fn mappings_to_descriptor_map(mappings: &[ParsedDescriptorMapping]) -> DescriptorMap {
    let mut sets = Vec::new();

    for m in mappings {
        let set = m.set as usize;
        if set >= sets.len() {
            sets.resize(set + 1, Vec::new());
        }
        let set = &mut sets[set];
        let max_binding_rel = (m.gl_range.1 - m.gl_range.0) as usize;
        let max_binding = m.binding_base as usize + max_binding_rel;
        if max_binding >= set.len() {
            set.resize(
                max_binding + 1,
                FlatBinding {
                    space: BindingSpace::Empty,
                    location: 0,
                },
            );
        }
        for i in 0..=max_binding_rel {
            let ii = m.gl_range.0 + i as u32;
            set[m.binding_base as usize + i] = FlatBinding {
                space: m.gl_space,
                location: ii,
            };
        }
    }

    DescriptorMap { sets }
}

impl<'rcx> PipelineDescriptionFile<'rcx> {
    pub fn load<P: AsRef<Path>>(
        arena: &'rcx Arena<OpenGlBackend>,
        path: P,
    ) -> Result<PipelineDescriptionFile<'rcx>, Box<Error>> {
        let mut src = String::new();
        File::open(path.as_ref())?.read_to_string(&mut src)?;

        let pp = preprocess_pipeline_description_file(&src, Some(path.as_ref()), &[])?;
        let version = pp.version.unwrap_or_else(|| {
            warn!(
                "({:?}) no GLSL version specified, defaulting to 3.30",
                path.as_ref()
            );
            330
        });
        let sep = SeparateShaderSources::from_combined_source(&pp.srcpp, version, pp.stages, &[]);

        let desc_map = mappings_to_descriptor_map(&pp.desc_map);

        let modules = {
            let path_str = path.as_ref().to_str().unwrap();
            let vert = sep
                .vert
                .as_ref()
                .ok_or_else(|| "no vertex source".to_owned())?;
            let frag = sep
                .frag
                .as_ref()
                .ok_or_else(|| "no fragment source".to_owned())?;
            let geom = sep.geom.as_ref();
            let tessctl = sep.tessctl.as_ref();
            let tesseval = sep.tesseval.as_ref();

            let spirv = compile_glsl_to_spirv(
                version,
                SourceWithFileName::new(vert, path_str),
                SourceWithFileName::new(frag, path_str),
                geom.map(|s| SourceWithFileName::new(s, path_str)),
                tessctl.map(|s| SourceWithFileName::new(s, path_str)),
                tesseval.map(|s| SourceWithFileName::new(s, path_str)),
            )?;

            // create shaders
            ShaderModules {
                vert: spirv.vert.as_ref().map(|data| {
                    arena.create_shader_module(as_bytes(data), ShaderStageFlags::VERTEX)
                }),
                frag: spirv.frag.as_ref().map(|data| {
                    arena.create_shader_module(as_bytes(data), ShaderStageFlags::FRAGMENT)
                }),
                geom: spirv.geom.as_ref().map(|data| {
                    arena.create_shader_module(as_bytes(data), ShaderStageFlags::GEOMETRY)
                }),
                tessctl: spirv.tessctl.as_ref().map(|data| {
                    arena.create_shader_module(as_bytes(data), ShaderStageFlags::TESS_CONTROL)
                }),
                tesseval: spirv.tesseval.as_ref().map(|data| {
                    arena.create_shader_module(as_bytes(data), ShaderStageFlags::TESS_EVAL)
                }),
                comp: None,
            }
        };

        Ok(PipelineDescriptionFile {
            src,
            path: Some(path.as_ref().to_path_buf()),
            pp,
            sep,
            desc_map,
            modules,
            vtx_bindings: Vec::new(),
        })
    }
}
