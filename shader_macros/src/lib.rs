#![feature(proc_macro_diagnostic)]
extern crate proc_macro;
extern crate proc_macro2;

use shaderc;
use proc_macro2::TokenStream;
use quote::quote;
use syn;
use std::sync::Mutex;
use std::path::Path;
use std::fs::File;

mod preprocessor;
use self::preprocessor::{SeparateShaderSources, preprocess_combined_shader_source};
use std::io::Read;

/*
fn gfx2_name() -> syn::Path {
    syn::parse_str("gfx2").unwrap()
}*/

#[proc_macro]
pub fn static_shader(src: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // parse a string literal
    let litstr = syn::parse_macro_input!(src as syn::LitStr);
    compile_glsl_shader(&litstr.value(), &litstr.span(), "<embedded GLSL>", shaderc::ShaderKind::Vertex).into()
}


#[proc_macro]
pub fn include_combined_shader(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let litpath = syn::parse_macro_input!(input as syn::LitStr);
    compile_combined_shader_file_to_spirv(litpath.value(), &litpath.span()).into()
}

fn compile_glsl_shader(src: &str, span: &proc_macro2::Span, file: &str, stage: shaderc::ShaderKind) -> TokenStream
{
    // the doc says that we should preferably create one instance of the compiler
    // and reuse it, but I don't see a way to reuse a compiler instance
    // between macro invocations. Notably, it cannot be put into a lazy_static block wrapped
    // in a mutex since it complains that `*mut shaderc::ffi::ShadercCompiler` is not `Send`.
    let mut compiler = shaderc::Compiler::new().unwrap();
    let mut opt = shaderc::CompileOptions::new().unwrap();
    opt.set_target_env(shaderc::TargetEnv::Vulkan, 0);
    opt.set_optimization_level(shaderc::OptimizationLevel::Zero);
    compile_glsl_shader_inner(&mut compiler, &opt, src, span, file, stage)
}

fn compile_glsl_shader_inner(
    compiler: &mut shaderc::Compiler,
    opts: &shaderc::CompileOptions,
    src: &str,
    span: &proc_macro2::Span,
    file: &str,
    stage: shaderc::ShaderKind) -> TokenStream
{
    let ca = compiler.compile_into_spirv(
        &src,
        stage,
        file,
        "main",
        Some(&opts),
    );

    match ca {
        Err(e) => {
            span.unstable()
                .error("error(s) encountered while compiling GLSL shader")
                .note(format!("{}", e))
                .emit();
            quote!(::gfx2_shader::StaticShader { spirv: &[] })
        },
        Ok(ca) => {
            // any warnings?
            let nw = ca.get_num_warnings();
            if nw != 0 {
                span.unstable().warning("warnings emitted during compilation")
                    .note(format!("compiler messages:\n{}", ca.get_warning_messages()));
            }

            let words = ca.as_binary();

            quote!(
                ::gfx2_shader::StaticShader {
                    spirv: &[ #(#words,)* ]
                }
            )
        }
    }
}

fn compile_combined_shader_file_to_spirv<P: AsRef<Path>>(
    path: P,
    span: &proc_macro2::Span
) -> TokenStream
{
    let mut src = String::new();
    File::open(path.as_ref()).unwrap().read_to_string(&mut src).unwrap();

    let pp = preprocess_combined_shader_source(&src, Some(path.as_ref()), &[]);

    let pp = match pp {
        Ok(pp) => pp,
        Err(e) => {
            span.unstable()
                .error("error(s) encountered while preprocessing combined GLSL source")
                .note(format!("{}", e))
                .emit();

            return quote!{()}
        }
    };

    let version = pp.version.unwrap_or_else(|| {
        span.unstable().warning(
            format!(
                "{:?}: no GLSL version specified, defaulting to 3.30",
                path.as_ref()));
        330
    });

    let sep = SeparateShaderSources::from_combined_source(&pp.srcpp, version, pp.stages, &[]);

    let mut compiler = shaderc::Compiler::new().unwrap();
    let mut opt = shaderc::CompileOptions::new().unwrap();
    opt.set_target_env(shaderc::TargetEnv::Vulkan, 0);
    opt.set_optimization_level(shaderc::OptimizationLevel::Zero);

    let path_str = path.as_ref().to_str().unwrap();

    let vert = sep.vert.as_ref();
    let frag = sep.frag.as_ref();
    let geom = sep.geom.as_ref();
    let tessctl = sep.tessctl.as_ref();
    let tesseval = sep.tesseval.as_ref();
    let comp = sep.comp.as_ref();

    let mut try_compile = |src: &Option<String>, stage: shaderc::ShaderKind| -> TokenStream {
        if let Some(src) = src {
            let inner = compile_glsl_shader_inner(&mut compiler, &opt, &src, span, path_str, stage);
            quote!{Some(#inner)}
        } else {
            quote!{None}
        }
    };

    let vert = try_compile(&sep.vert, shaderc::ShaderKind::Vertex);
    let frag = try_compile(&sep.frag, shaderc::ShaderKind::Fragment);
    let geom = try_compile(&sep.geom, shaderc::ShaderKind::Geometry);
    let tessctl = try_compile(&sep.tessctl, shaderc::ShaderKind::TessControl);
    let tesseval = try_compile(&sep.tesseval, shaderc::ShaderKind::TessEvaluation);
    let comp = try_compile(&sep.comp, shaderc::ShaderKind::Compute);

    quote!{
        ::gfx2_shader::CombinedShaders {
            vertex: #vert,
            fragment: #frag,
            geometry: #geom,
            tess_control: #tessctl,
            tess_eval: #tesseval,
            compute: #comp,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn compiles() {
        //assert_eq!(2 + 2, 4);
    }
}
