#![feature(proc_macro_diagnostic)]
#![recursion_limit = "128"]
extern crate proc_macro;
extern crate proc_macro2;

use darling::usage::{LifetimeSet, Purpose, UsesLifetimes};
use proc_macro2::Span;
use quote::quote;
use syn;
//use syn::parse::ParseStream;
//use syn::Token;

macro_rules! format_ident {
    ($($arg:tt)*) => { syn::Ident::new(&format!($($arg)*), Span::call_site()) };
}

fn rewrite_lifetimes_in_path(path: &syn::Path, l: &syn::Lifetime) -> syn::Path {
    syn::Path {
        segments: path
            .segments
            .pairs()
            .map(|p| {
                let arguments = match &p.value().arguments {
                    syn::PathArguments::AngleBracketed(abga) => {
                        syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                            args: abga
                                .args
                                .pairs()
                                .map(|p| {
                                    let new_arg = match p.value() {
                                        syn::GenericArgument::Lifetime(_) => {
                                            syn::GenericArgument::Lifetime(l.clone())
                                        }
                                        syn::GenericArgument::Type(ty) => {
                                            syn::GenericArgument::Type(rewrite_lifetimes(ty, l))
                                        }
                                        &other => other.clone(),
                                    };
                                    syn::punctuated::Pair::new(new_arg, p.punct().cloned().cloned())
                                })
                                .collect(),
                            ..abga.clone()
                        })
                    }
                    syn::PathArguments::Parenthesized(pga) => {
                        // TODO ???
                        syn::PathArguments::Parenthesized(pga.clone())
                    }
                    other => other.clone(),
                };
                let new_seg = syn::PathSegment {
                    arguments,
                    ident: p.value().ident.clone(),
                };
                syn::punctuated::Pair::new(new_seg, p.punct().cloned().cloned())
            })
            .collect(),
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
        syn::Type::Ptr(typtr) => syn::Type::Ptr(syn::TypePtr {
            elem: Box::new(rewrite_lifetimes(&typtr.elem, l)),
            ..typtr.clone()
        }),
        syn::Type::Reference(tyref) => syn::Type::Reference(syn::TypeReference {
            elem: Box::new(rewrite_lifetimes(&tyref.elem, l)),
            lifetime: Some(l.clone()),
            ..tyref.clone()
        }),
        syn::Type::BareFn(_tybarefn) => {
            // TODO?
            ty.clone()
        }
        syn::Type::Never(_) => ty.clone(),
        syn::Type::Tuple(tytuple) => {
            syn::Type::Tuple(syn::TypeTuple {
                elems: tytuple
                    .elems
                    .pairs()
                    .map(|p| {
                        syn::punctuated::Pair::new(
                            rewrite_lifetimes(p.value(), l),
                            p.punct().cloned().cloned(),
                        ) // hmmm
                    })
                    .collect(),
                ..tytuple.clone()
            })
        }
        syn::Type::Path(typath) => syn::Type::Path(syn::TypePath {
            path: rewrite_lifetimes_in_path(&typath.path, l),
            qself: typath.qself.clone(),
        }),
        syn::Type::TraitObject(tytraitobj) => syn::Type::TraitObject(syn::TypeTraitObject {
            bounds: tytraitobj
                .bounds
                .pairs()
                .map(|p| {
                    let r = match p.value() {
                        syn::TypeParamBound::Trait(traitbound) => {
                            syn::TypeParamBound::Trait(syn::TraitBound {
                                path: rewrite_lifetimes_in_path(&traitbound.path, l),
                                ..traitbound.clone()
                            })
                        }
                        syn::TypeParamBound::Lifetime(_) => {
                            syn::TypeParamBound::Lifetime(l.clone())
                        }
                    };
                    syn::punctuated::Pair::new(r, p.punct().cloned().cloned())
                })
                .collect(),
            ..tytraitobj.clone()
        }),
        syn::Type::ImplTrait(_tyslice) => unimplemented!(),
        syn::Type::Paren(_tyslice) => unimplemented!(),
        syn::Type::Group(_tyslice) => unimplemented!(),
        syn::Type::Infer(_tyslice) => unimplemented!(),
        syn::Type::Macro(_tyslice) => unimplemented!(),
        syn::Type::Verbatim(_tyslice) => unimplemented!(),
    }
}

