//! `datetime.date.today().isoformat()`: the local calendar date.

#[cfg(unix)]
/// Read the local calendar date without spawning a command or changing timezone state.
pub fn today() -> String {
    // SAFETY: localtime_r writes only into `tm`, which we own.
    unsafe {
        let t = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();

        if libc::localtime_r(&t, &mut tm).is_null() {
            panic!("localtime_r failed");
        }
        format!(
            "{:04}-{:02}-{:02}",
            tm.tm_year + 1900,
            tm.tm_mon + 1,
            tm.tm_mday
        )
    }
}

#[cfg(windows)]
/// Obtain Windows local calendar fields through the stable kernel32 ABI.
pub fn today() -> String {
    /// Windows SYSTEMTIME layout; repr(C) is required by GetLocalTime.
    #[repr(C)]
    struct SystemTime {
        year: u16,
        month: u16,
        day_of_week: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
        milliseconds: u16,
    }
    #[link(name = "kernel32")]
    extern "system" {
        /// Fills the supplied SYSTEMTIME and has no failure return value.
        fn GetLocalTime(st: *mut SystemTime);
    }

    let mut st = SystemTime {
        year: 0,
        month: 0,
        day_of_week: 0,
        day: 0,
        hour: 0,
        minute: 0,
        second: 0,
        milliseconds: 0,
    };
    // SAFETY: GetLocalTime fills the struct and cannot fail.
    unsafe { GetLocalTime(&mut st) };
    format!("{:04}-{:02}-{:02}", st.year, st.month, st.day)
}

/// Whether `YYYY-MM-DD` (already shape-checked) names a real date, as
/// `datetime.strptime(date, "%Y-%m-%d")` decides: years 1 to 9999.
pub fn is_real(date: &str) -> bool {
    let n = |r: std::ops::Range<usize>| date[r].parse::<u32>().unwrap_or(0);
    let (y, m, d) = (n(0..4), n(5..7), n(8..10));
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let days = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    y >= 1 && (1..=days).contains(&d)
}

/// `re.fullmatch(r"\d{4}-\d{2}-\d{2}", s)`, ASCII digits only.
pub fn is_shaped(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b.iter().enumerate().all(|(i, &c)| match i {
            4 | 7 => c == b'-',
            _ => c.is_ascii_digit(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert!(is_shaped("2026-10-21") && is_real("2026-10-21"));
        assert!(is_real("2024-02-29") && !is_real("2023-02-29"));
        assert!(!is_real("2000-13-01") && !is_real("0000-01-01"));
        assert!(is_real("2000-02-29") && !is_real("1900-02-29"));
        assert!(!is_shaped("2026-1-021") && !is_shaped("2026-10-2"));
        assert_eq!(today().len(), 10);
    }
}
