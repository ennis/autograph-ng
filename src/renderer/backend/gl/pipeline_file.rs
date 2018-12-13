//! Shader pipeline files.

use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::renderer;
use crate::renderer::backend::gl::shader::preprocessor::*;
use crate::renderer::backend::gl::{
    pipeline::{BindingLocation, BindingSpace, DescriptorMap},
    shader::ShaderModule,
    OpenGlBackend,
};
use crate::renderer::{
    Arena, Renderer, RendererBackend, ShaderStageFlags, VertexInputBindingDescription,
};

//--------------------------------------------------------------------------------------------------
struct ShaderSources {
    vs: Option<String>,
    fs: Option<String>,
    gs: Option<String>,
    tes: Option<String>,
    tcs: Option<String>,
    cs: Option<String>,
}

struct SpirvModules {
    vs: Option<Vec<u32>>,
    fs: Option<Vec<u32>>,
    gs: Option<Vec<u32>>,
    tes: Option<Vec<u32>>,
    tcs: Option<Vec<u32>>,
    cs: Option<Vec<u32>>,
}

//--------------------------------------------------------------------------------------------------
#[derive(Copy, Clone, Debug)]
struct SourceWithFileName<'a> {
    source: &'a str,
    file_name: &'a str,
}

impl<'a> SourceWithFileName<'a> {
    fn new(source: &'a str, file_name: &'a str) -> SourceWithFileName<'a> {
        SourceWithFileName { source, file_name }
    }
}

/// Compile a bunch of GLSL files to SPIR-V. File names are for better error reporting.
fn compile_glsl_to_spirv<'a>(
    version: u32,
    vertex: SourceWithFileName<'a>,
    fragment: SourceWithFileName<'a>,
    geometry: Option<SourceWithFileName<'a>>,
    tess_control: Option<SourceWithFileName<'a>>,
    tess_eval: Option<SourceWithFileName<'a>>,
) -> Result<SpirvModules, Box<Error>> {
    use shaderc;
    let mut compiler = shaderc::Compiler::new().unwrap();
    let mut options = shaderc::CompileOptions::new().unwrap();
    options.set_target_env(shaderc::TargetEnv::OpenGL, 0);
    options.set_forced_version_profile(version, shaderc::GlslProfile::None);
    options.set_optimization_level(shaderc::OptimizationLevel::Zero);

    let print_debug_info = |source: SourceWithFileName<'a>,
                            stage: ShaderStageFlags,
                            ca: &shaderc::CompilationArtifact| {
        debug!(
            "Successfully compiled shader to SPIR-V: {}, stage {:?}",
            source.file_name, stage
        );
        let nw = ca.get_num_warnings();
        if nw != 0 {
            debug!("warnings({}): ", nw);
            debug!("{}", ca.get_warning_messages());
        }
    };

    //debug!("==== Preprocessed ====\n\n{}", pp.vertex.as_ref().unwrap());

    let vertex_compile_result = compiler.compile_into_spirv(
        vertex.source,
        shaderc::ShaderKind::Vertex,
        vertex.file_name,
        "main",
        Some(&options),
    )?;
    print_debug_info(vertex, ShaderStageFlags::VERTEX, &vertex_compile_result);

    let fragment_compile_result = compiler.compile_into_spirv(
        fragment.source,
        shaderc::ShaderKind::Fragment,
        fragment.file_name,
        "main",
        Some(&options),
    )?;
    print_debug_info(fragment, ShaderStageFlags::FRAGMENT, &vertex_compile_result);

    let geometry_compile_result = if let Some(geometry) = geometry {
        let ca = compiler.compile_into_spirv(
            geometry.source,
            shaderc::ShaderKind::Geometry,
            geometry.file_name,
            "main",
            Some(&options),
        )?;
        print_debug_info(geometry, ShaderStageFlags::GEOMETRY, &ca);
        Some(ca)
    } else {
        None
    };
    let tess_control_compile_result = if let Some(tess_control) = tess_control {
        let ca = compiler.compile_into_spirv(
            tess_control.source,
            shaderc::ShaderKind::TessControl,
            tess_control.file_name,
            "main",
            Some(&options),
        )?;
        print_debug_info(tess_control, ShaderStageFlags::TESS_CONTROL, &ca);
        Some(ca)
    } else {
        None
    };
    let tess_eval_compile_result = if let Some(tess_eval) = tess_eval {
        let ca = compiler.compile_into_spirv(
            tess_eval.source,
            shaderc::ShaderKind::TessEvaluation,
            tess_eval.file_name,
            "main",
            Some(&options),
        )?;
        print_debug_info(tess_eval, ShaderStageFlags::TESS_EVAL, &ca);
        Some(ca)
    } else {
        None
    };

    Ok(SpirvModules {
        vs: Some(vertex_compile_result.as_binary().into()),
        fs: Some(fragment_compile_result.as_binary().into()),
        gs: geometry_compile_result.map(|gs| gs.as_binary().into()),
        tcs: tess_control_compile_result.map(|tcs| tcs.as_binary().into()),
        tes: tess_eval_compile_result.map(|tes| tes.as_binary().into()),
        cs: None,
    })
}

