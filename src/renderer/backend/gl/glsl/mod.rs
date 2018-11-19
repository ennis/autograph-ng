use failure::Error;
use gfx;
use gfx::pipeline::GraphicsPipelineBuilder;
use gfx::pipeline::VertexAttribute;
use gfx::shader;
use gfx::shader_interface;
use gl;
use gl::types::*;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

// public for testing
pub mod interface;
mod preprocessor;
mod spirv_parse;
pub use self::preprocessor::{preprocess_combined_shader_source, PreprocessedShaders};

bitflags! {
    #[derive(Default)]
    pub struct PipelineStages: u32 {
        ///
        const PS_VERTEX = (1 << 0);
        const PS_GEOMETRY = (1 << 1);
        const PS_FRAGMENT = (1 << 2);
        const PS_TESS_CONTROL = (1 << 3);
        const PS_TESS_EVAL = (1 << 4);
        const PS_COMPUTE = (1 << 5);
    }
}

pub struct Shader {
    obj: GLuint,
    stage: GLenum,
}

fn get_shader_info_log(obj: GLuint) -> String {
    unsafe {
        let mut log_size: GLint = 0;
        let mut log_buf: Vec<u8> = Vec::with_capacity(log_size as usize);
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

impl Shader {
    pub fn compile(source: &str, stage: GLenum) -> Result<Shader, String> {
        unsafe {
            let obj = gl::CreateShader(stage);
            let srcs = [source.as_ptr() as *const i8];
            let lens = [source.len() as GLint];
            gl::ShaderSource(
                obj,
                1,
                &srcs[0] as *const *const i8,
                &lens[0] as *const GLint,
            );
            gl::CompileShader(obj);
            let mut status: GLint = 0;
            gl::GetShaderiv(obj, gl::COMPILE_STATUS, &mut status);
            if status != gl::TRUE as GLint {
                error!("Error compiling shader");
                let log = get_shader_info_log(obj);
                gl::DeleteShader(obj);
                Err(log)
            } else {
                Ok(Shader { stage, obj })
            }
        }
    }

    pub fn from_spirv(stage: GLenum, bytecode: &[u32]) -> Result<Shader, Error> {
        unsafe {
            let mut obj = gl::CreateShader(stage);
            gl::ShaderBinary(
                1,
                &mut obj,
                gl::SHADER_BINARY_FORMAT_SPIR_V,
                bytecode.as_ptr() as *const ::std::os::raw::c_void,
                ::std::mem::size_of_val(bytecode) as i32,
            );
            let entry_point = ::std::ffi::CString::new("main").unwrap();
            // TODO specialization constants
            gl::SpecializeShader(
                obj,
                entry_point.as_ptr(),
                0,
                0 as *const GLuint,
                0 as *const GLuint,
            );
            let mut status: GLint = 0;
            gl::GetShaderiv(obj, gl::COMPILE_STATUS, &mut status);
            if status != gl::TRUE as GLint {
                error!("Error loading SPIR-V shader");
                let log = get_shader_info_log(obj);
                gl::DeleteShader(obj);
                //Ok(Shader { stage, obj:0 })
                Err(format_err!("{}", log))
            } else {
                Ok(Shader { stage, obj })
            }
        }
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteShader(self.obj);
        }
    }
}

fn link_program(obj: GLuint) -> Result<GLuint, String> {
    unsafe {
        gl::LinkProgram(obj);
        let mut status: GLint = 0;
        let mut log_size: GLint = 0;
        gl::GetProgramiv(obj, gl::LINK_STATUS, &mut status);
        gl::GetProgramiv(obj, gl::INFO_LOG_LENGTH, &mut log_size);
        //trace!("LINK_STATUS: log_size: {}, status: {}", log_size, status);
        if status != gl::TRUE as GLint {
            let mut log_buf: Vec<u8> = Vec::with_capacity(log_size as usize);
            gl::GetProgramInfoLog(
                obj,
                log_size,
                &mut log_size,
                log_buf.as_mut_ptr() as *mut i8,
            );
            log_buf.set_len(log_size as usize);
            Err(String::from_utf8(log_buf).unwrap())
        } else {
            Ok(obj)
        }
    }
}

impl shader::Shader for Shader {}
impl shader::VertexShader for Shader {}
impl shader::FragmentShader for Shader {}
impl shader::GeometryShader for Shader {}
impl shader::TessControlShader for Shader {}
impl shader::TessEvalShader for Shader {}
impl shader::ComputeShader for Shader {}

pub struct GlslGraphicsShaderPipeline {
    pub vertex: Shader,
    pub fragment: Shader,
    pub geometry: Option<Shader>,
    pub tess_control: Option<Shader>,
    pub tess_eval: Option<Shader>,
    pub program: GLuint,
}

impl ::std::fmt::Debug for GlslGraphicsShaderPipeline {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        unimplemented!()
    }
}

