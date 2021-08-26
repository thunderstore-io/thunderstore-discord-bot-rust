use std::intrinsics::transmute;

pub fn u64_to_i64(i: u64) -> i64 {
    unsafe { transmute(i) }
}
