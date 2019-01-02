#![feature(proc_macro_diagnostic)]
#![recursion_limit="128"]
extern crate proc_macro;
extern crate proc_macro2;

use proc_macro2::Span;
use quote::quote;
use syn;
//use syn::parse::ParseStream;
//use syn::Token;

macro_rules! format_ident {
    ($($arg:tt)*) => { syn::Ident::new(&format!($($arg)*), Span::call_site()) };
}

/*
struct LoadModule {
    lib: syn::Expr,
    path: syn::Path,
}

impl syn::parse::Parse for LoadModule {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        let lib: syn::Expr = input.parse()?;
        input.parse::<Token![,]>()?;
        let path: syn::Path = input.parse()?;
        Ok(LoadModule {lib, path})
    }
}

#[proc_macro]
pub fn load_module(src: proc_macro::TokenStream) -> proc_macro::TokenStream
{
    // expr, module path
    let LoadModule { lib, path } = syn::parse_macro_input!(src as LoadModule);
    let q = quote! {
        #path::__load::DllShims::load(#lib)
    };
    q.into()
}*/

#[proc_macro_attribute]
pub fn hot_reload_module(
    _attribs: proc_macro::TokenStream,
    src: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    //src

    // parse a whole module
    let m: syn::ItemMod = syn::parse_macro_input!(src as syn::ItemMod);

    let mut fnsymnames = Vec::new();
    let mut fnptrs = Vec::new();

    // Collect hot-reloadable items and generate signatures ----------------------------------------
    if let Some((_, ref contents)) = m.content {
        for item in contents.iter() {
            match item {
                syn::Item::Fn(itemfn) => {
                    let unsafety = &itemfn.unsafety;
                    let _abi = &itemfn.abi;
                    let _attrs = &itemfn.attrs;
                    let _asyncness = itemfn.asyncness;
                    let inputs = &itemfn.decl.inputs;
                    let output = &itemfn.decl.output;
                    let ident = &itemfn.ident;
                    let _where_clause = &itemfn.decl.generics.where_clause;
                    let generics = &itemfn.decl.generics;

                    if generics.type_params().next().is_some() {
                        // skip functions with generic type parameters, these are not hot-reloadable
                        continue;
                    }

                    // generate fn type ------------------------------------------------------------
                    let mut argtypes = Vec::new();
                    for arg in inputs.iter() {
                        match arg {
                            syn::FnArg::SelfRef(_) => { unimplemented!("methods") },
                            syn::FnArg::SelfValue(_) => { unimplemented!("methods") },
                            syn::FnArg::Captured(syn::ArgCaptured { ty, .. }) => {
                                argtypes.push(ty);
                            },
                            syn::FnArg::Inferred(_) => {
                                panic!("inferred arg on fn item")
                            },
                            syn::FnArg::Ignored(_) => {
                                panic!("ignored arg on fn item")
                            },
                        }
                    }

                    // Note: the where clause and lifetime constraints are ignored, since
                    // there is currently no way to spell a higher-ranked fn type with constraints
                    // on the lifetimes. (e.g. for <'a> where <'a: ...> fn() -> ...).
                    // TODO: detect those cases and display an error message.
                    let bound_lifetimes = if generics.params.is_empty() {
                        quote!{}
                    } else {
                        quote!{for #generics}
                    };
                    let fnptr = quote! { #bound_lifetimes #unsafety fn (#(#argtypes),*) #output };

                    fnsymnames.push(ident);
                    fnptrs.push(fnptr);
                }
                syn::Item::Const(itemconst) => {
                    let _ty = &itemconst.ty;
                    let _ident = &itemconst.ident;
                }
                _ => {}
            }
        }
    }

    // Generate stub -------------------------------------------------------------------------------
    let r = {
        let vis = m.vis;
        let mod_token = m.mod_token;
        let mod_name = m.ident;
        let attrs = m.attrs;
        let fnptrs = &fnptrs;
        let fnsymnames0 = &fnsymnames;
        let fnsymnames1 = &fnsymnames;

        let content = match m.content {
            Some((_, ref items)) => { quote! {#(#items)*} },
            None => quote!{}
        };

        quote! {
            #(#attrs)* #vis #mod_token #mod_name {
                #[doc(hidden)]
                pub mod __load {
                    pub struct FnPtrs<'__lib> {
                        #(pub #fnsymnames0: libloading::Symbol<'__lib, #fnptrs>,)*
                    }
                    impl<'__lib> FnPtrs<'__lib> {
                        pub fn load(lib: &'__lib libloading::Library) -> libloading::Result<Self> {
                            Ok(Self {
                                #(#fnsymnames0: unsafe { lib.get(stringify!(#fnsymnames1).as_bytes())? },)*
                            })
                        }
                    }
                }

                #content
            }
        }
    };

    println!("{}", r.to_string());
    r.into()
}