impl Drop for GlslGraphicsShaderPipeline {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.program);
        }
    }
}

impl shader::GraphicsShaderPipeline for GlslGraphicsShaderPipeline {
    fn vertex_shader(&self) -> &shader::VertexShader {
        &self.vertex
    }

    fn fragment_shader(&self) -> &shader::FragmentShader {
        &self.fragment
    }

    fn geometry_shader(&self) -> Option<&shader::GeometryShader> {
        self.geometry.as_ref().map(|x| x as &shader::GeometryShader)
    }

    fn tess_control_shader(&self) -> Option<&shader::TessControlShader> {
        self.tess_control
            .as_ref()
            .map(|x| x as &shader::TessControlShader)
    }

    fn tess_eval_shader(&self) -> Option<&shader::TessEvalShader> {
        self.tess_eval
            .as_ref()
            .map(|x| x as &shader::TessEvalShader)
    }

    fn is_compatible_with(
        &self,
        interface: &shader_interface::ShaderInterfaceDesc,
    ) -> Result<(), Error> {
        unimplemented!()
    }

    fn get_program(&self) -> Result<GLuint, Error> {
        Ok(self.program)
    }
}

pub struct GlslCombinedSource {
    pub shader_pipeline: GlslGraphicsShaderPipeline,
    pub input_layout: Vec<gfx::VertexAttribute>,
    pub primitive_topology: GLenum,
}

impl ::std::fmt::Debug for GlslCombinedSource {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        unimplemented!()
    }
}

#[derive(Fail, Debug)]
#[fail(
    display = "Compilation of GLSL shader failed (path: {:?}, stage: {:?}).", source_path, stage
)]
struct GlslCompilationError {
    source_path: PathBuf,
    stage: PipelineStages,
    log: String,
}

/// The shader "compiler" for combined-source GLSL files, through the driver's GLSL compiler.
/// Loads a combined GLSL source from the given path and returns compiled OpenGL shaders along with some pipeline configuration.
/// Does not support interface checking.
pub fn create_pipeline_via_gl<P: AsRef<Path>>(
    combined_src_path: P,
) -> Result<GlslCombinedSource, Error> {
    // load combined shader source
    let mut src = String::new();
    File::open(combined_src_path.as_ref())?.read_to_string(&mut src)?;

    // preprocess combined source code
    let (_stages, pp) =
        preprocessor::preprocess_combined_shader_source(&src, combined_src_path.as_ref(), &[], &[]);

    // try to compile shaders
    let print_error_log = |log: &str, stage| {
        error!("====================================================================");
        error!(
            "Shader compilation error ({:?}) | stage: {:?}",
            combined_src_path.as_ref(),
            stage
        );
        error!("{}\n", log);
    };

    // Compile shaders
    let vertex = Shader::compile(&pp.vertex.unwrap(), gl::VERTEX_SHADER).map_err(|log| {
        print_error_log(&log, PS_VERTEX);
        GlslCompilationError {
            source_path: combined_src_path.as_ref().to_owned(),
            stage: PS_VERTEX,
            log,
        }
    })?;
    let fragment = Shader::compile(&pp.fragment.unwrap(), gl::FRAGMENT_SHADER).map_err(|log| {
        print_error_log(&log, PS_FRAGMENT);
        GlslCompilationError {
            source_path: combined_src_path.as_ref().to_owned(),
            stage: PS_FRAGMENT,
            log,
        }
    })?;

    let geometry = if let Some(ref geometry) = pp.geometry {
        Some(
            Shader::compile(&geometry, gl::GEOMETRY_SHADER).map_err(|log| {
                print_error_log(&log, PS_GEOMETRY);
                GlslCompilationError {
                    source_path: combined_src_path.as_ref().to_owned(),
                    stage: PS_GEOMETRY,
                    log,
                }
            })?,
        )
    } else {
        None
    };

    let tess_control = if let Some(ref tess_control) = pp.tess_control {
        Some(
            Shader::compile(&tess_control, gl::TESS_CONTROL_SHADER).map_err(|log| {
                print_error_log(&log, PS_TESS_CONTROL);
                GlslCompilationError {
                    source_path: combined_src_path.as_ref().to_owned(),
                    stage: PS_TESS_CONTROL,
                    log,
                }
            })?,
        )
    } else {
        None
    };

    let tess_eval = if let Some(ref tess_eval) = pp.tess_eval {
        Some(
            Shader::compile(&tess_eval, gl::TESS_EVALUATION_SHADER).map_err(|log| {
                print_error_log(&log, PS_TESS_EVAL);
                GlslCompilationError {
                    source_path: combined_src_path.as_ref().to_owned(),
                    stage: PS_TESS_EVAL,
                    log,
                }
            })?,
        )
    } else {
        None
    };

    // TODO: this leaks on error return
    let program = unsafe { gl::CreateProgram() };

    unsafe {
        gl::AttachShader(program, vertex.obj);
        gl::AttachShader(program, fragment.obj);
        if let Some(ref s) = geometry {
            gl::AttachShader(program, s.obj);
        }
        if let Some(ref s) = tess_control {
            gl::AttachShader(program, s.obj);
        }
        if let Some(ref s) = tess_eval {
            gl::AttachShader(program, s.obj);
        }
    }

    link_program(program).map_err(|log| {
        unsafe {
            gl::DeleteProgram(program);
        }
        format_err!("Program link failed: {}", log)
    })?;

    // Specify layout
    Ok(GlslCombinedSource {
        shader_pipeline: GlslGraphicsShaderPipeline {
            vertex,
            fragment,
            geometry,
            tess_control,
            tess_eval,
            program,
        },
        input_layout: pp.input_layout.ok_or(format_err!(
            "Missing input layout in combined shader source: {}",
            combined_src_path.as_ref().display()
        ))?,
        primitive_topology: pp.primitive_topology.ok_or(format_err!(
            "Missing primitive topology in combined shader source: {}",
            combined_src_path.as_ref().display()
        ))?,
    })
}

