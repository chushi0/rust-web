use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_derive(TwoValue)]
pub fn two_value_enum_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();

    impl_two_value_enum(&ast)
}

fn impl_two_value_enum(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let syn::Data::Enum(data) = &ast.data else {
        panic!("#[derive(TwoValueEnum)] is only support enum type");
    };
    let token = &data.variants;
    if token.len() != 2 {
        panic!("#[derive(TwoValueEnum)] is only support 2-value enum")
    }
    let mut iter = token.iter();
    let mut next_varient = || {
        let varient = iter.next().unwrap();
        let syn::Fields::Unit = varient.fields else {
            panic!("#[derive(TwoValueEnum)] is only support unit enum");
        };
        varient.ident.clone()
    };
    let varient_1 = next_varient();
    let varient_2 = next_varient();

    let gen = quote! {
        impl TwoValueEnum for #name {
            fn opposite(&self) -> Self {
                match self {
                    #name::#varient_1 => #name::#varient_2,
                    #name::#varient_2 => #name::#varient_1,
                }
            }
        }
    };
    gen.into()
}
