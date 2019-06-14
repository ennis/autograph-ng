#![feature(proc_macro_diagnostic)]
#![feature(proc_macro_span)]
extern crate proc_macro;
extern crate proc_macro2;

use lazy_static::lazy_static;
use proc_macro2::{Span, TokenStream};
use quote::{quote, TokenStreamExt};
use regex::Regex;
use shaderc::{self, IncludeType, ResolvedInclude};
use std::{
    cell::RefCell,
    fs,
    path::{Path, PathBuf},
};
use syn::export::ToTokens;

mod reflection;

//--------------------------------------------------------------------------------------------------
struct CrateName;
const G: CrateName = CrateName;

impl ToTokens for CrateName {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append(syn::Ident::new("autograph_api", Span::call_site()))
    }
}

//--------------------------------------------------------------------------------------------------
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
    let litstr: syn::LitStr = syn::parse_macro_input!(src);
    compile_glsl_shader(&litstr.value(), None, &litstr.span(), stage, false).into()
}

#[proc_macro]
pub fn include_glsl(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    include_glsl_inner(input, false)
}

#[proc_macro]
pub fn include_glsl_raw(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    include_glsl_inner(input, true)
}

fn include_glsl_inner(input: proc_macro::TokenStream, raw: bool) -> proc_macro::TokenStream {
    let rel_path_lit: syn::LitStr = syn::parse_macro_input!(input);
    let rel_path = PathBuf::from(rel_path_lit.value());

    let stage = match rel_path.extension() {
        Some(ext) if ext == "vert" => shaderc::ShaderKind::Vertex,
        Some(ext) if ext == "frag" => shaderc::ShaderKind::Fragment,
        Some(ext) if ext == "geom" => shaderc::ShaderKind::Geometry,
        Some(ext) if ext == "tese" => shaderc::ShaderKind::TessEvaluation,
        Some(ext) if ext == "tesc" => shaderc::ShaderKind::TessControl,
        Some(ext) if ext == "comp" => shaderc::ShaderKind::Compute,
        _ => panic!("cannot deduce shader stage from extension"),
    };

    // look in the same directory as the source file
    let rust_src_path = rel_path_lit.span().unstable().source_file().path();
    let path = rust_src_path.with_file_name(rel_path);
    let src = if let Ok(src) = fs::read_to_string(&path) {
        src
    } else {
        panic!("failed to open GLSL shader source")
    };

    let sh = compile_glsl_shader(&src, Some(&path), &rel_path_lit.span(), stage, raw);

    // include_str so that it is considered when tracking dirty files
    let q = quote! { (#sh, include_str!(#rel_path_lit)).0 };
    q.into()
}

fn resolve_include(
    current_path: &Path,
    include_rel_path: &str,
    include_type: IncludeType,
    _source_name: &str,
    _include_depth: usize,
    all_includes: &mut Vec<String>,
) -> Result<ResolvedInclude, String> {
    // we only handle relative includes for now
    if include_type != IncludeType::Relative {
        panic!("`#include <...>` is not yet supported: use relative include directives (`#include \"...\"`");
    }

    let include_path = current_path.with_file_name(include_rel_path);

    match fs::read_to_string(&include_path) {
        Ok(content) => {
            let resolved_name = include_path
                .canonicalize()
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned();
            all_includes.push(resolved_name.clone());
            Ok(ResolvedInclude {
                resolved_name,
                content,
            })
        }
        Err(e) => Err(format!("error reading include file: {}", e)),
    }
}

fn compile_glsl_shader(
    src: &str,
    file_path: Option<&Path>,
    span: &Span,
    stage: shaderc::ShaderKind,
    raw: bool,
) -> proc_macro2::TokenStream {
    // the doc says that we should preferably create one instance of the compiler
    // and reuse it, but I don't see a way to reuse a compiler instance
    // between macro invocations. Notably, it cannot be put into a lazy_static block wrapped
    // in a mutex since it complains that `*mut shaderc::ffi::ShadercCompiler` is not `Send`.
    let mut compiler = shaderc::Compiler::new().unwrap();
    let all_includes = RefCell::new(Vec::new()); // RefCell because include_callback is not FnMut
    let mut opt = shaderc::CompileOptions::new().unwrap();
    opt.set_target_env(shaderc::TargetEnv::Vulkan, 0);
    opt.set_optimization_level(shaderc::OptimizationLevel::Zero);
    opt.set_include_callback(|name, include_type, source_name, depth| {
        if let Some(file_path) = file_path {
            let mut all_includes = all_includes.borrow_mut();
            resolve_include(
                file_path,
                name,
                include_type,
                source_name,
                depth,
                &mut all_includes,
            )
        } else {
            panic!("`#include` is not supported on embedded shaders")
        }
    });

    // compile GLSL
    let compilation_artifact = if let Some(file_path) = file_path {
        let file_path_str = file_path.to_str().unwrap();
        compiler.compile_into_spirv(src, stage, file_path_str, "main", Some(&opt))
    } else {
        compiler.compile_into_spirv(src, stage, "embedded GLSL", "main", Some(&opt))
    };

    match compilation_artifact {
        // Failed to compile
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
                        let (path, fixup_line) = if let Some(file_path) = file_path {
                            // external
                            (file_path.to_owned(), err.line as usize)
                        } else {
                            // embedded, span is the span of the string literal
                            // FIXME this is totally wrong (line within string literal does not correspond to the line in the rust source)
                            (
                                span.unstable().source_file().path(),
                                span.unstable().start().line + err.line as usize - 1,
                            )
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
        // Compilation successful
        Ok(ca) => {
            // any warnings?
            if ca.get_num_warnings() != 0 {
                span.unstable()
                    .warning("warnings emitted during compilation")
                    .note(format!("compiler messages:\n{}", ca.get_warning_messages()));
            }

            let bin = ca.as_binary_u8();
            // write the bytecode as a byte string because quoting a u8 slice is surprisingly slow.
            let binstr = syn::LitByteStr::new(bin, span.clone());

            // include_str all include files so that their changes are tracked
            let a = all_includes.borrow();
            let a = a.iter();
            let q = quote!((#binstr, #(include_str!(#a)),*).0);

            if raw {
                // raw output, without reflection info
                q
            } else {
                // reflection info requested
                let refl = reflection::generate_reflection_info(span, bin, stage);
                quote! {
                    #G::pipeline::ReflectedShader {
                         bytecode: #q,
                         reflection: &#refl
                    }
                }
            }
        }
    }
}
