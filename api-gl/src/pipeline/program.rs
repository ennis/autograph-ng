use super::shader::{
    create_specialized_spirv_shader, translate_spirv_to_gl_flavor, DescriptorMap,
    DescriptorMapBuilder, GlShaderModule, ShaderCreationError,
};
use crate::{
    api as gl,
    api::{types::*, Gl},
};
use autograph_api::pipeline::ShaderStageFlags;
use std::{error::Error, fmt};

//--------------------------------------------------------------------------------------------------
fn link_program(gl: &Gl, obj: GLuint) -> Result<GLuint, String> {
    unsafe {
        gl.LinkProgram(obj);
        let mut status = 0;
        let mut log_size = 0;
        gl.GetProgramiv(obj, gl::LINK_STATUS, &mut status);
        gl.GetProgramiv(obj, gl::INFO_LOG_LENGTH, &mut log_size);
        //trace!("LINK_STATUS: log_size: {}, status: {}", log_size, status);
        if status != gl::TRUE as GLint {
            let mut log_buf = Vec::with_capacity(log_size as usize);
            gl.GetProgramInfoLog(
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

//--------------------------------------------------------------------------------------------------
#[derive(Debug)]
pub struct ProgramCreationError(String);

impl fmt::Display for ProgramCreationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        fmt::Display::fmt(&self.0, f)
    }
}

impl Error for ProgramCreationError {}

impl From<ShaderCreationError> for ProgramCreationError {
    fn from(err: ShaderCreationError) -> Self {
        ProgramCreationError(err.0)
    }
}

pub(crate) fn create_graphics_program(
    gl: &Gl,
    vert: &GlShaderModule,
    frag: Option<&GlShaderModule>,
    geom: Option<&GlShaderModule>,
    tessctl: Option<&GlShaderModule>,
    tesseval: Option<&GlShaderModule>,
    //user_dm: DescriptorMap,
) -> Result<(GLuint, DescriptorMap), ProgramCreationError> {
    let spirv = vert.spirv.is_some();

    // Verify that we are not mixing GLSL and SPIR-V shaders
    if frag.map_or(false, |s| s.spirv.is_some() != spirv)
        || geom.map_or(false, |s| s.spirv.is_some() != spirv)
        || tessctl.map_or(false, |s| s.spirv.is_some() != spirv)
        || tesseval.map_or(false, |s| s.spirv.is_some() != spirv)
    {
        return Err(ProgramCreationError(
            "cannot mix both SPIR-V and GLSL shaders".into(),
        ));
    }

    let (vs, fs, gs, tcs, tes, dm) = if spirv {
        // SPIR-V path: translate to GL dialect and at the same time build
        // the descriptor map
        let mut dmb = DescriptorMapBuilder::new();
        let vert = vert.spirv.as_ref().unwrap();
        let frag = frag.map(|s| s.spirv.as_ref().unwrap());
        let geom = geom.map(|s| s.spirv.as_ref().unwrap());
        let tessctl = tessctl.map(|s| s.spirv.as_ref().unwrap());
        let tesseval = tesseval.map(|s| s.spirv.as_ref().unwrap());

        let vs = {
            let vert = translate_spirv_to_gl_flavor(vert, ShaderStageFlags::VERTEX, &mut dmb);
            create_specialized_spirv_shader(gl, ShaderStageFlags::VERTEX, "main", &vert)?
        };

        let fs = if let Some(s) = frag {
            let s = translate_spirv_to_gl_flavor(s, ShaderStageFlags::FRAGMENT, &mut dmb);
            create_specialized_spirv_shader(gl, ShaderStageFlags::FRAGMENT, "main", &s)?.into()
        } else {
            None
        };

        let gs = if let Some(s) = geom {
            let s = translate_spirv_to_gl_flavor(s, ShaderStageFlags::GEOMETRY, &mut dmb);
            create_specialized_spirv_shader(gl, ShaderStageFlags::GEOMETRY, "main", &s)?.into()
        } else {
            None
        };
        let tcs = if let Some(s) = tessctl {
            let s = translate_spirv_to_gl_flavor(s, ShaderStageFlags::TESS_CONTROL, &mut dmb);
            create_specialized_spirv_shader(gl, ShaderStageFlags::TESS_CONTROL, "main", &s)?.into()
        } else {
            None
        };
        let tes = if let Some(s) = tesseval {
            let s = translate_spirv_to_gl_flavor(s, ShaderStageFlags::TESS_EVAL, &mut dmb);
            create_specialized_spirv_shader(gl, ShaderStageFlags::TESS_EVAL, "main", &s)?.into()
        } else {
            None
        };

        // overwrite user-provided descriptor map
        let dm = dmb.into();
        debug!("inferred descriptor map: {:#?}", dm);
        (vs, fs, gs, tcs, tes, dm)
    } else {
        // GLSL path
        unimplemented!("descriptor map for GLSL compilation path")
        /*(
            vert.obj,
            frag.map(|s| s.obj),
            geom.map(|s| s.obj),
            tessctl.map(|s| s.obj),
            tesseval.map(|s| s.obj),
        )*/
    };

    // create program, attach shaders, and link program
    unsafe {
        let program = gl.CreateProgram();

        gl.AttachShader(program, vs);
        if let Some(s) = fs {
            gl.AttachShader(program, s);
        }
        if let Some(s) = gs {
            gl.AttachShader(program, s);
        }
        if let Some(s) = tcs {
            gl.AttachShader(program, s);
        }
        if let Some(s) = tes {
            gl.AttachShader(program, s);
        }

        link_program(gl, program).map_err(|log| {
            // cleanup
            gl.DeleteProgram(program);
            // the SPIR-V path has generated new shader objects: don't leak them
            if spirv {
                gl.DeleteShader(vs);
                if let Some(s) = fs {
                    gl.DeleteShader(s);
                }
                if let Some(s) = gs {
                    gl.DeleteShader(s);
                }
                if let Some(s) = tcs {
                    gl.DeleteShader(s);
                }
                if let Some(s) = tes {
                    gl.DeleteShader(s);
                }
            }

            ProgramCreationError(format!("program link error: {}", log))
        })?;

        if spirv {
            // cleanup
            gl.DeleteShader(vs);
            if let Some(s) = fs {
                gl.DeleteShader(s);
            }
            if let Some(s) = gs {
                gl.DeleteShader(s);
            }
            if let Some(s) = tcs {
                gl.DeleteShader(s);
            }
            if let Some(s) = tes {
                gl.DeleteShader(s);
            }
        }

        Ok((program, dm))
    }
}
