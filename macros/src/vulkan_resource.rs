use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

pub fn impl_derive_vulkan_resource(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let name = &ast.ident;

    let gen = quote! {
        impl crate::engine::graphics::vulkan::util::VulkanResource for #name {}
    };

    gen.into()
}
