use super::result;

pub fn platform_getrandom(buffer: &mut [u8]) -> result::Result {
    result::Result::Success
}
