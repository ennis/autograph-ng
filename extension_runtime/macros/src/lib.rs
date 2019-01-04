#![feature(proc_macro_diagnostic)]
#![recursion_limit = "128"]
extern crate proc_macro;
extern crate proc_macro2;

use proc_macro2::Span;
use quote::quote;
use syn;

macro_rules! format_ident {
    ($($arg:tt)*) => { syn::Ident::new(&format!($($arg)*), Span::call_site()) };
}

fn rt_path() -> syn::Path {
    syn::parse_str("::gfx2_extension_runtime").unwrap()
}

/// Checks if the function is generic (i.e. if the function has generic type parameters,
/// or `impl Traits` in arguments or return type).
fn is_function_generic(decl: &syn::FnDecl) -> bool {
    unimplemented!()
}

fn rewrite_lifetimes_in_path(path: &syn::Path, l: &syn::Lifetime) -> syn::Path
{
    syn::Path {
        segments: path.segments.pairs().map(|p| {
            let arguments = match p.value().arguments {
                syn::PathArguments::AngleBracketed(abga) =>
                    syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                        args: abga.args.pairs().map(|p| {
                            let new_arg = match p.value() {
                                syn::GenericArgument::Lifetime(_) => syn::GenericArgument::Lifetime(l.clone()),
                                syn::GenericArgument::Type(ty) => rewrite_lifetimes(ty, l),
                                other => other.clone()
                            };
                            syn::punctuated::Pair::new(new_arg, p.cloned().cloned())
                        }),
                        ..abga.clone()
                    }),
                syn::PathArguments::Parenthesized(pga) => {
                    // TODO ???
                    syn::PathArguments::Parenthesized(pga.clone())
                }
            };
            let new_seg = syn::PathSegment {
                arguments,
                ident: p.value().ident.clone(),
            };
            syn::punctuated::Pair::new(new_seg, p.punct().cloned().cloned())
        }).collect(),
        ..path.clone()
    }
}

// Replace all lifetimes appearing in the type, and add the lifetime to any reference type
// found.
fn rewrite_lifetimes(ty: &syn::Type, l: &syn::Lifetime) -> syn::Type {
    match ty {
        syn::Type::Slice(tyslice) => {
            // slices &[T]
            syn::Type::Slice(syn::TypeSlice {
                elem: Box::new(rewrite_lifetimes(&tyslice.elem, l)),
                ..tyslice.clone()
            })
        }
        syn::Type::Array(tyarray) => {
            // arrays []
            syn::Type::Array(syn::TypeArray {
                elem: Box::new(rewrite_lifetimes(&tyarray.elem, l)),
                ..tyarray.clone()
            })
        }
        syn::Type::Ptr(typtr) => {
            syn::Type::Ptr(syn::TypePtr {
                elem: Box::new(rewrite_lifetimes(&typtr.elem, l)),
                ..typtr.clone()
            })
        },
        syn::Type::Reference(tyref) => {
            syn::Type::Reference(syn::TypeReference {
                elem: Box::new(rewrite_lifetimes(&tyref.elem, l)),
                lifetime: Some(l.clone()),
                ..tyref.clone()
            })
        },
        syn::Type::BareFn(tybarefn) => {
            // TODO?
            ty.clone()
        },
        syn::Type::Never(_) => {
            ty.clone()
        },
        syn::Type::Tuple(tytuple) => {
            syn::Type::Tuple(syn::TypeTuple {
                elems: tytuple.elems.pairs().map(|p| {
                    syn::punctuated::Pair::new(rewrite_lifetimes(p.value(), l), p.punct().cloned().cloned())  // hmmm
                }).collect(),
                ..tytuple.clone()
            })
        },
        syn::Type::Path(tyslice) => {
            // nothing we can do here
            ty.clone()
        },
        syn::Type::TraitObject(tytraitobj) => {
            syn::Type::TraitObject(syn::TypeTraitObject {
                bounds: tytraitobj.bounds.pairs().map(|p| {
                    let r = match p.value() {
                        syn::TypeParamBound::Trait(traitbound) => {
                            syn::TypeParamBound::Trait(syn::TraitBound {
                                path: rewrite_lifetimes_in_path(&traitbound.path, l),
                                ..traitbound.clone()
                            })
                        },
                        syn::TypeParamBound::Lifetime(_) => {
                            syn::TypeParamBound::Lifetime(l.clone())
                        }
                    };
                    syn::punctuated::Pair::new(r, p.punct().cloned().cloned())
                })
            })
        },
        syn::Type::ImplTrait(tyslice) => unimplemented!(),
        syn::Type::Paren(tyslice) => unimplemented!(),
        syn::Type::Group(tyslice) => unimplemented!(),
        syn::Type::Infer(tyslice) => unimplemented!(),
        syn::Type::Macro(tyslice) => unimplemented!(),
        syn::Type::Verbatim(tyslice) => unimplemented!(),
    }
}

