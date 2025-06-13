// Inspired by [libc::S_IEXEC]
const EXECUTE_BIT_FLAG: u16 = 0b001000000;
// Inspired by [libc::S_IWRITE]
const WRITE_BIT_FLAG: u16 = 0b010000000;
// Inspired by [libc::S_IREAD]
const READ_BIT_FLAG: u16 = 0b100000000;

pub fn has_execute_perm(perm: u16) -> bool {
    return (perm & EXECUTE_BIT_FLAG) != 0;
}

pub fn has_write_perm(perm: u16) -> bool {
    return (perm & WRITE_BIT_FLAG) != 0;
}

pub fn has_read_perm(perm: u16) -> bool {
    return (perm & READ_BIT_FLAG) != 0;
}
