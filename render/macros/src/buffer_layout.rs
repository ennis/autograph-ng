use super::autograph_name;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Ident, Meta, NestedMeta};

pub fn generate(ast: &syn::DeriveInput, fields: &syn::Fields) -> TokenStream {
    let gfx = autograph_name();

    // detect repr(C)
    let struct_is_repr_c = ast.attrs.iter().any(|attr| match attr.parse_meta() {
        Ok(meta) => match meta {
            Meta::List(list) => {
                (&list.ident.to_string() == "repr")
                    && list.nested.iter().next().map_or(false, |n| match n {
                        NestedMeta::Meta(Meta::Word(ref ident)) => ident.to_string() == "C",
                        _ => false,
                    })
            }
            _ => false,
        },
        Err(_) => false,
    });

    if !struct_is_repr_c {
        panic!("derive(BufferLayout) can only be used on repr(C) structs");
    }

    let struct_name = &ast.ident;

    let fields = match *fields {
        syn::Fields::Named(ref fields_named) => &fields_named.named,
        syn::Fields::Unnamed(ref fields_unnamed) => &fields_unnamed.unnamed,
        syn::Fields::Unit => panic!("BufferLayout trait cannot be derived on unit structs"),
    };

    let private_module_name = syn::Ident::new(
        &format!("__buffer_layout_{}", struct_name),
        Span::call_site(),
    );

    let mut offset_consts = Vec::new();
    let mut field_descs = Vec::new();

    for (i, f) in fields.iter().enumerate() {
        //println!("{} => {:?}", i, f.ident);
        let field_ty = &f.ty;
        /*let _field_name = f
        .ident
        .clone()
        .unwrap_or(Ident::new(&format!("unnamed_{}", i), Span::call_site()));*/

        // field offset item
        if i == 0 {
            offset_consts.push(quote!{ pub const OFFSET_0: usize = 0; pub const SIZE_0: usize = ::std::mem::size_of::<#field_ty>(); });
            field_descs.push(quote!{ (#private_module_name::OFFSET_0, <#field_ty as #gfx::interface::BufferLayout>::TYPE) });
        } else {
            let offset0 = Ident::new(&format!("OFFSET_{}", i - 1), Span::call_site());
            let offset1 = Ident::new(&format!("OFFSET_{}", i), Span::call_site());
            let size0 = Ident::new(&format!("SIZE_{}", i - 1), Span::call_site());
            let size1 = Ident::new(&format!("SIZE_{}", i), Span::call_site());
            offset_consts.push(quote! { pub const #offset1: usize =
            (#offset0+#size0)
            + (::std::mem::align_of::<#field_ty>() -
                    (#offset0+#size0)
                        % ::std::mem::align_of::<#field_ty>())
              % ::std::mem::align_of::<#field_ty>();
              pub const #size1: usize = ::std::mem::size_of::<#field_ty>();});

            field_descs.push(quote! {
               (#private_module_name::#offset1, <#field_ty as #gfx::interface::BufferLayout>::TYPE)
            });
        };
    }

    //let num_fields = field_descs.len();

    quote! {
        #[allow(non_snake_case)]
        mod #private_module_name {
            use super::*;
            #(#offset_consts)*
        }

        unsafe impl #gfx::interface::BufferLayout for #struct_name {
            const TYPE: &'static #gfx::interface::TypeDesc<'static> = &#gfx::interface::TypeDesc::Struct(&[#(#field_descs),*]);
        }
    }
}
