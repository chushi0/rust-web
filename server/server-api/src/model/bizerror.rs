#[repr(i32)]
pub enum BizError {
    InternalError = 20000,
    InvalidUsernameOrPassword = 20001,
    DuplicateUsername = 20002,
}
