//! Proc-macro for auto-deriving pipeline interfaces:
//! - `PipelineInterface`
//! - `BufferLayout` for verifying the layout of uniform buffer data with SPIR-V
//! - `AttachmentGroup` for groups of attachments
//! - `VertexLayout` for verifying the layout of vertex buffers
//!
#![recursion_limit = "128"]

extern crate proc_macro;
extern crate proc_macro2;
#[macro_use]
extern crate darling; // this is a _good crate_
#[macro_use]
extern crate syn;
#[macro_use]
extern crate quote;

mod buffer_layout;
mod descriptor_set_interface;

fn gfx2_name() -> syn::Path {
    syn::parse_str("gfx2::renderer").unwrap()
}

#[proc_macro_derive(BufferLayout)]
pub fn buffer_layout_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).expect("Couldn't parse item");

    let result = match ast.data {
        syn::Data::Struct(ref s) => buffer_layout::generate(&ast, &s.fields),
        _ => panic!("BufferLayout trait can only be automatically derived on structs."),
    };

    result.into()
}

#[proc_macro_derive(DescriptorSetInterface, attributes(interface, descriptor))]
pub fn descriptor_set_interface_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).expect("Couldn't parse item");

    let result = match ast.data {
        syn::Data::Struct(ref s) => descriptor_set_interface::generate(&ast, &s.fields),
        _ => panic!("DescriptorSetInterface trait can only be derived on structs"),
    };

    result.into()
}

