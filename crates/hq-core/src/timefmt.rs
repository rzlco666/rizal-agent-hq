//! Format waktu WIB (UTC+7) tanpa dependensi tanggal eksternal.

/// epoch ms -> (tahun, bulan, hari, jam, menit, detik) di WIB.
pub fn wib_parts(epoch_ms: i64) -> (i64, u32, u32, u32, u32, u32) {
    let secs = epoch_ms.div_euclid(1000) + 7 * 3600;
    let days = secs.div_euclid(86400);
    let tod = secs.rem_euclid(86400);
    let (h, m, s) = (
        (tod / 3600) as u32,
        ((tod % 3600) / 60) as u32,
        (tod % 60) as u32,
    );
    // Algoritma civil-from-days (Howard Hinnant).
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if mo <= 2 { y + 1 } else { y }, mo, d, h, m, s)
}

pub fn wib_hms(epoch_ms: i64) -> String {
    let (_, _, _, h, m, s) = wib_parts(epoch_ms);
    format!("{h:02}:{m:02}:{s:02}")
}

pub fn wib_date(epoch_ms: i64) -> String {
    let (y, mo, d, _, _, _) = wib_parts(epoch_ms);
    format!("{y:04}-{mo:02}-{d:02}")
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// "baru saja", "5 menit lalu", "2 jam lalu", "kemarin", "3 hari lalu".
pub fn relative_id(epoch_ms: i64, now: i64) -> String {
    let diff = (now - epoch_ms).max(0) / 1000;
    if diff < 45 {
        "baru saja".into()
    } else if diff < 3600 {
        format!("{} menit lalu", (diff / 60).max(1))
    } else if diff < 86400 {
        format!("{} jam lalu", diff / 3600)
    } else if diff < 172800 {
        "kemarin".into()
    } else {
        format!("{} hari lalu", diff / 86400)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tanggal_wib_benar() {
        // Terverifikasi via `date`: 1793607060 = 2026-11-02 08:11 UTC = 15:11 WIB
        let ms = 1793607060000;
        assert_eq!(wib_date(ms), "2026-11-02");
        assert_eq!(wib_hms(ms), "15:11:00");
    }
}
