#![feature(proc_macro_diagnostic)]
#![feature(proc_macro_span)]
extern crate proc_macro;
extern crate proc_macro2;

use lazy_static::lazy_static;
use proc_macro2::Span;
use quote::quote;
use regex::Regex;
use shaderc;
use shaderc::ResolvedInclude;
use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use syn;

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
    let q = quote! { (#bytecode, include_str!(#pathlit)).0 };

    q.into()

    /*let words = vec![0u8; 10000];
    let words = &words;
    let time_begin = time::Instant::now();
    let q = quote!(&[ #(#words,)*]);
    let elapsed = time_begin.elapsed();
    let sec = (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1_000_000_000.0);
    pathlit.span().unstable().warning(format!("compile_into_spirv took {}s", sec))
        .note(format!("bytecode size: {} bytes", words.len()))
        .emit();

    q.into()*/
}

fn compile_glsl_shader<'a, 'b, 'c>(
    src: &'a str,
    span: &'b Span,
    file: &'c str,
    file_span: Option<&Span>,
    stage: shaderc::ShaderKind,
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
    opt.set_include_callback(|name, _include_type, _source_name, _depth| {
        let path = Path::new(file);
        let mut inc = path.parent().unwrap().to_owned();
        inc.push(name);

        match fs::read_to_string(&inc) {
            Ok(content) => {
                let resolved_name = inc.canonicalize().unwrap().to_str().unwrap().to_owned();
                all_includes.borrow_mut().push(resolved_name.clone());
                Ok(ResolvedInclude {
                    resolved_name,
                    content,
                })
            }
            Err(e) => Err(format!("error reading include file: {}", e)),
        }
    });

    // compile GLSL (and output time)
    let ca = compiler.compile_into_spirv(&src, stage, file, "main", Some(&opt));

    match ca {
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

            //let time_begin = time::Instant::now();
            let q = quote!((#binstr, #(include_str!(#a)),*).0);
            //let elapsed = time_begin.elapsed();
            //let sec = (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1_000_000_000.0);
            //span.unstable().warning(format!("compile_into_spirv took {}s", sec))
            //    .note(format!("bytecode size: {} bytes", bin.len()))
            //    .emit();

            q
        }
    }
}
