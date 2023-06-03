#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]

mod gen {
    volo::include_service!("volo_gen.rs");
}

mod protos_gen {
    include!(concat!(env!("OUT_DIR"), "/protos/mod.rs"));
}

pub use gen::volo_gen::*;
pub use protos_gen::*;