/*
#[proc_macro_derive(VertexType, attributes(rename))]
pub fn vertex_type_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).expect("Couldn't parse item");

    let result = match ast.data {
        syn::Data::Struct(ref s) => process_vertex_struct(&ast, &s.fields),
        _ => panic!("VertexType trait can only be derived on structs"),
    };

    result.into()
}

fn process_vertex_struct(ast: &syn::DeriveInput, fields: &syn::Fields) -> TokenStream {
    let struct_name = &ast.ident;

    let fields = match *fields {
        syn::Fields::Named(ref fields_named) => &fields_named.named,
        syn::Fields::Unnamed(ref fields_unnamed) => &fields_unnamed.unnamed,
        syn::Fields::Unit => panic!("VertexType trait cannot be derived on unit structs"),
    };

    let mut attrib_descs = Vec::new();
    let mut attrib_sizes = Vec::new();

    for (i, f) in fields.iter().enumerate() {
        println!("{} => {:?}", i, f.ident);
        let field_ty = &f.ty;
        let field_name = f
            .ident
            .clone()
            .unwrap_or(syn::Ident::new(&format!("unnamed_{}", i), Span::call_site()));
        let field_offset = if let Some(ref name) = f.ident {
            quote!(offset_of!(#struct_name,#name))
        } else {
            quote!(offset_of!(#struct_name,#i))
        };
        attrib_descs.push(quote!(::autograph::gfx::VertexAttributeDesc {
            name: Some(stringify!(#field_name).into()),
            loc: #i as u8,
            ty: <#field_ty as ::autograph::gfx::VertexAttributeType>::EQUIVALENT_TYPE,
            format: <#field_ty as ::autograph::gfx::VertexAttributeType>::FORMAT,
            offset: #field_offset as u8
        }));
        attrib_sizes.push(quote!(::std::mem::size_of::<#field_ty>()));
    }

    let num_attribs = attrib_descs.len();

    let private_module_name = syn::Ident::new(
        &format!("__vertex_type_{}", struct_name),
        proc_macro2::Span::call_site(),
    );

    quote! {
        #[allow(non_snake_case)]
        mod #private_module_name {
            use super::#struct_name;

            lazy_static!{
                pub(super) static ref ATTRIBUTES: [::autograph::gfx::VertexAttributeDesc;#num_attribs] = {[#(#attrib_descs),*]};
                pub(super) static ref STRIDE: usize = #(#attrib_sizes)+*;
                pub(super) static ref LAYOUT: ::autograph::gfx::VertexLayout =
                    ::autograph::gfx::VertexLayout {
                        attributes: &*ATTRIBUTES,
                        stride: *STRIDE
                    };
            }
        }

        impl ::autograph::gfx::VertexType for #struct_name {
            fn get_layout() -> &'static ::autograph::gfx::VertexLayout {
                &*#private_module_name::LAYOUT
            }
        }
    }
}

#[proc_macro_derive(
ShaderInterface,
attributes(uniform_constant, texture_binding, vertex_buffer, index_buffer, uniform_buffer)
)]
pub fn shader_interface_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).expect("Couldn't parse item");

    let result = match ast.data {
        syn::Data::Struct(ref s) => process_struct(&ast, &s.fields),
        _ => panic!("ShaderInterface trait can only be derived on structs"),
    };

    result.into()
}

#[derive(FromField)]
#[darling(attributes(uniform_constant))]
struct UniformConstant {
    ident: Option<syn::Ident>,
    ty: syn::Type,
    vis: syn::Visibility,
    #[darling(default)]
    rename: Option<String>,
    #[darling(default)]
    index: Option<u32>,
}

#[derive(FromField)]
#[darling(attributes(texture_binding))]
struct TextureBinding {
    ident: Option<syn::Ident>,
    ty: syn::Type,
    vis: syn::Visibility,
    #[darling(default)]
    rename: Option<String>,
    #[darling(default)]
    index: Option<u32>,
}

#[derive(FromField)]
#[darling(attributes(vertex_buffer))]
struct VertexBuffer {
    ident: Option<syn::Ident>,
    ty: syn::Type,
    vis: syn::Visibility,
    #[darling(default)]
    rename: Option<String>,
    #[darling(default)]
    index: Option<u32>,
}

#[derive(FromField)]
#[darling(attributes(uniform_buffer))]
struct UniformBuffer {
    ident: Option<syn::Ident>,
    ty: syn::Type,
    vis: syn::Visibility,
    #[darling(default)]
    rename: Option<String>,
    #[darling(default)]
    index: Option<u32>,
}

#[derive(FromField)]
#[darling(attributes(index_buffer))]
struct IndexBuffer {
    ident: Option<syn::Ident>,
    ty: syn::Type,
    vis: syn::Visibility,
    #[darling(default)]
    rename: Option<String>,
}

#[derive(FromField)]
#[darling(attributes(named_uniform))]
struct RenderTarget {
    ident: Option<syn::Ident>,
    ty: syn::Type,
    vis: syn::Visibility,
    #[darling(default)]
    rename: Option<String>,
    #[darling(default)]
    index: Option<u32>,
}

fn error_multiple_interface_attrs() {
    panic!("Multiple interface attributes on field.");
}

fn make_option_tokens<T: quote::ToTokens>(v: &Option<T>) -> TokenStream {
    if let Some(v) = v.as_ref() {
        quote!(Some(#v))
    } else {
        quote!(None)
    }
}

fn process_struct(ast: &syn::DeriveInput, fields: &syn::Fields) -> TokenStream {
    let struct_name = &ast.ident;

    let mut uniform_constants = Vec::new();
    let mut texture_bindings = Vec::new();
    let mut vertex_buffers = Vec::new();
    let mut render_targets = Vec::new();
    let mut uniform_buffers = Vec::new();
    let mut index_buffer = None;

    match *fields {
        syn::Fields::Named(ref fields) => {
            for f in fields.named.iter() {
                let field_name = f.ident.clone().unwrap();
                let mut seen_interface_attr = false;
                for a in f.attrs.iter() {
                    let meta = a.interpret_meta();
                    let meta = if let Some(meta) = meta {
                        meta
                    } else {
                        continue;
                    };

                    match meta.name().to_string().as_ref() {
                        "uniform_constant" => {
                            if seen_interface_attr {
                                error_multiple_interface_attrs();
                            }
                            let uniform_constant =
                                <UniformConstant as FromField>::from_field(f).unwrap();
                            uniform_constants.push(uniform_constant);
                            seen_interface_attr = true;
                        }
                        "texture_binding" => {
                            if seen_interface_attr {
                                error_multiple_interface_attrs();
                            }
                            let texture_binding =
                                <TextureBinding as FromField>::from_field(f).unwrap();
                            texture_bindings.push(texture_binding);
                            seen_interface_attr = true;
                        }
                        "vertex_buffer" => {
                            if seen_interface_attr {
                                error_multiple_interface_attrs();
                            }
                            let vb = <VertexBuffer as FromField>::from_field(f).unwrap();
                            vertex_buffers.push(vb);
                            seen_interface_attr = true;
                        }
                        "uniform_buffer" => {
                            if seen_interface_attr {
                                error_multiple_interface_attrs();
                            }
                            let ub = <UniformBuffer as FromField>::from_field(f).unwrap();
                            uniform_buffers.push(ub);
                            seen_interface_attr = true;
                        }
                        "index_buffer" => {
                            if seen_interface_attr {
                                error_multiple_interface_attrs();
                            }
                            if index_buffer.is_some() {
                                panic!("Only one index buffer can be specified.")
                            }
                            let ib = <IndexBuffer as FromField>::from_field(f).unwrap();
                            index_buffer = Some(ib);
                            seen_interface_attr = true;
                        }
                        "render_target" => {
                            if seen_interface_attr {
                                error_multiple_interface_attrs();
                            }
                            let rt = <RenderTarget as FromField>::from_field(f).unwrap();
                            render_targets.push(rt);
                            seen_interface_attr = true;
                        }
                        _ => {}
                    }
                }
            }
        }
        _ => panic!("ShaderInterface trait cannot be derived on unit structs or tuple structs."),
    }

    /*//
    // named uniforms
    //
    let uniform_constant_items = uniform_constants
        .iter()
        .map(|u| {
            let name = u
                .rename
                .as_ref()
                .map_or(u.ident.clone().unwrap(), |s| syn::Ident::new(s.as_str(), Span::call_site()));
            let index_tokens = make_option_tokens(&u.index);
            let ty = &u.ty;

            //let index_tokens = make_option_tokens(texbind.index);
            quote! {
                UniformConstantDesc {
                    name: Some(stringify!(#name).into()),
                    index: #index_tokens,
                    ty: <#ty as UniformInterface>::get_description()
                }
            }
        })
        .collect::<Vec<_>>();
    let num_uniform_constant_items = uniform_constant_items.len();

    //
    // texture+sampler bindings
    //
    let mut texture_binding_items = Vec::new();
    let mut texture_bind_statements = Vec::new();
    for texbind in texture_bindings.iter() {
        let orig_name = texbind.ident.clone().unwrap();
        let name = texbind
            .rename
            .as_ref()
            .map_or(texbind.ident.clone().unwrap(), |s| syn::Ident::new(s.as_str(), Span::call_site()));
        let index_tokens = make_option_tokens(&texbind.index);
        let ty = &texbind.ty;

        texture_binding_items.push(quote! {
            ::autograph::gfx::shader_interface::TextureBindingDesc {
                name: Some(stringify!(#name).into()),
                index: #index_tokens,
                data_type: <<#ty as SampledTextureInterface>::TextureType as TextureInterface>::get_data_type(),
                dimensions: <<#ty as SampledTextureInterface>::TextureType as TextureInterface>::get_dimensions()
            }
        });

        texture_bind_statements.push(quote! {
             {
                let tex = interface.#orig_name.get_texture().into_texture_any();
                let sampler = interface.#orig_name.get_sampler();
                let sampler_obj = frame.queue().context().get_sampler(sampler);
                state_cache.set_texture((#index_tokens).unwrap(), &tex, &sampler_obj);
                frame.ref_texture(tex);
            }
        });
    }
    let num_texture_binding_items = texture_binding_items.len();

    //
    // vertex buffers
    //
    let vertex_buffer_items = vertex_buffers
        .iter()
        .map(|vb| {
            let name = vb
                .rename
                .as_ref()
                .map_or(vb.ident.clone().unwrap(), |s| syn::Ident::new(s.as_str(), Span::call_site()));
            let index_tokens = make_option_tokens(&vb.index);
            let ty = &vb.ty;

            quote! {
                ::autograph::gfx::shader_interface::VertexBufferDesc {
                    name: Some(stringify!(#name).into()),
                    index: #index_tokens,
                    layout: <<#ty as ::autograph::gfx::VertexDataSource>::ElementType as ::autograph::gfx::VertexType>::get_layout()
                }
            }
        })
        .collect::<Vec<_>>();
    let num_vertex_buffer_items = vertex_buffer_items.len();

    //
    // uniform buffers
    //
    let mut uniform_buffer_items = Vec::new();
    let mut uniform_buffer_bind_statements = Vec::new();
    for ub in uniform_buffers.iter() {
        let orig_name = ub.ident.clone().unwrap();
        let name = ub
            .rename
            .as_ref()
            .map_or(ub.ident.clone().unwrap(), |s| syn::Ident::new(s.as_str(), Span::call_site()));
        let index_tokens = make_option_tokens(&ub.index);
        let ty = &ub.ty;

        uniform_buffer_items.push(quote! {
            ::autograph::gfx::shader_interface::UniformBufferDesc {
                name: Some(stringify!(#name).into()),
                index: #index_tokens,
                tydesc: <#ty as ::autograph::gfx::BufferInterface>::get_layout()
            }
        });
        uniform_buffer_bind_statements.push(quote! {
            {
                let slice_any = interface.#orig_name.to_slice_any();
                state_cache.set_uniform_buffer((#index_tokens).unwrap(), &slice_any);
                frame.ref_buffer(slice_any.owner);
            }
        });
    }

    let num_uniform_buffer_items = uniform_buffer_items.len();

    //
    // render targets
    //
    let render_target_items = render_targets
        .iter()
        .map(|rt| {
            let name = rt
                .rename
                .as_ref()
                .map_or(rt.ident.clone().unwrap(), |s| syn::Ident::new(s.as_str(), Span::call_site()));
            let index_tokens = make_option_tokens(&rt.index);
            let ty = &rt.ty;
            quote! {
                ::autograph::gfx::shader_interface::RenderTargetDesc {
                    name: Some(stringify!(#name).into()),
                    index: #index_tokens,
                    format: None
                }
            }
        })
        .collect::<Vec<_>>();
    let num_render_target_items = render_target_items.len();

    let index_buffer_item = if let Some(ib) = index_buffer {
        let ty = &ib.ty;
        quote! {
            Some(IndexBufferDesc {
                format: <<#ty as ::autograph::gfx::IndexDataSource>::ElementType as ::autograph::gfx::IndexElementType>::FORMAT
            })
        }
    } else {
        quote!(None)
    };

    let private_module_name = syn::Ident::new(
        &format!("__shader_interface_{}", struct_name),
        proc_macro2::Span::call_site(),
    );

    // generate impls
    quote!{
        #[allow(non_snake_case)]
        mod #private_module_name {
            use super::#struct_name;
            use ::autograph::gfx::shader_interface::*;

            pub(super) struct Desc;
            pub(super) struct Binder;

            lazy_static!{
                static ref UNIFORM_CONSTANTS: [UniformConstantDesc;#num_uniform_constant_items] = [#(#uniform_constant_items),*];
                static ref TEXTURE_BINDINGS: [TextureBindingDesc;#num_texture_binding_items] = [#(#texture_binding_items),*];
                static ref VERTEX_BUFFERS: [VertexBufferDesc;#num_vertex_buffer_items] = [#(#vertex_buffer_items),*];
                static ref UNIFORM_BUFFERS: [UniformBufferDesc;#num_uniform_buffer_items] = [#(#uniform_buffer_items),*];
                static ref RENDER_TARGETS: [RenderTargetDesc;#num_render_target_items] = [#(#render_target_items),*];
                static ref INDEX_BUFFER: Option<IndexBufferDesc> = #index_buffer_item;
            }

            impl ShaderInterfaceDesc for Desc {
                fn get_uniform_constants(&self) -> &'static [UniformConstantDesc] {
                    &*UNIFORM_CONSTANTS
                }
                fn get_render_targets(&self) -> &'static [RenderTargetDesc] {
                     &*RENDER_TARGETS
                }
                fn get_vertex_buffers(&self) -> &'static [VertexBufferDesc] {
                    &*VERTEX_BUFFERS
                }
                fn get_index_buffer(&self) -> Option<&'static IndexBufferDesc> {
                    INDEX_BUFFER.as_ref()
                }
                fn get_texture_bindings(&self) -> &'static [TextureBindingDesc] {
                    &*TEXTURE_BINDINGS
                }
                fn get_uniform_buffers(&self) -> &'static [UniformBufferDesc] {
                    &*UNIFORM_BUFFERS
                }
                //fn get_framebuffer(&self) ->
            }

            impl InterfaceBinder<#struct_name> for Binder {
                unsafe fn bind_unchecked(&self, interface: &#struct_name, frame: &::autograph::gfx::Frame, state_cache: &mut ::autograph::gfx::StateCache) {
                    use ::autograph::gfx::ToBufferSliceAny;
                    use ::autograph::gfx::SampledTextureInterface;
                    unsafe {
                        #(#uniform_buffer_bind_statements)*
                    }
                }
            }

        }

        impl ::autograph::gfx::ShaderInterface for #struct_name {
            fn get_description() -> &'static ::autograph::gfx::ShaderInterfaceDesc {
                static INSTANCE: &'static ::autograph::gfx::ShaderInterfaceDesc = &#private_module_name::Desc;
                INSTANCE
            }

            fn create_interface_binder(pipeline: &::autograph::gfx::GraphicsPipeline) -> Result<Box<::autograph::gfx::InterfaceBinder<Self>>, ::failure::Error> where Self: Sized {
                // TODO: verify interface
                Ok(Box::new(#private_module_name::Binder))
            }
        }
    }*/
}
*/

#[cfg(test)]
mod test {
    #[test]
    fn compiles() {}
}
