use super::autograph_name;
use darling::{util::Flag, FromDeriveInput, FromField};
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;

// Q: which attributes to expose?
// arrays should rarely be used => prefer assigning a meaningful name to each binding
// vertex_buffer, vertex_buffer_array

#[derive(FromDeriveInput, Debug)]
#[darling(attributes(argument), forward_attrs(allow, doc, cfg, repr))]
struct ArgumentsStruct {
    ident: syn::Ident,
    generics: syn::Generics,
    vis: syn::Visibility,
    attrs: Vec<syn::Attribute>,
    #[darling(default)]
    backend: Option<String>,
    //vertex_shader: String,
    //fragment_shader: String,
    //topology: String,
}

#[derive(FromField)]
#[darling(attributes(argument))]
struct ArgumentsItem {
    #[darling(default)]
    inherit: Flag,
    #[darling(default)]
    render_target: Flag,
    #[darling(default)]
    depth_stencil_render_target: Flag,
    #[darling(default)]
    viewport: Flag,
    #[darling(default)]
    viewport_array: Flag,
    #[darling(default)]
    scissor: Flag,
    #[darling(default)]
    scissor_array: Flag,
    #[darling(default)]
    vertex_buffer: Flag,
    #[darling(default)]
    vertex_buffer_array: Flag,
    #[darling(default)]
    index_buffer: Flag,
    // Shader interfaces -----------------------
    #[darling(default)]
    uniform_buffer: Flag,
    #[darling(default)]
    storage_buffer: Flag,
    #[darling(default)]
    sampled_image: Flag,
    #[darling(default)]
    storage_image: Flag,
}

fn quote_descriptor(
    gfx: &syn::Path,
    index: usize,
    ty: &syn::Type,
    ty_backend: &syn::Ident,
    descty: &str,
) -> TokenStream {
    let descty = Some(syn::Ident::new(descty, Span::call_site()));
    quote! {
        #gfx::descriptor::DescriptorBinding {
            binding: #index,
            descriptor_type: #gfx::descriptor::DescriptorType::#descty,
            stage_flags: #gfx::pipeline::ShaderStageFlags::ALL_GRAPHICS,
            count: 1,
            tydesc: <#ty as #gfx::descriptor::DescriptorInterface<#ty_backend> >::TYPE,
        }
    }
}

