extern crate proc_macro;
use quote::quote;
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

pub mod base_packet;

pub trait Packet: Send {
    fn as_length_byte(&self) -> &[u8];
    fn as_id_byte(&self) -> &[u8];
    fn as_data_byte(&self) -> &[u8];
    fn clone_length_byte(&self) -> Vec<u8>;
    fn clone_id_byte(&self) -> Vec<u8>;
    fn clone_data_byte(&self) -> Vec<u8>;
    fn data_to_string(&self) -> String;
    fn packet_type(&self) -> PacketType;
    fn equal(&self, packet_type: PacketType) -> bool;
}

#[derive(Eq, PartialEq, Clone, Copy)]
pub enum PacketType {
    BasePacket,
    AgentInformationPacket,
    AlivePacket,
    AliveReplyPacket,
    ConfirmPacket,
    ControlStatePacket,
    DataChannelPortPacket,
    FileBodyPacket,
    FileHeaderPacket,
    FileHeaderReplyPacket,
    FileTransferReplyPacket,
    PerformancePacket,
    ResultPacket,
    StillProcessPacket,
    StillProcessReplyPacket,
    TaskInfoPacket,
    TaskInfoReplyPacket,
}

#[proc_macro_derive(Packet)]
pub fn packet_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let expanded = quote! {
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
                String::from_utf8_lossy(&*self.data.clone()).to_string()
            }
            fn packet_type(&self) -> PacketType {
                self.packet_type
            }
            fn equal(&self, packet_type: PacketType) -> bool {
                self.packet_type.eq(&packet_type)
            }
        }
    };
    TokenStream::from(expanded)
}


impl PacketType {
    pub fn as_byte(&self) -> Vec<u8> {
        let id: usize = *self as usize;
        id.to_be_bytes().to_vec()
    }

    pub fn parse_packet_type(byte: &Vec<u8>) -> PacketType {
        let mut byte_array = [0_u8; 8];
        byte_array.copy_from_slice(&byte);
        let id = usize::from_be_bytes(byte_array);
        match id {
            1 => PacketType::AgentInformationPacket,
            2 => PacketType::AlivePacket,
            3 => PacketType::AliveReplyPacket,
            4 => PacketType::ConfirmPacket,
            5 => PacketType::ControlStatePacket,
            6 => PacketType::DataChannelPortPacket,
            7 => PacketType::FileBodyPacket,
            8 => PacketType::FileHeaderPacket,
            9 => PacketType::FileTransferReplyPacket,
            10 => PacketType::PerformancePacket,
            11 => PacketType::ResultPacket,
            12 => PacketType::StillProcessPacket,
            13 => PacketType::StillProcessReplyPacket,
            14 => PacketType::TaskInfoPacket,
            15 => PacketType::TaskInfoReplyPacket,
            _ => PacketType::BasePacket,
        }
    }
}

pub fn length_to_byte(length: usize) -> Vec<u8> {
    length.to_be_bytes().to_vec()
}
