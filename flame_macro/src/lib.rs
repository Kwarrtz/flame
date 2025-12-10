use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemEnum};
use quote::{quote, format_ident};

#[proc_macro_attribute]
pub fn variation(_argument: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemEnum);

    let ident = input.ident.clone();
    let discr_ident = format_ident!("{}Discriminant", ident);

    let variant_idents: Vec<_> = input.variants.iter()
        .map(|v| v.ident.clone())
        .collect();
    let variant_args: Vec<_> = input.variants.iter()
        .map(|v| v.fields.len())
        .collect();

    let head_variant_ident = &variant_idents[0];
    let tail_variant_idents = &variant_idents[1..];
    let discr = quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum #discr_ident {
            #head_variant_ident = 0,
            #( #tail_variant_idents ),*
        }
    };

    let blank_fields = variant_args.iter()
        .map(|&n| {
            if n == 0 {
                quote!()
            } else {
                let underscores = std::iter::repeat(quote!{_}).take(n);
                quote! { (#(#underscores),*) }
            }
        });
    let from_impl = quote! {
        impl From<#ident> for #discr_ident {
            fn from(val: #ident) -> Self {
                match val {
                    #( #ident::#variant_idents #blank_fields => Self::#variant_idents ),*
                }
            }
        }
    };

    let num_parameters = quote! {
        pub fn num_parameters(&self) -> usize {
            match self {
                #( &Self::#variant_idents => #variant_args ),*
            }
        }
    };

    let build_fields = variant_args.iter()
        .map(|&n| {
            if n == 0 {
                quote!()
            } else {
                let params = (0..n).map(|i| quote!(parameters[#i]));
                quote! { (#(#params),*) }
            }
        });
    let build = quote! {
        pub fn build(discr: #discr_ident, parameters: impl ::std::iter::IntoIterator<Item=f32>) -> Option<Self> {
            let parameters: Vec<_> = parameters.into_iter().collect();

            if parameters.len() != discr.num_parameters() { return None; }

            Some(match discr {
                #(#discr_ident::#variant_idents => Self::#variant_idents #build_fields),*
            })
        }
    };

    quote! {
        #input

        #discr

        #from_impl

        impl #ident {
            #build
        }

        impl #discr_ident {
            #num_parameters
        }
    }.into()
}
