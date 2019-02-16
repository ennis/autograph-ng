#![feature(proc_macro_diagnostic)]
#![feature(proc_macro_span)]
extern crate proc_macro;
extern crate proc_macro2;

use lazy_static::lazy_static;
use proc_macro2::Span;
use quote::quote;
use regex::Regex;
use shaderc;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use syn;
//use syn::spanned::Spanned;
use shaderc::IncludeType;
use shaderc::ResolvedInclude;

lazy_static! {
    static ref RE_COMPILE_ERROR: Regex =
        Regex::new(r#"^(?P<srcid>[^:]*):(?P<line>[^:]*):(?P<msg>.*)$"#).unwrap();
}

struct ShaderCompilationError {
    _srcid: String,
    line: u32,
    msg: String,
}

impl ShaderCompilationError {
    pub fn from_log_line(line: &str) -> Option<ShaderCompilationError> {
        if let Some(c) = RE_COMPILE_ERROR.captures(line) {
            Some(ShaderCompilationError {
                line: (&c["line"]).parse::<u32>().unwrap_or(0),
                _srcid: c["srcid"].to_string(),
                msg: c["msg"].to_string(),
            })
        } else {
            // Failed to parse
            None
        }
    }

    pub fn from_log(log: &str) -> Vec<ShaderCompilationError> {
        log.lines()
            .flat_map(|line| Self::from_log_line(line))
            .collect()
    }
}

#[proc_macro]
pub fn glsl_vertex(src: proc_macro::TokenStream) -> proc_macro::TokenStream {
    compile_embedded_shader(src, shaderc::ShaderKind::Vertex)
}
#[proc_macro]
pub fn glsl_fragment(src: proc_macro::TokenStream) -> proc_macro::TokenStream {
    compile_embedded_shader(src, shaderc::ShaderKind::Fragment)
}

#[proc_macro]
pub fn glsl_geometry(src: proc_macro::TokenStream) -> proc_macro::TokenStream {
    compile_embedded_shader(src, shaderc::ShaderKind::Geometry)
}

#[proc_macro]
pub fn glsl_tess_control(src: proc_macro::TokenStream) -> proc_macro::TokenStream {
    compile_embedded_shader(src, shaderc::ShaderKind::TessControl)
}

#[proc_macro]
pub fn glsl_tess_eval(src: proc_macro::TokenStream) -> proc_macro::TokenStream {
    compile_embedded_shader(src, shaderc::ShaderKind::TessEvaluation)
}

#[proc_macro]
pub fn glsl_compute(src: proc_macro::TokenStream) -> proc_macro::TokenStream {
    compile_embedded_shader(src, shaderc::ShaderKind::Compute)
}

fn compile_embedded_shader(
    src: proc_macro::TokenStream,
    stage: shaderc::ShaderKind,
) -> proc_macro::TokenStream {
    // parse a string literal
    let litstr = syn::parse_macro_input!(src as syn::LitStr);
    compile_glsl_shader(
        &litstr.value(),
        &litstr.span(),
        "<embedded GLSL>",
        Some(&litstr.span()),
        stage,
    )
    .into()
}

#[proc_macro]
pub fn include_shader(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let pathlit: syn::LitStr = syn::parse_macro_input!(input as syn::LitStr);
    let filename = PathBuf::from(pathlit.value());

    let stage = match filename.extension() {
        Some(ext) if ext == "vert" => shaderc::ShaderKind::Vertex,
        Some(ext) if ext == "frag" => shaderc::ShaderKind::Fragment,
        Some(ext) if ext == "geom" => shaderc::ShaderKind::Geometry,
        Some(ext) if ext == "tese" => shaderc::ShaderKind::TessEvaluation,
        Some(ext) if ext == "tesc" => shaderc::ShaderKind::TessControl,
        Some(ext) if ext == "comp" => shaderc::ShaderKind::Compute,
        _ => {
            return syn::Error::new(pathlit.span(), "cannot deduce shader stage from extension")
                .to_compile_error()
                .into();
        }
    };

    // look in the same directory as the source file
    let mut path = pathlit.span().unstable().source_file().path();
    path.set_file_name(&filename);

    let src = fs::read_to_string(&path);
    let src = if let Ok(src) = src {
        src
    } else {
        return syn::Error::new(pathlit.span(), "failed to open GLSL shader source")
            .to_compile_error()
            .into();
    };

    let bytecode = compile_glsl_shader(
        &src,
        &pathlit.span(),
        path.as_os_str().to_str().unwrap(),
        None,
        stage,
    );

    // include_str so that it is considered when tracking dirty files
    let q = quote! { (include_str!(#pathlit), #bytecode).1 };
    q.into()
}

fn compile_glsl_shader(
    src: &str,
    span: &Span,
    file: &str,
    file_span: Option<&Span>,
    stage: shaderc::ShaderKind,
) -> proc_macro2::TokenStream {
    // the doc says that we should preferably create one instance of the compiler
    // and reuse it, but I don't see a way to reuse a compiler instance
    // between macro invocations. Notably, it cannot be put into a lazy_static block wrapped
    // in a mutex since it complains that `*mut shaderc::ffi::ShadercCompiler` is not `Send`.
    let mut compiler = shaderc::Compiler::new().unwrap();
    let mut opt = shaderc::CompileOptions::new().unwrap();
    opt.set_target_env(shaderc::TargetEnv::Vulkan, 0);
    opt.set_optimization_level(shaderc::OptimizationLevel::Zero);
    opt.set_include_callback(
        |name: &str, _include_type: IncludeType, _source_name: &str, _depth: usize| {
            let path = Path::new(file);
            let mut inc = path.parent().unwrap().to_owned();
            inc.push(name);

            match File::open(&inc) {
                Ok(mut incfile) => {
                    let mut content = String::new();
                    incfile.read_to_string(&mut content).unwrap();
                    Ok(ResolvedInclude {
                        resolved_name: inc.canonicalize().unwrap().to_str().unwrap().to_owned(),
                        content,
                    })
                }
                Err(_e) => Err("include file not found".to_owned()),
            }
        },
    );

    let ca = compiler.compile_into_spirv(&src, stage, file, "main", Some(&opt));

    match ca {
        Err(ref e) => {
            let mut diag = span
                .unstable()
                .error("error(s) encountered while compiling GLSL shader");

            match e {
                shaderc::Error::CompilationError(num_err, log) => {
                    // With raw strings, there is a way to build a span to the location of the error,
                    // and produce a span_note with context for extra shiny diagnostics.
                    // Unfortunately, there is no way to do so with external files, so don't do it
                    // for now. (see https://github.com/rust-lang/rust/issues/55904)

                    // compilation errors, try to parse log to get more precise info
                    let parsed = ShaderCompilationError::from_log(log);
                    // fixup line
                    for err in parsed.iter() {
                        let (path, fixup_line) = if let Some(s) = file_span {
                            (
                                s.unstable().source_file().path(),
                                span.unstable().start().line + err.line as usize - 1,
                            )
                        } else {
                            (PathBuf::from(file.to_string()), err.line as usize)
                        };
                        // mimic the format of rustc diagnostics so that my IDE can pick them up...
                        diag = diag.note(format!(
                            "{}\n  --> {}:{}:",
                            err.msg,
                            path.display(),
                            fixup_line
                        ));
                    }

                    if parsed.len() != *num_err as usize {
                        // we did not parse them all, print full log for good measure
                        diag = diag.note(format!("full error log: {}", e));
                    }
                }
                _ => {
                    diag = diag.note(format!("{}", e));
                }
            }

            diag.emit();
            quote!(&[])
        }
        Ok(ca) => {
            // any warnings?
            let nw = ca.get_num_warnings();
            if nw != 0 {
                span.unstable()
                    .warning("warnings emitted during compilation")
                    .note(format!("compiler messages:\n{}", ca.get_warning_messages()));
            }

            let words = ca.as_binary_u8();

            quote!(&[ #(#words,)* ])
        }
    }
}