fn as_bytes(buf: &[u32]) -> &[u8] {
    unsafe { ::std::slice::from_raw_parts(buf.as_ptr() as *const u8, buf.len() * 4) }
}

//--------------------------------------------------------------------------------------------------
pub struct ShaderModules<'rcx> {
    pub vs: Option<renderer::ShaderModule<'rcx, OpenGlBackend>>,
    pub fs: Option<renderer::ShaderModule<'rcx, OpenGlBackend>>,
    pub gs: Option<renderer::ShaderModule<'rcx, OpenGlBackend>>,
    pub tes: Option<renderer::ShaderModule<'rcx, OpenGlBackend>>,
    pub tcs: Option<renderer::ShaderModule<'rcx, OpenGlBackend>>,
    pub cs: Option<renderer::ShaderModule<'rcx, OpenGlBackend>>,
}

pub struct PipelineDescriptionFile<'rcx> {
    pub source: String,
    pub path: Option<PathBuf>,
    pub preprocessed: PreprocessResult,
    pub descriptor_map: DescriptorMap,
    pub separate_sources: SeparateShaderSources,
    pub modules: ShaderModules<'rcx>,
    pub vertex_input_bindings: Vec<VertexInputBindingDescription>,
}

fn mappings_to_descriptor_map(mappings: &[ParsedDescriptorMapping]) -> DescriptorMap {
    let mut sets = Vec::new();

    for m in mappings {
        let set = m.set as usize;
        if set >= sets.len() {
            sets.resize(set + 1, Vec::new());
        }
        let set = &mut sets[set];
        let max_binding_rel = (m.gl_binding_range.1 - m.gl_binding_range.0) as usize;
        let max_binding = m.binding_base as usize + max_binding_rel;
        if max_binding >= set.len() {
            set.resize(
                max_binding + 1,
                BindingLocation {
                    space: BindingSpace::Empty,
                    location: 0,
                },
            );
        }
        for i in 0..=max_binding_rel {
            let ii = m.gl_binding_range.0 + i as u32;
            set[m.binding_base as usize + i] = BindingLocation {
                space: m.gl_binding_space,
                location: ii,
            };
        }
    }

    DescriptorMap { sets }
}

impl<'rcx> PipelineDescriptionFile<'rcx> {
    pub fn load<P: AsRef<Path>>(
        arena: &'rcx Arena<OpenGlBackend>,
        file_path: P,
    ) -> Result<PipelineDescriptionFile<'rcx>, Box<Error>> {
        let mut source = String::new();
        File::open(file_path.as_ref())?.read_to_string(&mut source)?;

        let preprocessed =
            preprocess_pipeline_description_file(&source, Some(file_path.as_ref()), &[])?;
        let version = preprocessed.version.unwrap_or_else(|| {
            warn!(
                "({:?}) no GLSL version specified, defaulting to 3.30",
                file_path.as_ref()
            );
            330
        });
        let separate_sources = SeparateShaderSources::from_combined_source(
            &preprocessed.preprocessed_source,
            version,
            preprocessed.stages,
            &[],
        );

        let descriptor_map = mappings_to_descriptor_map(&preprocessed.descriptor_map);

