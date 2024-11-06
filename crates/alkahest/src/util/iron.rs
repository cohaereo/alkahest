#[repr(u32)]
#[allow(unused)]
pub enum Policy {
    Disabled,
    Enforce,
}

extern "C" {
    fn iron_set_policy(p: Policy);
    fn iron_get_content_policy() -> bool;
}

pub fn set_policy(policy: Policy) {
    unsafe {
        iron_set_policy(policy);
    }
}

pub fn get_content_policy() -> bool {
    unsafe { iron_get_content_policy() }
}