#[proc_macro_attribute]
pub fn hot_reload_module(
    _attribs: proc_macro::TokenStream,
    src: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    // parse a whole module
    let m: syn::ItemMod = syn::parse_macro_input!(src as syn::ItemMod);

    let mut fnsymnames = Vec::new();
    let mut fnptrs = Vec::new();

    // Collect hot-reloadable items and generate signatures ----------------------------------------
    if let Some((_, ref contents)) = m.content {
        for item in contents.iter() {
            match item {
                // Functions -----------------------------------------------------------------------
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
                            syn::FnArg::SelfRef(_) => unimplemented!("methods"),
                            syn::FnArg::SelfValue(_) => unimplemented!("methods"),
                            syn::FnArg::Captured(syn::ArgCaptured { ty, .. }) => {
                                argtypes.push(ty);
                            }
                            syn::FnArg::Inferred(_) => panic!("inferred arg on fn item"),
                            syn::FnArg::Ignored(_) => panic!("ignored arg on fn item"),
                        }
                    }

                    // Note: the where clause and lifetime constraints are ignored, since
                    // there is currently no way to spell a higher-ranked fn type with constraints
                    // on the lifetimes. (e.g. for <'a> where <'a: ...> fn() -> ...).
                    // TODO: detect those cases and display an error message.
                    let bound_lifetimes = if generics.params.is_empty() {
                        quote! {}
                    } else {
                        quote! {for #generics}
                    };
                    let fnptr = quote! { #bound_lifetimes #unsafety fn (#(#argtypes),*) #output };

                    fnsymnames.push(ident);
                    fnptrs.push(fnptr);
                }
                // Constants -----------------------------------------------------------------------
                syn::Item::Const(itemconst) => {
                    let _ty = &itemconst.ty;
                    let _ident = &itemconst.ident;

                    // For constants, the symbol will just be a pointer to the data, so just stick
                    // the '__lib lifetime to it.
                    //
                    // Still, the very common case of `&'static str` is wildly unsafe: it's very
                    // easy to deref and copy the pointer away.
                    // Mitigation: track and replace all instances of static in the type.
                    // Limits to the mitigation: `'static` can be elided, or smuggled inside
                    // a type alias or wrapper type.
                    // Solution: prevent moving away? but shared refs are always copy
                    //
                    // This should be tractable in some cases (no type aliases, or wrappers)
                    // by syntactically adding 'lib to all refs that appear in the type.
                    // (we can assume that all refs are static anyway, so can work around some elision cases)
                    // When the elision happens for a lifetime parameter, then will fail to
                    // compile anyway (cannot elide to static in the struct).
                    // Smuggling with wrappers is still an issue.
                    // But this solves the case of &'static str and friends.
                    //
                    // Note: user must un-elide 'static lifetimes for constants
                    // for consistency, maybe also require un-elision of lifetimes in function decls?
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
        let rt = rt_path();
        let rt0 = std::iter::repeat(&rt);
        let rt1 = std::iter::repeat(&rt);

        let content = match m.content {
            Some((_, ref items)) => {
                quote! {#(#items)*}
            }
            None => quote! {},
        };

        quote! {
            #(#attrs)* #vis #mod_token #mod_name {
                #[doc(hidden)]
                pub mod __load {
                    pub struct FnPtrs<'__lib> {
                        #(pub #fnsymnames0: #rt0::FnWrap<'__lib, #fnptrs>,)*
                    }
                    impl<'__lib> FnPtrs<'__lib> {
                        pub fn load(lib: &'__lib libloading::Library) -> libloading::Result<Self> {
                            Ok(Self {
                                #(#fnsymnames0: #rt1::FnWrap(unsafe { *lib.get(stringify!(#fnsymnames1).as_bytes())? }, ::std::marker::PhantomData),)*
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
