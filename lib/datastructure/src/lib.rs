/// 包含使用数组结构封装的各种结构体
mod array;
/// 包含与枚举相关的结构体
mod enums;
/// 包含可以暂时规避所有权问题的结构体
mod handle;

pub use array::*;
pub use enums::*;
pub use handle::*;

pub mod macros {
    pub use datastructure_macro_derive::TwoValue;
}
