use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Attribute, Ident, ItemEnum, Meta, Type, TypePath, parse_macro_input};

fn extract_range(attrs: &[Attribute]) -> Option<TokenStream> {
    for attr in attrs {
        if let Meta::List(ref metalist) = attr.meta
            && metalist.path.is_ident("range")
        {
            return Some(metalist.tokens.clone())
        }
    }

    None
}

#[proc_macro_attribute]
pub fn variation(_argument: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as ItemEnum);

    let ident = input.ident.clone();
    let discr_ident = format_ident!("{}Discriminant", ident);
    let dynamic_ident = format_ident!("Dynamic{}", ident);

    let variant_idents: Vec<_> = input.variants.iter().map(|v| v.ident.clone()).collect();
    let mut variant_param_counts: Vec<(usize, usize)> = vec![];
    let mut variant_int_ranges: Vec<Vec<Option<TokenStream>>> = vec![];
    let mut variant_float_ranges: Vec<Vec<Option<TokenStream>>> = vec![];

    for var in input.variants.clone() {
        let mut int_count = 0usize;
        let mut float_count = 0usize;
        let mut int_ranges = vec![];
        let mut float_ranges = vec![];

        for field in var.fields {
            if let Type::Path(TypePath { path, .. }) = field.ty {
                if path.is_ident("i8") {
                    int_count += 1;
                    int_ranges.push(extract_range(&field.attrs[..]));
                } else if path.is_ident("f32") {
                    float_count += 1;
                    float_ranges.push(extract_range(&field.attrs[..]));
                }
            }
        }

        variant_param_counts.push((int_count, float_count));
        variant_int_ranges.push(int_ranges);
        variant_float_ranges.push(float_ranges);
    }

    let num_variants = variant_idents.len();

    let discr = quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum #discr_ident {
            #( #variant_idents ),*
        }
    };

    let dynamic = quote! {
        #[derive(Debug, Clone, PartialEq)]
        pub struct #dynamic_ident {
            pub discriminant: #discr_ident,
            pub int_params: Vec<i8>,
            pub float_params: Vec<f32>,
        }
    };

    let blank_fields = variant_param_counts.iter().map(|(ni, nf)| {
        let n = ni + nf;
        if n == 0 {
            quote! {}
        } else {
            let fields = std::iter::repeat(quote!{_}).take(n);
            quote! { (#(#fields),*) }
        }
    });

    let from_impl = quote! {
        impl From<&#ident> for #discr_ident {
            fn from(val: &#ident) -> Self {
                match val {
                    #( &#ident::#variant_idents #blank_fields => Self::#variant_idents ),*
                }
            }
        }
    };

    let num_params_arms = variant_param_counts.iter()
        .map(|(i, f)| quote!{ (#i, #f) });

    let num_parameters = quote! {
        pub fn num_parameters(&self) -> (usize, usize) {
            match self {
                #( &Self::#variant_idents => #num_params_arms ),*
            }
        }
    };

    let param_ranges_arms: Vec<TokenStream> =
        variant_int_ranges.iter()
        .zip(variant_float_ranges.iter())
        .map(|(int_ranges, float_ranges)| {
            let to_option_token = |x: &Option<TokenStream>| {
                match x.clone() {
                    Some(v) => quote!{ Some(#v) },
                    None => quote! { None }
                }
            };

            let int_range_tokens = int_ranges.iter().map(to_option_token);
            let float_range_tokens = float_ranges.iter().map(to_option_token);

            quote! { (vec![#(#int_range_tokens),*], vec![#(#float_range_tokens),*]) }
        })
        .collect();

    let parameter_ranges = quote! {
        pub fn parameter_ranges(&self) -> (Vec<Option<std::ops::Range<i8>>>, Vec<Option<std::ops::Range<f32>>>) {
            match self {
                #( &Self::#variant_idents => #param_ranges_arms ),*
            }
        }
    };

    let to_dynamic_patterns = variant_param_counts.iter()
        .map(|(ni, nf)| {
            if ni + nf == 0 {
                return quote!{};
            }

            let ints = (0..*ni).map(|idx| format_ident!("i{}", idx));
            let floats = (0..*nf).map(|idx| format_ident!("f{}", idx));
            quote!{ (#(#ints,)*#(#floats),*) }
        });

    let to_dynamic_int_vecs = variant_param_counts.iter()
        .map(|(ni, _)| {
            let params = (0..*ni).map(|idx| format_ident!("i{}", idx));
            quote!{ vec![#(#params),*] }
        });

    let to_dynamic_float_vecs = variant_param_counts.iter()
        .map(|(_, nf)| {
            let params = (0..*nf).map(|idx| format_ident!("f{}", idx));
            quote!{ vec![#(#params),*] }
        });

    let into_dynamic_impl = quote! {
        impl From<#ident> for #dynamic_ident {
            fn from(val: #ident) -> Self {
                match val {
                    #(
                        #ident::#variant_idents #to_dynamic_patterns =>
                        #dynamic_ident {
                            discriminant: #discr_ident::#variant_idents,
                            int_params: #to_dynamic_int_vecs,
                            float_params: #to_dynamic_float_vecs,
                        }
                    ),*
                }
            }
        }
    };

    let from_dynamic_fields = variant_param_counts.iter()
        .map(|(ni, nf)| {
            if ni + nf == 0 {
                return quote!{};
            }

            let ints = (0..*ni).map(|idx| quote!{ val.int_params[#idx] });
            let floats = (0..*nf).map(|idx| quote!{ val.float_params[#idx] });
            quote!{ (#(#ints,)*#(#floats),*) }
        });

    let from_dynamic_impl = quote! {
        impl TryFrom<#dynamic_ident> for #ident {
            type Error = ();

            fn try_from(val: #dynamic_ident) -> Result<Self, ()> {
                Ok(match val.discriminant {
                    #(
                        #discr_ident::#variant_idents =>
                        #ident::#variant_idents #from_dynamic_fields
                    ),*
                })
            }
        }
    };

    let const_discrs_ident = Ident::new(
        &format!("{}_DISCRIMINANTS", ident.to_string().to_uppercase()),
        ident.span(),
    );
    let const_discrs = quote! {
        pub const #const_discrs_ident: [#discr_ident; #num_variants] = [#(#discr_ident::#variant_idents),*];
    };

    // strip #[range(...)] attrs from the output enum so the compiler
    // doesn't see unknown attributes on struct fields
    let mut cleaned_input = input.clone();
    for variant in &mut cleaned_input.variants {
        for field in &mut variant.fields {
            field.attrs.retain(|attr| !attr.path().is_ident("range"));
        }
    }

    quote! {
        #cleaned_input

        #discr

        #dynamic

        #const_discrs

        #from_impl

        #into_dynamic_impl

        #from_dynamic_impl

        impl #discr_ident {
            #num_parameters

            #parameter_ranges
        }
    }
    .into()
}