#[proc_macro_attribute]
pub fn hot_reload_module(
    _attribs: proc_macro::TokenStream,
    src: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    //src

    // parse a whole module
    let m: syn::ItemMod = syn::parse_macro_input!(src as syn::ItemMod);

    let mut shims = Vec::new();
    let mut fnsymnames = Vec::new();
    let mut fnptrs = Vec::new();
    let mut varsymnames = Vec::new();
    let mut varptrtys = Vec::new();

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
                    let static_lifetime = syn::parse_quote! { 'static };

                    // extract argument types ------------------------------------------------------
                    let mut argtypes = Vec::new();
                    // generate new names
                    let mut argnames = Vec::new();
                    for (i, arg) in inputs.iter().enumerate() {
                        match arg {
                            syn::FnArg::SelfRef(_) => unimplemented!("methods"),
                            syn::FnArg::SelfValue(_) => unimplemented!("methods"),
                            syn::FnArg::Captured(syn::ArgCaptured { ty, .. }) => {
                                let an = format_ident!("arg{}", i);
                                // clone ty because darling lacks an impl of UsesLifetimes for &T where T: UsesLifetimes
                                argtypes.push(ty.clone());
                                argnames.push(an);
                            }
                            syn::FnArg::Inferred(_) => panic!("inferred arg on fn item"),
                            syn::FnArg::Ignored(_) => panic!("ignored arg on fn item"),
                        }
                    }

                    // extract lifetimes in signature ----------------------------------------------
                    let mut lifetimes_and_static = itemfn
                        .decl
                        .generics
                        .lifetimes()
                        .map(|lt| lt.lifetime.clone())
                        .collect::<LifetimeSet>();
                    lifetimes_and_static.insert(static_lifetime);

                    // input+output lifetimes in the signature, potentially including 'static
                    // (easy way to get an error if 'static syntactically appears in the signature)
                    let mut lifetimes =
                        argtypes.uses_lifetimes(&Purpose::Declare.into(), &lifetimes_and_static);
                    lifetimes.extend(
                        itemfn
                            .decl
                            .output
                            .uses_lifetimes(&Purpose::Declare.into(), &lifetimes_and_static),
                    );
                    let lifetimes = &lifetimes;

                    // Add our lifetime bounds -----------------------------------------------------
                    let mut adjusted_generics: syn::Generics = itemfn.decl.generics.clone();

                    let bounded = true;
                    if bounded {
                        if !lifetimes.is_empty() {
                            // constrain *all* lifetimes, not only those in output position:
                            // we may have something like &mut Vec<&'a i32>,with 'a appearing only
                            // in input position. Without putting the bound on 'a,
                            // we can smuggle a ref that outlives the dylib.
                            adjusted_generics
                                .make_where_clause()
                                .predicates
                                .push(syn::parse_quote! {'__lib: #(#lifetimes)+*});
                        }
                    }

                    // generate shim ---------------------------------------------------------------

                    // We store the function pointers as raw pointers, because AFAIK
                    // there is currently no way to spell a higher-ranked fn type with constraints
                    // on the lifetimes. (e.g. for <'a> where <'a: ...> fn() -> ...)
                    //
                    // To perform the call, we transmute the pointer to the correct function type
                    // at the last moment, inside the generated shim (which has the lifetimes).
                    // In this context the lifetimes are already fixed so there is no need to spell
                    // a higher-ranked fn type.

                    // add DylibSafe bounds on argument types
                    if bounded {
                        /* /*let mut mkbound = |ty: &syn::Type| {
                            adjusted_generics.make_where_clause().predicates.push(
                                syn::parse2::<syn::WherePredicate>(quote_spanned! {ty.span() => #ty: gfx2_extension_runtime::DylibSafe }).unwrap())
                        };*/
                        let mut allty = Vec::new();
                        for ty in argtypes.iter() {
                        //mkbound(ty);
                        allty.push(quote_spanned!{ ty.span() => #ty })
                        }
                        match output {
                        syn::ReturnType::Type(_, ty) => {
                        allty.push(quote_spanned!{ ty.span() => #ty });
                        }
                        syn::ReturnType::Default => {}
                        }

                        adjusted_generics.make_where_clause().predicates.push(
                        syn::parse_quote!{(#(#allty),*): gfx2_extension_runtime::DylibSafe })*/
                    }

                    // generate shims
                    let fnptr_ident = format_ident!("fnptr_{}", ident.to_string());
                    let argnames0 = &argnames;
                    let argnames1 = &argnames;
                    let where_clause = &adjusted_generics.where_clause;

                    let shim = quote! {
                        pub #unsafety fn #ident #adjusted_generics (&self, #(#argnames0: #argtypes),*) #output
                            #where_clause
                        {
                            // the lifetimes contained in inputs and outputs
                            // are fixed in this context, no need for a higher-ranked type.
                            (unsafe {::std::mem::transmute::<_, fn(#inputs) #output>(*self.#fnptr_ident) }) (#(#argnames1),*)
                        }
                    };

                    shims.push(shim);
                    fnsymnames.push(ident);
                    fnptrs.push(fnptr_ident);
                }
                syn::Item::Static(itemstatic) => {
                    let ty = &itemstatic.ty;
                    let ident = &itemstatic.ident;
                    // replace all lifetimes with 'lib (all lifetimes should be 'static anyway, except higher-ranked ones).
                    // this also performs limited 'syntactical' un-elision by adding '__lib in all
                    // positions where a lifetime is known to be elided.
                    // (it won't add the bound on types with an elided lifetime param since there is
                    // no way to syntactically know that there should be a lifetime there: this will generate
                    // an error later in the process anyway)
                    let lib_lifetime: syn::Lifetime = syn::parse_quote! { '__lib };
                    let ty2 = rewrite_lifetimes(ty, &lib_lifetime);
                    varptrtys.push(ty2);
                    varsymnames.push(ident);
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
        //let wrapper_name = format_ident!("DllShimsFor_{}", mod_name.to_string());
        let fnptrs = &fnptrs;
        let varsymnames = &varsymnames;
        let varsymnames0 = varsymnames;

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
                    pub struct DllShims<'__lib> {
                        // function pointers
                        #(#fnptrs: ::libloading::Symbol<'__lib, *const ::std::ffi::c_void>,)*
                        // variable pointers (public)
                        #(pub #varsymnames: &'__lib #varptrtys ,)*
                    }
                    impl<'__lib> DllShims<'__lib> {
                        #(#shims)*

                        pub fn load(lib: &'__lib ::autograph_plugin::Dylib) -> ::libloading::Result<Self> {
                            Ok(Self {
                                #(#fnptrs: unsafe { lib.get(stringify!(#fnsymnames))? },)*
                                #(#varsymnames: unsafe { *lib.get(stringify!(#varsymnames0))? },)*
                            })
                        }
                    }
                }

                #content
            }
        }
    };

    //println!("{}", r.to_string());
    r.into()
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
