#![allow(non_snake_case)]

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, LitInt};

#[proc_macro]
pub fn create_unbounded_channels(input: TokenStream) -> TokenStream {
    let n = parse_macro_input!(input as LitInt);
    let n = n.base10_parse::<usize>().unwrap();

    let mut channel_declarations = Vec::new();

    for i in 0..n {
        let tx_ident = syn::Ident::new(&format!("channel_{}_tx", i), proc_macro2::Span::call_site());
        let rx_ident = syn::Ident::new(&format!("channel_{}_rx", i), proc_macro2::Span::call_site());

        let decl = quote! {
            let (#tx_ident, #rx_ident) = mpsc::unbounded_channel();
        };
        channel_declarations.push(decl);
    }

    let output = quote! {
        #(#channel_declarations)*
    };

    output.into()
}

#[proc_macro_derive(DefinePacketWithData)]
pub fn define_packet_with_data(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;

    let mut has_length = false;
    let mut has_id = false;
    let mut has_data = false;
    let mut has_packet_type = false;

    if let Data::Struct(data_struct) = &input.data {
        if let Fields::Named(fields_named) = &data_struct.fields {
            for field in &fields_named.named {
                let field_name = field.ident.as_ref().unwrap().to_string();
                match field_name.as_str() {
                    "length" => has_length = true,
                    "id" => has_id = true,
                    "data" => has_data = true,
                    "packet_type" => has_packet_type = true,
                    _ => (),
                }
            }
        }
    }

    if !(has_length && has_id && has_data && has_packet_type) {
        return syn::Error::new_spanned(
            &input.ident,
            "Struct must have fields: length, id, data, packet_type",
        )
            .to_compile_error()
            .into();
    }

    let expanded = quote! {
        impl #name {
            pub fn new(data: Vec<u8>) -> Self {
                let packet_type = PacketType::#name;
                Self {
                    length: length_to_byte(16 + data.len()),
                    id: packet_type.as_byte(),
                    data,
                    packet_type,
                }
            }
        }

        impl Packet for #name {
            fn as_length_byte(&self) -> &[u8] {
                &self.length
            }

            fn as_id_byte(&self) -> &[u8] {
                &self.id
            }

            fn as_data_byte(&self) -> &[u8] {
                &self.data
            }

            fn clone_length_byte(&self) -> Vec<u8> {
                self.length.clone()
            }

            fn clone_id_byte(&self) -> Vec<u8> {
                self.id.clone()
            }

            fn clone_data_byte(&self) -> Vec<u8> {
                self.data.clone()
            }

            fn data_to_string(&self) -> String {
                String::from_utf8_lossy(&self.data).to_string()
            }

            fn packet_type(&self) -> PacketType {
                self.packet_type
            }

            fn equal(&self, packet_type: PacketType) -> bool {
                self.packet_type == packet_type
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(DefinePacketWithoutData)]
pub fn define_packet_without_data(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;

    let mut has_length = false;
    let mut has_id = false;
    let mut has_data = false;
    let mut has_packet_type = false;

    if let Data::Struct(data_struct) = &input.data {
        if let Fields::Named(fields_named) = &data_struct.fields {
            for field in &fields_named.named {
                let field_name = field.ident.as_ref().unwrap().to_string();
                match field_name.as_str() {
                    "length" => has_length = true,
                    "id" => has_id = true,
                    "data" => has_data = true,
                    "packet_type" => has_packet_type = true,
                    _ => (),
                }
            }
        }
    }

    if !(has_length && has_id && has_data && has_packet_type) {
        return syn::Error::new_spanned(
            &input.ident,
            "Struct must have fields: length, id, data, packet_type",
        )
            .to_compile_error()
            .into();
    }

    let expanded = quote! {
        impl #name {
            pub fn new() -> Self {
                let packet_type = PacketType::#name;
                Self {
                    length: length_to_byte(16),
                    id: packet_type.as_byte(),
                    data: Vec::new(),
                    packet_type,
                }
            }
        }

        impl Packet for #name {
            fn as_length_byte(&self) -> &[u8] {
                &self.length
            }

            fn as_id_byte(&self) -> &[u8] {
                &self.id
            }

            fn as_data_byte(&self) -> &[u8] {
                &self.data
            }

            fn clone_length_byte(&self) -> Vec<u8> {
                self.length.clone()
            }

            fn clone_id_byte(&self) -> Vec<u8> {
                self.id.clone()
            }

            fn clone_data_byte(&self) -> Vec<u8> {
                self.data.clone()
            }

            fn data_to_string(&self) -> String {
                String::from_utf8_lossy(&self.data).to_string()
            }

            fn packet_type(&self) -> PacketType {
                self.packet_type
            }

            fn equal(&self, packet_type: PacketType) -> bool {
                self.packet_type == packet_type
            }
        }
    };

    TokenStream::from(expanded)
}
