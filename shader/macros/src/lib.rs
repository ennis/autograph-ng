#![feature(proc_macro_diagnostic)]
#![feature(proc_macro_span)]
extern crate proc_macro;
extern crate proc_macro2;

use lazy_static::lazy_static;
use proc_macro2::Span;
//use proc_macro2::TokenStream;
use quote::quote;
use regex::Regex;
use shaderc;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use syn;
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;
//use syn::spanned::Spanned;
use shaderc::IncludeType;
use shaderc::ResolvedInclude;
use syn::Token;

mod preprocessor;

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
    compile_shader(src, shaderc::ShaderKind::Vertex)
}
#[proc_macro]
pub fn glsl_fragment(src: proc_macro::TokenStream) -> proc_macro::TokenStream {
    compile_shader(src, shaderc::ShaderKind::Fragment)
}
#[proc_macro]
pub fn glsl_geometry(src: proc_macro::TokenStream) -> proc_macro::TokenStream {
    compile_shader(src, shaderc::ShaderKind::Geometry)
}
#[proc_macro]
pub fn glsl_tess_control(src: proc_macro::TokenStream) -> proc_macro::TokenStream {
    compile_shader(src, shaderc::ShaderKind::TessControl)
}
#[proc_macro]
pub fn glsl_tess_eval(src: proc_macro::TokenStream) -> proc_macro::TokenStream {
    compile_shader(src, shaderc::ShaderKind::TessEvaluation)
}
#[proc_macro]
pub fn glsl_compute(src: proc_macro::TokenStream) -> proc_macro::TokenStream {
    compile_shader(src, shaderc::ShaderKind::Compute)
}

