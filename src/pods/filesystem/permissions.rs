// Inspired by [libc::S_IEXEC]
const EXECUTE_BIT_FLAG: u16 = 0b001 << 6;
// Inspired by [libc::S_IWRITE]
const WRITE_BIT_FLAG: u16 = 0b010 << 6;
// Inspired by [libc::S_IREAD]
const READ_BIT_FLAG: u16 = 0b100 << 6;

pub fn has_execute_perm(perm: u16) -> bool {
    return (perm & EXECUTE_BIT_FLAG) != 0;
}

pub fn has_write_perm(perm: u16) -> bool {
    return (perm & WRITE_BIT_FLAG) != 0;
}

pub fn has_read_perm(perm: u16) -> bool {
    return (perm & READ_BIT_FLAG) != 0;
}

pub mod test {
    #[allow(unused)]
    use crate::pods::arbo::WINDOWS_DEFAULT_PERMS_MODE;
    #[allow(unused)]
    use crate::pods::filesystem::permissions::{
        has_execute_perm, has_read_perm, has_write_perm, EXECUTE_BIT_FLAG, READ_BIT_FLAG,
        WRITE_BIT_FLAG,
    };

    #[test]
    fn test_r_bit() {
        assert!(
            has_read_perm(WINDOWS_DEFAULT_PERMS_MODE),
            "WINDOWS_DEFAULT_PERMS_MODE is read"
        );
        assert!(has_read_perm(READ_BIT_FLAG), "READ_BIT_FLAG is correct");
        assert!(has_read_perm(0o400), "has_read_perm is correct");
        assert!(has_read_perm(0o777), "has_read_perm is correct");
    }

    #[test]
    fn test_w_bit() {
        assert!(
            has_write_perm(WINDOWS_DEFAULT_PERMS_MODE),
            "WINDOWS_DEFAULT_PERMS_MODE is write"
        );
        assert!(has_write_perm(WRITE_BIT_FLAG), "WRITE_BIT_FLAG is correct");
        assert!(has_write_perm(0o200), "has_write_perm is correct");
        assert!(has_write_perm(0o777), "has_write_perm is correct");
    }

    #[test]
    fn test_x_bit() {
        assert!(
            !has_execute_perm(WINDOWS_DEFAULT_PERMS_MODE),
            "WINDOWS_DEFAULT_PERMS_MODE is not exec"
        );
        assert!(
            has_execute_perm(WRITE_BIT_FLAG),
            "WRITE_BIT_FLAG is correct"
        );
        assert!(has_execute_perm(0o100), "has_execute_perm is correct");
        assert!(has_execute_perm(0o777), "has_execute_perm is correct");
    }
}
