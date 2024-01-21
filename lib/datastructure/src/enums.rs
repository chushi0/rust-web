/// 二值枚举
///
/// 该trait表示枚举只有两个可能取值。
pub trait TwoValueEnum {
    /// 返回二值枚举的另一个枚举
    fn opposite(&self) -> Self;
}
