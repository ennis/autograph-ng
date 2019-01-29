use crate::autograph_name;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Ident;

/// Checks that the derive input has a repr(C) attribute.
fn has_repr_c_attr(ast: &syn::DeriveInput) -> bool {
    ast.attrs.iter().any(|attr| match attr.parse_meta() {
        Ok(meta) => match meta {
            syn::Meta::List(list) => {
                (&list.ident.to_string() == "repr")
                    && list.nested.iter().next().map_or(false, |n| match n {
                    syn::NestedMeta::Meta(syn::Meta::Word(ref ident)) => ident.to_string() == "C",
                    _ => false,
                })
            }
            _ => false,
        },
        Err(_) => false,
    })
}

/// See [generate_struct_layout]
struct StructLayout
{
    offsets: Vec<syn::ItemConst>,
    sizes: Vec<syn::ItemConst>
}

/// Utility function to generate a set of constant items containing the offsets and sizes of each
/// field of a repr(C) struct.
fn generate_struct_layout(fields: &syn::Fields) -> StructLayout
{
    let fields = match *fields {
        syn::Fields::Named(ref fields_named) => &fields_named.named,
        syn::Fields::Unnamed(ref fields_unnamed) => &fields_unnamed.unnamed,
        syn::Fields::Unit => panic!("Cannot generate struct layout of unit structs"),
    };

    let mut offsets = Vec::new();
    let mut sizes = Vec::new();
    let mut offset_idents = Vec::new();
    let mut size_idents = Vec::new();

    for (i, f) in fields.iter().enumerate() {
        let field_ty = &f.ty;

        // field offset item
        if i == 0 {
            offsets.push(syn::parse_quote!{ pub const OFFSET_0: usize = 0; });
            sizes.push(syn::parse_quote!{ pub const SIZE_0: usize = ::std::mem::size_of::<#field_ty>(); });
            offset_idents.push(Ident::new("OFFSET_0", Span::call_site()));
            size_idents.push(Ident::new("SIZE_0", Span::call_site()));
        } else {
            let offset0 = &offset_idents[i-1];
            let offset1 = Ident::new(&format!("OFFSET_{}", i), Span::call_site());
            let size0 = &size_idents[i-1];
            let size1 = Ident::new(&format!("SIZE_{}", i), Span::call_site());

            offsets.push(syn::parse_quote! {
                pub const #offset1: usize =
                    (#offset0+#size0)
                    + (::std::mem::align_of::<#field_ty>() -
                            (#offset0+#size0)
                                % ::std::mem::align_of::<#field_ty>())
                      % ::std::mem::align_of::<#field_ty>();
            });
            sizes.push(syn::parse_quote! {
                 pub const #size1: usize = ::std::mem::size_of::<#field_ty>();
            });

            offset_idents.push(offset1);
            size_idents.push(size1);
        };
    }

    StructLayout {
        offsets,
        sizes
    }
}


pub fn generate_structured_buffer_data(ast: &syn::DeriveInput, fields: &syn::Fields) -> TokenStream {
    let gfx = autograph_name();

    if !has_repr_c_attr(ast) {
        panic!("derive(StructuredBufferData) can only be used on repr(C) structs");
    }

    let struct_name = &ast.ident;
    let privmod = syn::Ident::new(
        &format!("__StructuredBufferData_{}", struct_name),
        Span::call_site(),
    );

    let layout = generate_struct_layout(fields);

    let fields = match *fields {
        syn::Fields::Named(ref fields_named) => &fields_named.named,
        syn::Fields::Unnamed(ref fields_unnamed) => &fields_unnamed.unnamed,
        syn::Fields::Unit => panic!("Cannot generate struct layout of unit structs"),
    };

    let mut field_descs = Vec::new();

    for (i, f) in fields.iter().enumerate() {
        let field_ty = &f.ty;
        let offset = &layout.offsets[i];
        let offset = &offset.ident;

        field_descs.push(
            quote!{ (#privmod::#offset, <#field_ty as #gfx::buffer::StructuredBufferData>::TYPE) }
        );
    }

    let offsets = &layout.offsets;
    let sizes = &layout.sizes;

    quote! {
        #[allow(non_snake_case)]
        mod #privmod {
            use super::*;
            #(#offsets)*
            #(#sizes)*
        }

        unsafe impl #gfx::buffer::StructuredBufferData for #struct_name {
            const TYPE: &'static #gfx::typedesc::TypeDesc<'static> = &#gfx::typedesc::TypeDesc::Struct(
                #gfx::typedesc::StructLayout {
                    fields: &[#(#field_descs),*],
                });
        }
    }
}


pub fn generate_vertex_data(ast: &syn::DeriveInput, fields: &syn::Fields) -> TokenStream {
    let gfx = autograph_name();

    if !has_repr_c_attr(ast) {
        panic!("derive(VertexData) can only be used on repr(C) structs");
    }

    let struct_name = &ast.ident;
    let privmod = syn::Ident::new(
        &format!("__vertex_data_{}", struct_name),
        Span::call_site(),
    );

    let layout = generate_struct_layout(fields);

    let fields = match *fields {
        syn::Fields::Named(ref fields_named) => &fields_named.named,
        syn::Fields::Unnamed(ref fields_unnamed) => &fields_unnamed.unnamed,
        syn::Fields::Unit => panic!("Cannot generate struct layout of unit structs"),
    };

    let mut attribs = Vec::new();

    for (i, f) in fields.iter().enumerate() {
        let field_ty = &f.ty;
        let offset = &layout.offsets[i];
        let offset = &offset.ident;

        attribs.push(
            quote!{
                #gfx::vertex::TypedVertexInputAttributeDescription {
                    ty: &<#field_ty as #gfx::vertex::VertexAttributeType>::EQUIVALENT_TYPE,
                    //location: #i as u32,
                    format: <#field_ty as #gfx::vertex::VertexAttributeType>::FORMAT,
                    offset: #privmod::#offset as u32,
                }
            }
        );
    }

    let offsets = &layout.offsets;
    let sizes = &layout.sizes;

    quote! {
        #[allow(non_snake_case)]
        mod #privmod {
            use super::*;
            #(#offsets)*
            #(#sizes)*
        }

        unsafe impl #gfx::vertex::VertexData for #struct_name {
            const LAYOUT: #gfx::vertex::VertexLayout<'static> =
                #gfx::vertex::VertexLayout {
                    elements: &[#(#attribs,)*],
                    stride: ::std::mem::size_of::<#struct_name>()
                };
        }
    }
}