pub struct SpirvGraphicsShaderPipeline {
    pub vertex: Shader,
    pub fragment: Shader,
    pub geometry: Option<Shader>,
    pub tess_control: Option<Shader>,
    pub tess_eval: Option<Shader>,
    pub program: GLuint,
    pub spirv_modules: SpirvModules,
}

impl ::std::fmt::Debug for SpirvGraphicsShaderPipeline {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        unimplemented!()
    }
}

impl Drop for SpirvGraphicsShaderPipeline {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.program);
        }
    }
}

impl shader::GraphicsShaderPipeline for SpirvGraphicsShaderPipeline {
    fn vertex_shader(&self) -> &shader::VertexShader {
        &self.vertex
    }

    fn fragment_shader(&self) -> &shader::FragmentShader {
        &self.fragment
    }

    fn geometry_shader(&self) -> Option<&shader::GeometryShader> {
        self.geometry.as_ref().map(|x| x as &shader::GeometryShader)
    }

    fn tess_control_shader(&self) -> Option<&shader::TessControlShader> {
        self.tess_control
            .as_ref()
            .map(|x| x as &shader::TessControlShader)
    }

    fn tess_eval_shader(&self) -> Option<&shader::TessEvalShader> {
        self.tess_eval
            .as_ref()
            .map(|x| x as &shader::TessEvalShader)
    }

    fn is_compatible_with(
        &self,
        interface: &shader_interface::ShaderInterfaceDesc,
    ) -> Result<(), Error> {
        interface::verify_spirv_interface(
            interface,
            self.spirv_modules.vs.as_ref(),
            self.spirv_modules.fs.as_ref(),
            self.spirv_modules.gs.as_ref().map(|v| v.as_ref()),
            self.spirv_modules.tcs.as_ref().map(|v| v.as_ref()),
            self.spirv_modules.tes.as_ref().map(|v| v.as_ref()),
        )?;
        Ok(())
    }

    fn get_program(&self) -> Result<GLuint, Error> {
        Ok(self.program)
    }
}