        let modules = {
            let file_path_str = file_path.as_ref().to_str().unwrap();
            let vertex_src = separate_sources
                .vertex
                .as_ref()
                .ok_or_else(|| "no vertex source".to_owned())?;
            let fragment_src = separate_sources
                .fragment
                .as_ref()
                .ok_or_else(|| "no vertex source".to_owned())?;
            let geometry_src = separate_sources.geometry.as_ref();
            let tess_control_src = separate_sources.tess_control.as_ref();
            let tess_eval_src = separate_sources.tess_eval.as_ref();

            let spirv = compile_glsl_to_spirv(
                version,
                SourceWithFileName::new(vertex_src, file_path_str),
                SourceWithFileName::new(fragment_src, file_path_str),
                geometry_src.map(|s| SourceWithFileName::new(s, file_path_str)),
                tess_control_src.map(|s| SourceWithFileName::new(s, file_path_str)),
                tess_eval_src.map(|s| SourceWithFileName::new(s, file_path_str)),
            )?;

            // create shaders
            ShaderModules {
                vs: spirv.vs.as_ref().map(|data| {
                    arena.create_shader_module(as_bytes(data), ShaderStageFlags::VERTEX)
                }),
                fs: spirv.fs.as_ref().map(|data| {
                    arena.create_shader_module(as_bytes(data), ShaderStageFlags::FRAGMENT)
                }),
                gs: spirv.gs.as_ref().map(|data| {
                    arena.create_shader_module(as_bytes(data), ShaderStageFlags::GEOMETRY)
                }),
                tcs: spirv.tcs.as_ref().map(|data| {
                    arena.create_shader_module(as_bytes(data), ShaderStageFlags::TESS_CONTROL)
                }),
                tes: spirv.tes.as_ref().map(|data| {
                    arena.create_shader_module(as_bytes(data), ShaderStageFlags::TESS_EVAL)
                }),
                cs: None,
            }
        };

        Ok(PipelineDescriptionFile {
            source,
            path: Some(file_path.as_ref().to_path_buf()),
            preprocessed,
            separate_sources,
            descriptor_map,
            modules,
            vertex_input_bindings: Vec::new(),
        })
    }

    pub fn with_vertex_input_bindings(
        &mut self,
        bindings: &[VertexInputBindingDescription],
    ) -> &mut Self {
        self.vertex_input_bindings = bindings.to_vec();
        self
    }
}

/*//--------------------------------------------------------------------------------------------------
impl<'a> From<&'a PipelineDescriptionFile> for GraphicsPipelineCreateInfo<'a, OpenGlBackend>
{
    fn from(p: &'a PipelineDescriptionFile) -> Self {

        let shader_stages = GraphicsPipelineShaderStages {
            vertex: p.modules.vs.unwrap(),
            geometry: p.modules.gs,
            fragment: p.modules.fs,
            tess_eval: p.modules.tes,
            tess_control: p.modules.tcs,
        };

        GraphicsPipelineCreateInfo {
            shader_stages,
            vertex_input_state: PipelineVertexInputStateCreateInfo {
                bindings: p.vertex_input_bindings.as_slice(),
                attributes: p.preprocessed.vertex_attributes.as_ref().unwrap().as_slice()
            },
            viewport_state: PipelineViewportStateCreateInfo {
                viewports_scissors: &[],
            },
            rasterization_state: PipelineRasterizationStateCreateInfo {
                depth_clamp_enable: false,
                rasterizer_discard_enable: false,
                polygon_mode: PolygonMode::Fill,
                cull_mode: CullModeFlags::NONE,
                depth_bias: DepthBias::Disabled,
                front_face: FrontFace::Clockwise,
                line_width: 1.0.into()
            },
            multisample_state: PipelineMultisampleStateCreateInfo {
                rasterization_samples: 0,
                sample_shading: SampleShading::Disabled,
                alpha_to_coverage_enable: false,
                alpha_to_one_enable: false
            },
            depth_stencil_state: PipelineDepthStencilStateCreateInfo {
                depth_test_enable: true,
                depth_write_enable: true,
                depth_compare_op: CompareOp::Less,
                depth_bounds_test: DepthBoundTest::Disabled,
                stencil_test: StencilTest::Disabled
            },
            input_assembly_state: (),
            color_blend_state: (),
            dynamic_state: (),
            pipeline_layout: (),
            attachment_layout: (),
            additional: ()
        }

    }
}
*/