fn compile_shader(
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
        |name: &str, include_type: IncludeType, source_name: &str, depth: usize| {
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

    compile_glsl_shader_inner(&mut compiler, &opt, src, span, file, file_span, stage)
}

fn compile_glsl_shader_inner(
    compiler: &mut shaderc::Compiler,
    opts: &shaderc::CompileOptions,
    src: &str,
    span: &Span,
    file: &str,
    file_span: Option<&Span>,
    stage: shaderc::ShaderKind,
) -> proc_macro2::TokenStream {
    let ca = compiler.compile_into_spirv(&src, stage, file, "main", Some(&opts));

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

                    /*
                    let mut lit = proc_macro::Literal::string(src);
                    lit.set_span(span.unstable());
                    // compilation errors, try to parse log to get more precise info
                    let parsed = ShaderCompilationError::from_log(log);
                    // fix spans and add them as notes
                    for err in parsed.iter() {
                        // FIXME this is really just a best-effort solution
                        // find beginning of line
                        let span_begin = if err.line == 1 || err.line == 0 {
                            2 // FIXME HACK skip raw-string lead-in : r\"
                        } else {
                            src.match_indices('\n')
                                .nth(err.line as usize - 2)
                                .map(|(i, _)| i + 3) // FIXME HACK +2 to skip the r\" for raw strings, +1 to skip NL
                                .unwrap_or(0)
                        };
                        let span2 = lit
                            .subspan(span_begin..=span_begin)
                            .unwrap_or(span.unstable());

                        diag = diag.span_note(span2, format!("<GLSL>:{}: {}", err.line, err.msg));
                        //let file_path = span.unstable().source_file().path();
                        //diag = diag.note(format!("{}\n  --> {}:{}:", err.msg, span.unstable().source_file().path().display(), err.line));
                    }*/

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

struct CombinedShader {
    visibility: syn::Visibility,
    name: syn::Ident,
    path: syn::LitStr,
    stages: Punctuated<syn::Ident, Token![,]>,
}

impl syn::parse::Parse for CombinedShader {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        let visibility: syn::Visibility = input.parse()?;
        let name: syn::Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let path: syn::LitStr = input.parse()?;
        input.parse::<Token![,]>()?;
        let stages = Punctuated::<syn::Ident, Token![,]>::parse_terminated(input)?;
        Ok(CombinedShader {
            visibility,
            name,
            path,
            stages,
        })
    }
}

#[proc_macro]
pub fn include_combined_shader(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let s = syn::parse_macro_input!(input as CombinedShader);
    compile_combined_shader_file_to_spirv(&s).into()
}

fn compile_combined_shader_file_to_spirv(s: &CombinedShader) -> proc_macro2::TokenStream {
    let mut stages = Vec::new();
    for ident in s.stages.iter() {
        let stage = ident.to_string();
        match stage.as_str() {
            "vertex" => stages.push(shaderc::ShaderKind::Vertex),
            "fragment" => stages.push(shaderc::ShaderKind::Fragment),
            "tess_control" => stages.push(shaderc::ShaderKind::TessControl),
            "tess_eval" => stages.push(shaderc::ShaderKind::TessEvaluation),
            "geometry" => stages.push(shaderc::ShaderKind::Geometry),
            "compute" => stages.push(shaderc::ShaderKind::Compute),
            other => ident
                .span()
                .unstable()
                .error(format!("invalid stage: {}", other))
                .emit(),
        }
    }

    let path = s.path.value();
    let mut src = String::new();
    File::open(&path).unwrap().read_to_string(&mut src).unwrap();

    let pp = preprocessor::process_includes(&src, Some(Path::new(&path)), &[]);

    let pp = match pp {
        Ok(pp) => pp,
        Err(e) => {
            s.path
                .span()
                .unstable()
                .error("error(s) encountered while preprocessing combined GLSL source")
                .note(format!("{}", e))
                .emit();

            return quote! {()};
        }
    };

    let version = pp.version.unwrap_or_else(|| {
        s.path.span().unstable().warning(format!(
            "{:?}: no GLSL version specified, defaulting to 3.30",
            path
        ));
        330
    });

    let mut compiler = shaderc::Compiler::new().unwrap();
    let mut opt = shaderc::CompileOptions::new().unwrap();
    opt.set_target_env(shaderc::TargetEnv::Vulkan, 0);
    opt.set_optimization_level(shaderc::OptimizationLevel::Zero);

    let mut try_compile = |src: &String, stage: shaderc::ShaderKind| -> proc_macro2::TokenStream {
        if stages.contains(&stage) {
            let (stage_item, stage_macro) = match stage {
                shaderc::ShaderKind::Vertex => {
                    (syn::Ident::new("VERTEX", Span::call_site()), "_VERTEX_")
                }
                shaderc::ShaderKind::Geometry => {
                    (syn::Ident::new("GEOMETRY", Span::call_site()), "_GEOMETRY_")
                }
                shaderc::ShaderKind::Fragment => {
                    (syn::Ident::new("FRAGMENT", Span::call_site()), "_FRAGMENT_")
                }
                shaderc::ShaderKind::TessControl => (
                    syn::Ident::new("TESS_CONTROL", Span::call_site()),
                    "_TESS_CONTROL_",
                ),
                shaderc::ShaderKind::TessEvaluation => (
                    syn::Ident::new("TESS_EVAL", Span::call_site()),
                    "_TESS_EVAL_",
                ),
                shaderc::ShaderKind::Compute => {
                    (syn::Ident::new("COMPUTE", Span::call_site()), "_COMPUTE_")
                }
                _ => panic!("invalid shader stage"),
            };
            let stage_src = preprocessor::extract_stage(src, version, stage_macro, &[]);
            let inner = compile_glsl_shader_inner(
                &mut compiler,
                &opt,
                &stage_src,
                &s.path.span(),
                &path,
                None,
                stage,
            );
            quote! { pub const #stage_item: &'static [u8] = #inner; }
        } else {
            quote! {}
        }
    };

    let vert = try_compile(&pp.src, shaderc::ShaderKind::Vertex);
    let frag = try_compile(&pp.src, shaderc::ShaderKind::Fragment);
    let geom = try_compile(&pp.src, shaderc::ShaderKind::Geometry);
    let tessctl = try_compile(&pp.src, shaderc::ShaderKind::TessControl);
    let tesseval = try_compile(&pp.src, shaderc::ShaderKind::TessEvaluation);
    let comp = try_compile(&pp.src, shaderc::ShaderKind::Compute);

    let visibility = &s.visibility;
    let name = &s.name;

    quote! {
        #visibility struct #name;
        impl #name {
            #vert
            #frag
            #geom
            #tessctl
            #tesseval
            #comp
        }
    }
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

    let mut path = pathlit.span().unstable().source_file().path();
    path.set_file_name(&filename);

    // TODO maybe it's better to return a compile_error!{} ?
    let src = fs::read_to_string(&path).expect("failed to open GLSL shader source");

    // preprocess shader
    //let pp = preprocessor::process_includes(&src, Some(&path), &[]).expect("preprocessing failed");

    let bytecode = compile_glsl_shader(
        &src,
        &pathlit.span(),
        path.as_os_str().to_str().expect("path was not valid UTF-8"),
        None,
        stage,
    );

    // include_str so that it is considered when tracking dirty files
    let q = quote! { (include_str!(#pathlit), #bytecode).1 };
    q.into()
}