impl SpirvGraphicsShaderPipeline {
    pub fn from_binary(spirv_modules: SpirvModules) -> Result<SpirvGraphicsShaderPipeline, Error> {
        let vertex = Shader::from_spirv(gl::VERTEX_SHADER, &spirv_modules.vs)?;
        let fragment = Shader::from_spirv(gl::FRAGMENT_SHADER, &spirv_modules.fs)?;
        let geometry = if let Some(ref gs) = spirv_modules.gs {
            Some(Shader::from_spirv(gl::GEOMETRY_SHADER, gs)?)
        } else {
            None
        };
        let tess_control = if let Some(ref tcs) = spirv_modules.tcs {
            Some(Shader::from_spirv(gl::TESS_CONTROL_SHADER, tcs)?)
        } else {
            None
        };
        let tess_eval = if let Some(ref tes) = spirv_modules.tes {
            Some(Shader::from_spirv(gl::TESS_EVALUATION_SHADER, tes)?)
        } else {
            None
        };

        // TODO: this leaks on error return
        let program = unsafe { gl::CreateProgram() };

        unsafe {
            gl::AttachShader(program, vertex.obj);
            gl::AttachShader(program, fragment.obj);
            if let Some(ref s) = geometry {
                gl::AttachShader(program, s.obj);
            }
            if let Some(ref s) = tess_control {
                gl::AttachShader(program, s.obj);
            }
            if let Some(ref s) = tess_eval {
                gl::AttachShader(program, s.obj);
            }
        }

        link_program(program).map_err(|log| {
            unsafe {
                gl::DeleteProgram(program);
            }
            format_err!("Program link failed: {}", log)
        })?;

        // get reflection data?
        Ok(SpirvGraphicsShaderPipeline {
            vertex,
            fragment,
            geometry,
            tess_control,
            tess_eval,
            program,
            spirv_modules,
        })
    }

    /*fn from_binaries(
        vert_bytecode: &[u32],
        frag_bytecode: &[u32],
        geom_bytecode: Option<&[u32]>,
        tcs_bytecode: Option<&[u32]>,
        tes_bytecode: Option<&[u32]>) -> Result<SpirvGraphicsShaderPipeline,Error>
    {
        Self::from_binary(SpirvModules {

        })
    }*/
}
/*
#[derive(Fail, Debug)]
#[fail(
    display = "Compilation of GLSL shader to SPIR-V failed (path: {:?}, stage: {:?}).",
    source_path,
    stage
)]
struct GlslViaSpirvCompilationError {
    source_path: PathBuf,
    stage: PipelineStages,
    log: String,
}
*/

use shaderc;

pub struct SpirvModules {
    //pub pp: preprocessor::PreprocessedShaders,
    pub vs: Vec<u32>,
    pub fs: Vec<u32>,
    pub gs: Option<Vec<u32>>,
    pub tcs: Option<Vec<u32>>,
    pub tes: Option<Vec<u32>>,
}

pub fn load_combined_shader_source<P: AsRef<Path>>(
    path: P,
) -> Result<preprocessor::PreprocessedShaders, Error> {
    // load combined shader source
    let mut src = String::new();
    File::open(path.as_ref())?.read_to_string(&mut src)?;

    // preprocess combined source code
    let (_stages, pp) =
        preprocessor::preprocess_combined_shader_source(&src, path.as_ref(), &[], &[]);

    Ok(pp)
}

pub struct SourceWithFileName<'a> {
    pub source: &'a str,
    pub file_name: &'a str,
}

