use proc_macro::TokenStream;
use syn::{parse_macro_input, Ident, ItemEnum};
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
    let num_variants = variant_idents.len();

    let discr = quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum #discr_ident {
            #( #variant_idents ),*
        }
    };

    macro_rules! fields {
        ($f:expr) => {
            variant_args.iter()
                .map(|&n| {
                    if n == 0 {
                        quote! {}
                    } else {
                        let param = (0..n).map($f);
                        quote! { (#(#param),*) }
                    }
                })
        };
    }

    let blank_fields = fields!(|_| quote!{_});
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

    // let build_fields = fields!(|i| quote! { parameters[#i] });
    // let build = quote! {
    //     pub fn build(discr: #discr_ident, parameters: impl ::std::iter::IntoIterator<Item=f32>) -> Option<Self> {
    //         let parameters: Vec<_> = parameters.into_iter().collect();

    //         if parameters.len() != discr.num_parameters() { return None; }

    //         Some(match discr {
    //             #(#discr_ident::#variant_idents => Self::#variant_idents #build_fields),*
    //         })
    //     }
    // };

    let build_fields = fields!(|_| quote! { parameters.next()? });
    let build = quote! {
        pub fn build(discr: #discr_ident, parameters: impl ::std::iter::IntoIterator<Item=f32>) -> Option<Self> {
            let mut parameters = parameters.into_iter();

            let var = match discr {
                #(#discr_ident::#variant_idents => Self::#variant_idents #build_fields),*
            };

            match parameters.next() {
                None => Some(var),
                _ => None
            }
        }
    };

    let const_discrs_ident = Ident::new(&format!("{}_DISCRIMINANTS", ident.to_string().to_uppercase()), ident.span());
    let const_discrs = quote! {
        pub const #const_discrs_ident: [#discr_ident; #num_variants] = [#(#discr_ident::#variant_idents),*];
    };

    // let match_branches = (0..num_variants);
    // let rand_impl = quote! {
    //     impl ::rand::distr::Distribution<#discr_ident> for #discr_ident {
    //         fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Self {
    //             match rng.random_range(0..#num_variants) {
    //                 #( #match_branches => Self::#variant_idents ),*,
    //                 _ => unreachable!()
    //             }
    //         }
    //     }
    // };

    quote! {
        #input

        #discr

        #const_discrs

        #from_impl

        // #rand_impl

        impl #ident {
            #build
        }

        impl #discr_ident {
            #num_parameters


        }
    }.into()
}
