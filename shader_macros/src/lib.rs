#![feature(proc_macro_diagnostic)]
extern crate proc_macro;
extern crate proc_macro2;

use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use shaderc;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use syn;
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::Token;
use syn::TypeBareFn;

mod preprocessor;

/*
fn gfx2_name() -> syn::Path {
    syn::parse_str("gfx2").unwrap()
}*/

#[proc_macro_attribute]
pub fn shader_module(
    _attribs: proc_macro::TokenStream,
    src: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    //src

    // parse a whole module
    let m: syn::ItemMod = syn::parse_macro_input!(src as syn::ItemMod);
    let mut stub_fields = Vec::new();

    if let Some((_, ref contents)) = m.content {
        for item in contents.iter() {
            match item {
                syn::Item::Fn(itemfn) => {
                    println!("fn {:?}", itemfn);
                    let lifetimes = itemfn.decl.generics.lifetimes();
                    let unsafety = &itemfn.unsafety;
                    let abi = &itemfn.abi;
                    let _attrs = &itemfn.attrs;
                    let _asyncness = itemfn.asyncness;
                    let inputs = &itemfn.decl.inputs;
                    let output = &itemfn.decl.output;
                    let ident = &itemfn.ident;
                    let tytk = quote! {
                        #ident: &'lib for<#(#lifetimes),*> #unsafety #abi fn(#inputs) #output
                    };
                    println!("{}", tytk.to_string());
                    stub_fields.push(tytk);
                }
                syn::Item::Const(itemconst) => {
                    let ty = &itemconst.ty;
                    let ident = &itemconst.ident;
                    let tytk = quote! {
                        #ident: &'lib #ty,
                    };
                    stub_fields.push(tytk);
                }
                _ => {}
            }
        }
    }

    let mod_name = m.ident;
    let stub_name = syn::Ident::new(
        &format!("{}_Symbols", mod_name.to_string()),
        Span::call_site(),
    );
    let items = m.content.as_ref().unwrap().1.iter();

    let q = quote! {
        mod #mod_name {
            #(#items)*

            #[allow(non_snake_case)]
            struct #stub_name <'lib> {
                #(#stub_fields)*
            }
        }
    };
    q.into()
}

fn compile_shader(
    src: proc_macro::TokenStream,
    stage: shaderc::ShaderKind,
) -> proc_macro::TokenStream {
    // parse a string literal
    let litstr = syn::parse_macro_input!(src as syn::LitStr);
    compile_glsl_shader(&litstr.value(), &litstr.span(), "<embedded GLSL>", stage).into()
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

fn compile_glsl_shader(
    src: &str,
    span: &proc_macro2::Span,
    file: &str,
    stage: shaderc::ShaderKind,
) -> TokenStream {
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
    stage: shaderc::ShaderKind,
) -> TokenStream {
    let ca = compiler.compile_into_spirv(&src, stage, file, "main", Some(&opts));

    match ca {
        Err(e) => {
            span.unstable()
                .error("error(s) encountered while compiling GLSL shader")
                .note(format!("{}", e))
                .emit();
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

fn compile_combined_shader_file_to_spirv(s: &CombinedShader) -> TokenStream {
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

    let mut try_compile = |src: &String, stage: shaderc::ShaderKind| -> TokenStream {
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