/// Compile a bunch of GLSL files to SPIR-V. File names are for better error reporting.
pub fn compile_glsl_to_spirv<'a>(
    vert: SourceWithFileName<'a>,
    frag: SourceWithFileName<'a>,
    geom: Option<SourceWithFileName<'a>>,
    tess_control: Option<SourceWithFileName<'a>>,
    tess_eval: Option<SourceWithFileName<'a>>,
) -> Result<SpirvModules, Error> {
    // load combined shader source
    /*let mut src = String::new();
    File::open(combined_src_path.as_ref())?.read_to_string(&mut src)?;
    let src_path_str = combined_src_path.as_ref().to_str().unwrap();

    // preprocess combined source code
    let (_stages, pp) =
        preprocessor::preprocess_combined_shader_source(&src, combined_src_path.as_ref(), &[], &[]);*/

    // try to compile shaders

    use shaderc;
    let mut compiler = shaderc::Compiler::new().unwrap();
    let mut options = shaderc::CompileOptions::new().unwrap();
    options.set_target_env(shaderc::TargetEnv::OpenGL, 0);
    options.set_forced_version_profile(450, shaderc::GlslProfile::None);
    options.set_optimization_level(shaderc::OptimizationLevel::Zero);

    //debug!("==== Preprocessed ====\n\n{}", pp.vertex.as_ref().unwrap());

    let vertex_compile_result = compiler.compile_into_spirv(
        vert.source,
        shaderc::ShaderKind::Vertex,
        vert.file_name,
        "main",
        Some(&options),
    )?;
    /*let text_result = compiler.compile_into_spirv_assembly(
        &pp.vertex.unwrap(), shaderc::ShaderKind::Vertex,
        &src_path_str, "main", Some(&options))?;
    debug!("==== SPIR-V ====\n\n{}",text_result.as_text());*/

    let fragment_compile_result = compiler.compile_into_spirv(
        frag.source,
        shaderc::ShaderKind::Fragment,
        frag.file_name,
        "main",
        Some(&options),
    )?;
    let geometry_compile_result = if let Some(geom) = geom {
        Some(compiler.compile_into_spirv(
            geom.source,
            shaderc::ShaderKind::Geometry,
            geom.file_name,
            "main",
            Some(&options),
        )?)
    } else {
        None
    };
    let tess_control_compile_result = if let Some(tess_control) = tess_control {
        Some(compiler.compile_into_spirv(
            tess_control.source,
            shaderc::ShaderKind::TessControl,
            tess_control.file_name,
            "main",
            Some(&options),
        )?)
    } else {
        None
    };
    let tess_eval_compile_result = if let Some(tess_eval) = tess_eval {
        Some(compiler.compile_into_spirv(
            tess_eval.source,
            shaderc::ShaderKind::TessEvaluation,
            tess_eval.file_name,
            "main",
            Some(&options),
        )?)
    } else {
        None
    };

    Ok(SpirvModules {
        vs: vertex_compile_result.as_binary().into(),
        fs: fragment_compile_result.as_binary().into(),
        gs: geometry_compile_result.map(|gs| gs.as_binary().into()),
        tcs: tess_control_compile_result.map(|tcs| tcs.as_binary().into()),
        tes: tess_eval_compile_result.map(|tes| tes.as_binary().into()),
    })
}

pub trait GraphicsPipelineBuilderExt: Sized {
    /// Loads shaders from the GLSL combined source file specified by path.
    fn with_glsl_file<P: AsRef<Path>>(self, path: P) -> Result<Self, Error>;
    /// Loads shaders from the GLSL combined source file specified by path.
    fn with_glsl_file_via_spirv<P: AsRef<Path>>(self, path: P) -> Result<Self, Error>;
}

impl GraphicsPipelineBuilderExt for GraphicsPipelineBuilder {
    fn with_glsl_file<P: AsRef<Path>>(self, path: P) -> Result<Self, Error> {
        let compiled = create_pipeline_via_gl(path)?;

        let tmp = self
            .with_shader_pipeline(Box::new(compiled.shader_pipeline))
            .with_input_layout(compiled.input_layout)
            .with_primitive_topology(compiled.primitive_topology);

        Ok(tmp)
    }

    fn with_glsl_file_via_spirv<P: AsRef<Path>>(self, path: P) -> Result<Self, Error> {
        let pp = load_combined_shader_source(path.as_ref())?;
        let src_path_str = path.as_ref().to_str().unwrap();
        let spv_modules = compile_glsl_to_spirv(
            SourceWithFileName {
                source: pp.vertex.as_ref().ok_or(format_err!("No vertex shader defined in input file (missing `#pragma stages' directive?)"))?,
                file_name: &src_path_str,
            },
            SourceWithFileName {
                source: pp.fragment.as_ref().ok_or(format_err!("No fragment shader defined in input file (missing `#pragma stages' directive?)"))?,
                file_name: &src_path_str,
            },
            pp.geometry.as_ref().map(|geom| SourceWithFileName {
                source: geom,
                file_name: &src_path_str,
            }),
            pp.tess_control
                .as_ref()
                .map(|tess_control| SourceWithFileName {
                    source: tess_control,
                    file_name: &src_path_str,
                }),
            pp.tess_eval.as_ref().map(|tess_eval| SourceWithFileName {
                source: tess_eval,
                file_name: &src_path_str,
            }),
        )?;

        let spv_pipeline = SpirvGraphicsShaderPipeline::from_binary(spv_modules)?;

        let tmp = self
            .with_shader_pipeline(Box::new(spv_pipeline))
            .with_input_layout(pp.input_layout.ok_or(format_err!(
                "Missing input layout in combined shader source: {}",
                path.as_ref().display()
            ))?)
            .with_primitive_topology(pp.primitive_topology.ok_or(format_err!(
                "Missing primitive topology in combined shader source: {}",
                path.as_ref().display()
            ))?);

        Ok(tmp)
    }
}
