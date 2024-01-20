#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]

#[allow(deprecated)] // volo::include_service!改成volo::include!和include!都报错，暂时压制警告
mod gen {
    volo::include_service!("volo_gen.rs");
}

mod protos_gen {
    include!(concat!(env!("OUT_DIR"), "/protos/mod.rs"));
}

pub use gen::volo_gen::*;
pub use protos_gen::*;
