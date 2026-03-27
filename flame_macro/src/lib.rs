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
    // let variant_args: Vec<_> = input.variants.iter().map(|v| v.fields.len()).collect();
    let mut variant_args: Vec<(usize,usize)> = vec![];
    let mut variant_int_arg_ranges: Vec<Vec<Option<TokenStream>>> = vec![];
    let mut variant_float_arg_ranges: Vec<Vec<Option<TokenStream>>> = vec![];

    for var in input.variants.clone() {
        let mut int_args = 0usize;
        let mut float_args = 0usize;
        let mut int_arg_ranges = vec![];
        let mut float_arg_ranges = vec![];

        for field in var.fields {
            if let Type::Path(TypePath { path, .. }) = field.ty {
                if path.is_ident("i8") {
                    int_args += 1;
                    int_arg_ranges.push(extract_range(&field.attrs[..]));
                } else if path.is_ident("f32") {
                    float_args += 1;
                    float_arg_ranges.push(extract_range(&field.attrs[..]));
                }
            }
        }

        variant_args.push((int_args, float_args));
        variant_int_arg_ranges.push(int_arg_ranges);
        variant_float_arg_ranges.push(float_arg_ranges);
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

    // macro_rules! fields {
    //     ($f:expr) => {
    //         variant_args.iter().map(|&n| {
    //             if n == 0 {
    //                 quote! {}
    //             } else {
    //                 let param = (0..n).map($f);
    //                 quote! { (#(#param),*) }
    //             }
    //         })
    //     };
    // }

    // let blank_fields = fields!(|_| quote! {_});
    
    let blank_fields = variant_args.iter().map(|(ni, nf)| {
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

    let num_parameters_rhs = variant_args.iter()
        .map(|(i, f)| quote!{ (#i, #f) });
    
    let num_parameters = quote! {
        pub fn num_parameters(&self) -> (usize, usize) {
            match self {
                #( &Self::#variant_idents => #num_parameters_rhs ),*
            }
        }
    };

    let parameter_ranges_rhs: Vec<TokenStream> =
        variant_int_arg_ranges.iter()
        .zip(variant_float_arg_ranges.iter())
        .map(|(vi, vf)| {
            let opt_trans = |x: &Option<TokenStream>| {
                match x.clone() {
                    Some(v) => quote!{ Some(#v) },
                    None => quote! { None }
                }
            };

            let vi = vi.iter().map(opt_trans);
            let vf = vf.iter().map(opt_trans);

            quote! { (vec![#(#vi),*], vec![#(#vf),*]) }
        })
        .collect();
    
    let parameter_ranges = quote! {
        pub fn parameter_ranges(&self) -> (Vec<Option<std::ops::Range<i8>>>, Vec<Option<std::ops::Range<f32>>>) {
            match self {
                #( &Self::#variant_idents => #parameter_ranges_rhs ),*
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

    // let build_fields = fields!(|_| quote! { parameters.next()? });

    // let build = quote! {
    //     pub fn build(discr: #discr_ident, parameters: impl ::std::iter::IntoIterator<Item=f32>) -> Option<Self> {
    //         let mut parameters = parameters.into_iter();

    //         let var = match discr {
    //             #(#discr_ident::#variant_idents => Self::#variant_idents #build_fields),*
    //         };

    //         match parameters.next() {
    //             None => Some(var),
    //             _ => None
    //         }
    //     }
    // };

    // let deconstruct_patterns = variant_args.iter()
    //     .map(|&n| {
    //         if n == 0 {
    //             quote! {}
    //         } else {
    //             let params = (0..n).map(|i| format_ident!("p{}", i));
    //             quote! { (#(#params),*) }
    //         }
    //     });

    // let deconstruct_fields = fields!(|i| format_ident!("p{}", i));

    // let deconstruct_vecs = variant_args.iter().map(|&n| {
    //     let params = (0..n).map(|i| format_ident!("p{}", i));
    //     quote! { vec![#(#params),*] }
    // });

    // let deconstruct = quote! {
    //     pub fn deconstruct(self) -> (#discr_ident, Vec<f32>) {
    //         match self {
    //             #( #ident::#variant_idents #deconstruct_fields => (#discr_ident::#variant_idents, #deconstruct_vecs) ),*
    //         }
    //     }
    // };
    
    let orig_to_dynamic_pattern = variant_args.iter()
        .map(|(ni, nf)| {
            if ni + nf == 0 {
                return quote!{};
            }

            let ints = (0..*ni).map(|idx| format_ident!("i{}", idx));
            let floats = (0..*nf).map(|idx| format_ident!("f{}", idx));
            quote!{ (#(#ints,)*#(#floats),*) }
        });

    let orig_to_dynamic_int_vecs = variant_args.iter()
        .map(|(ni, _)| {
            let params = (0..*ni).map(|idx| format_ident!("i{}", idx));
            quote!{ vec![#(#params),*] }
        });

    let orig_to_dynamic_float_vecs = variant_args.iter()
        .map(|(_, nf)| {
            let params = (0..*nf).map(|idx| format_ident!("f{}", idx));
            quote!{ vec![#(#params),*] }
        });

    let orig_to_dynamic = quote! {
        impl From<#ident> for #dynamic_ident {
            fn from(val: #ident) -> Self {
                match val {
                    #(
                        #ident::#variant_idents #orig_to_dynamic_pattern =>
                        #dynamic_ident {
                            discriminant: #discr_ident::#variant_idents,
                            int_params: #orig_to_dynamic_int_vecs,
                            float_params: #orig_to_dynamic_float_vecs,
                        }
                    ),*
                }
            }
        }
    };

    let dynamic_to_orig_fields = variant_args.iter()
        .map(|(ni, nf)| {
            if ni + nf == 0 {
                return quote!{};
            }

            let ints = (0..*ni).map(|idx| quote!{ val.int_params[#idx] });
            let floats = (0..*nf).map(|idx| quote!{ val.float_params[#idx] });
            quote!{ (#(#ints,)*#(#floats),*) }
        });

    let dynamic_to_orig = quote! {
        impl TryFrom<#dynamic_ident> for #ident {
            type Error = ();

            fn try_from(val: #dynamic_ident) -> Result<Self, ()> {
                Ok(match val.discriminant {
                    #(
                        #discr_ident::#variant_idents =>
                        #ident::#variant_idents #dynamic_to_orig_fields
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

        // #rand_impl

        // impl #ident {
        //     #build

        //     #deconstruct
        // }

        #orig_to_dynamic

        #dynamic_to_orig

        impl #discr_ident {
            #num_parameters

            #parameter_ranges
        }
    }
    .into()
}