pub fn generate(ast: &syn::DeriveInput, fields: &syn::Fields) -> TokenStream {
    let s: ArgumentsStruct = <ArgumentsStruct as FromDeriveInput>::from_derive_input(ast).unwrap();

    let gfx = autograph_name();
    let struct_name = &s.ident;

    //----------------------------------------------------------------------------------------------
    let (impl_generics, ty_generics, where_clause) = s.generics.split_for_impl();
    let first_lt = s.generics.lifetimes().next();

    let lt_arena = if let Some(lt) = first_lt {
        lt
    } else {
        return syn::Error::new(
            s.generics.span(),
            "expected exactly one lifetime on target of `#[derive(Arguments)]`",
        )
        .to_compile_error();
    };

    let ty_backend = if let Some(ref backend) = s.backend {
        syn::Ident::new(backend, Span::call_site())
    } else {
        // no backend specified, assume that it's the first generic type argument
        let first_ty = s.generics.type_params().next();
        if let Some(ty) = first_ty {
            ty.ident.clone()
        } else {
            s.ident.span().unstable().error("could not deduce backend type")
                .help("specify the backend type with #[pipeline(backend=\"...\")] or make it a generic type parameter on the type")
                .emit();
            return quote!();
        }
    };

    //----------------------------------------------------------------------------------------------
    let fields = match fields {
        syn::Fields::Named(ref fields_named) => &fields_named.named,
        syn::Fields::Unnamed(ref fields_unnamed) => &fields_unnamed.unnamed,
        syn::Fields::Unit => panic!("Arguments trait cannot be derived on unit structs"),
    };

    let mut stmts = Vec::new();
    let mut iter_args = Vec::new();
    let mut iter_render_targets = Vec::new();
    let mut iter_descriptors = Vec::new();
    let mut iter_vertex_buffers = Vec::new();
    let mut iter_viewports = Vec::new();
    let mut iter_scissors = Vec::new();

    let mut i_inherited_ty = Vec::new();
    let mut i_inherited = Vec::new();
    let mut i_fragout = Vec::new();
    let mut i_vtxin = Vec::new();
    let mut i_desc = Vec::new();
    let mut ib_format = None;
    let mut seen_dst = false;
    let mut _n_viewports = 0usize;
    let mut _n_scissors = 0usize;

    for f in fields.iter() {
        let ty = &f.ty;
        let name = &f.ident.as_ref().unwrap();

        match <ArgumentsItem as FromField>::from_field(f) {
            Ok(pitem) => {
                // check for duplicates
                let mut num_attrs = 0;
                if pitem.inherit.is_some() {
                    num_attrs += 1;
                }
                if pitem.render_target.is_some() {
                    num_attrs += 1;
                }
                if pitem.viewport.is_some() {
                    num_attrs += 1;
                }
                if pitem.viewport_array.is_some() {
                    num_attrs += 1;
                }
                if pitem.scissor.is_some() {
                    num_attrs += 1;
                }
                if pitem.scissor_array.is_some() {
                    num_attrs += 1;
                }
                if pitem.vertex_buffer.is_some() {
                    num_attrs += 1;
                }
                if pitem.vertex_buffer_array.is_some() {
                    num_attrs += 1;
                }
                if pitem.index_buffer.is_some() {
                    num_attrs += 1;
                }
                if pitem.uniform_buffer.is_some() {
                    num_attrs += 1;
                }
                if pitem.storage_buffer.is_some() {
                    num_attrs += 1;
                }
                if pitem.sampled_image.is_some() {
                    num_attrs += 1;
                }
                if pitem.storage_image.is_some() {
                    num_attrs += 1;
                }
                if pitem.depth_stencil_render_target.is_some() {
                    num_attrs += 1;
                }

                if num_attrs == 0 {
                    stmts.push(syn::Error::new(name.span(), "missing or incomplete `argument(...)` attribute. See the documentation of `Arguments` for more information.")
                        .to_compile_error());
                    continue;
                } else if num_attrs > 1 {
                    stmts.push(
                        syn::Error::new(
                            name.span(),
                            "field has more than one `argument(...)` attribute.",
                        )
                        .to_compile_error(),
                    );
                    continue;
                }

                // arguments --------------------------------------------
                if pitem.inherit.is_some() {
                    //let index = i_subsig.len();
                    iter_args.push(quote! {
                        std::iter::once(self.#name.into())
                    });
                    i_inherited.push(
                        quote! { <#ty as #gfx::pipeline::Arguments<#ty_backend>>::SIGNATURE },
                    );
                    i_inherited_ty.push(quote!(#ty));
                }
                // render target --------------------------------------------
                else if pitem.render_target.is_some() {
                    iter_render_targets.push(quote! {
                        std::iter::once(self.#name.into())
                    });
                    i_fragout.push(quote! { #gfx::framebuffer::FragmentOutputDescription{} });
                }
                // depth stencil render target --------------------------------------------
                else if pitem.depth_stencil_render_target.is_some() {
                    if !seen_dst {
                        stmts
                            .push(quote! { depth_stencil_render_target = Some(self.#name.into()) });
                        seen_dst = true;
                    } else {
                        stmts.push(
                            syn::Error::new(
                                name.span(),
                                "duplicate `argument(depth_stencil_render_target)` attribute",
                            )
                            .to_compile_error(),
                        );
                    }
                }
                // descriptor: ubo --------------------------------------------
                else if pitem.uniform_buffer.is_some() {
                    iter_descriptors.push(quote! {
                       std::iter::once(self.#name.into())
                    });
                    let index = i_desc.len();
                    i_desc.push(quote_descriptor(
                        &gfx,
                        index,
                        ty,
                        &ty_backend,
                        "UniformBuffer",
                    ));
                }
                // descriptor: ssbo --------------------------------------------
                else if pitem.storage_buffer.is_some() {
                    iter_descriptors.push(quote! {
                       std::iter::once(self.#name.into())
                    });
                    let index = i_desc.len();
                    i_desc.push(quote_descriptor(
                        &gfx,
                        index,
                        ty,
                        &ty_backend,
                        "StorageBuffer",
                    ));
                }
                // descriptor: tex --------------------------------------------
                else if pitem.sampled_image.is_some() {
                    iter_descriptors.push(quote! {
                       std::iter::once(self.#name.into())
                    });
                    let index = i_desc.len();
                    i_desc.push(quote_descriptor(
                        &gfx,
                        index,
                        ty,
                        &ty_backend,
                        "SampledImage",
                    ));
                }
                // descriptor: img --------------------------------------------
                else if pitem.storage_image.is_some() {
                    iter_descriptors.push(quote! {
                        std::iter::once(self.#name.into())
                    });
                    let index = i_desc.len();
                    i_desc.push(quote_descriptor(
                        &gfx,
                        index,
                        ty,
                        &ty_backend,
                        "StorageImage",
                    ));
                }
                // vertex buffer --------------------------------------------
                else if pitem.vertex_buffer.is_some() {
                    iter_vertex_buffers.push(quote! {
                        std::iter::once(self.#name.into())
                    });

                    i_vtxin.push(quote! {
                        <<#ty as #gfx::vertex::VertexBufferInterface<#ty_backend>>::Vertex as #gfx::vertex::VertexData>::LAYOUT
                    });
                }
                // index buffer --------------------------------------------
                else if pitem.index_buffer.is_some() {
                    if ib_format.is_none() {
                        // need indextype + offset
                        stmts.push(quote! {
                            index_buffer = Some(self.#name.into());
                        });
                        ib_format = Some(
                            quote!(Some(<<#ty as #gfx::vertex::IndexBufferInterface<#ty_backend>>::Index as #gfx::vertex::IndexData>::FORMAT)),
                        );
                    } else {
                        stmts.push(
                            syn::Error::new(
                                name.span(),
                                "duplicate `argument(index_buffer)` attribute",
                            )
                            .to_compile_error(),
                        );
                    }
                }
                // viewport --------------------------------------------
                else if pitem.viewport.is_some() {
                    iter_viewports.push(quote! {
                        std::iter::once(self.#name.clone().into())
                    });

                    _n_viewports += 1;
                }
                // scissor --------------------------------------------
                else if pitem.scissor.is_some() {
                    iter_scissors.push(quote! {
                        std::iter::once(self.#name.clone().into())
                    });

                    _n_scissors += 1;
                } else if pitem.viewport_array.is_some() {
                    unimplemented!()
                } else if pitem.scissor_array.is_some() {
                    unimplemented!()
                } else if pitem.vertex_buffer_array.is_some() {
                    unimplemented!()
                }
            }
            Err(e) => {
                stmts.push(
                    syn::Error::new(
                        name.span(),
                        format!("failed to parse `argument(...)` attribute: {}", e),
                    )
                    .to_compile_error(),
                );
            }
        }
    }

    let is_root_fragment_output_signature = i_fragout.len() > 0;
    let is_root_vertex_input_signature = false;
    let depth_stencil_fragment_output = if seen_dst {
        quote!(Some(#gfx::framebuffer::FragmentOutputDescription{}))
    } else {
        quote!(None)
    };
    let ib_format = match ib_format {
        Some(fmt) => fmt,
        None => quote!(None),
    };

    let privmod = syn::Ident::new(
        &format!("__Arguments_UniqueType_{}", struct_name),
        Span::call_site(),
    );

    let ty_params = s.generics.type_params();
    let ty_params2 = s.generics.type_params();

    let q = quote! {

        #[doc(hidden)]
        mod #privmod {
            // IMPORTANT must be generic if interface struct is generic
            // but the generic parameters must also be 'static...
            pub struct Dummy<#(#ty_params,)*>;
        }

        impl #impl_generics #gfx::pipeline::Arguments<#lt_arena, #ty_backend> for #struct_name #ty_generics #where_clause {

            type UniqueType = #privmod::Dummy<#(#ty_params2,)*>;
            type IntoInterface = Self;

            const SIGNATURE: &'static #gfx::pipeline::SignatureDescription<'static> = &#gfx::pipeline::SignatureDescription {
                is_root_fragment_output_signature : #is_root_fragment_output_signature,
                is_root_vertex_input_signature    : #is_root_vertex_input_signature,
                inherited                         : &[#(#i_inherited,)*],
                descriptors                       : &[#(#i_desc,)*],
                vertex_layouts                    : &[#(#i_vtxin,)*],
                fragment_outputs                  : &[#(#i_fragout,)*],
                depth_stencil_fragment_output     : #depth_stencil_fragment_output,
                index_format                      : #ib_format,
                //typeid                            : Some(std::any::TypeId::of::<Self::UniqueType>()),
            };

            fn get_inherited_signatures(renderer: &#lt_arena #gfx::Renderer<#ty_backend>) -> Vec<&#lt_arena <#ty_backend as #gfx::Backend>::Signature> {
                use autograph_render::pipeline::Signature;
                let mut sig = Vec::new();
                #(sig.push(renderer.get_cached_signature::<#i_inherited_ty>().inner());)*
                sig
            }

            fn into_block(
                    self,
                    signature: #gfx::pipeline::TypedSignature<#lt_arena, #ty_backend, Self::IntoInterface>,
                    arena: &#lt_arena #gfx::Arena <#ty_backend>) ->
                    #gfx::pipeline::ArgumentBlock<#lt_arena,  #ty_backend, #gfx::pipeline::TypedSignature<#lt_arena, #ty_backend, Self::IntoInterface>>
            {
                //use #gfx::pipeline::ArgumentBlock;
                use #gfx::pipeline::Arguments;

                let mut index_buffer = None;
                let mut depth_stencil_render_target = None;

                #(#stmts)*

                let arguments = std::iter::empty()#(.chain(#iter_args))*;
                let descriptors = std::iter::empty()#(.chain(#iter_descriptors))*;
                let vertex_buffers = std::iter::empty()#(.chain(#iter_vertex_buffers))*;
                let render_targets = std::iter::empty()#(.chain(#iter_render_targets))*;
                let viewports = std::iter::empty()#(.chain(#iter_viewports))*;
                let scissors = std::iter::empty()#(.chain(#iter_scissors))*;

                arena.create_argument_block(
                    signature,
                    arguments,
                    descriptors,
                    vertex_buffers,
                    index_buffer,
                    render_targets,
                    depth_stencil_render_target,
                    viewports,
                    scissors)
            }
        }
    };

    q
}
