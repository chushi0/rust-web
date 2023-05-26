#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]

mod gen {
    volo::include_service!("volo_gen.rs");
}

pub use gen::volo_gen::*;
