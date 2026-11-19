// Дата по хиджре — табличный (арифметический) календарь, а не наблюдаемый: может отличаться от
// объявленной в стране на день. Поэтому показывается только по настройке `hijri` и как справка.

/// 0-я, 1-я … 11-я: названия месяцев по-английски (латиницей) и по-арабски
const EN: [&str; 12] = ["Muharram", "Safar", "Rabi' al-awwal", "Rabi' al-thani", "Jumada al-awwal", "Jumada al-thani", "Rajab", "Sha'ban", "Ramadan", "Shawwal", "Dhu al-Qi'dah", "Dhu al-Hijjah"];
const AR: [&str; 12] = ["محرم", "صفر", "ربيع الأول", "ربيع الآخر", "جمادى الأولى", "جمادى الآخرة", "رجب", "شعبان", "رمضان", "شوال", "ذو القعدة", "ذو الحجة"];

/// число дней от 1970-01-01 → (год, месяц 1..12, день) по хиджре
pub fn from_days(days: i64) -> (i64, u32, u32) {
    let jdn = days + 2_440_588;
    let year = (30 * (jdn - 1_948_440) + 10_646).div_euclid(10_631);
    let first = |y: i64, m: i64| (29.5 * (m - 1) as f64).ceil() as i64 + (y - 1) * 354 + (3 + 11 * y).div_euclid(30) + 1_948_440;
    let month = (((jdn - first(year, 1)) as f64 / 29.5).floor() as i64 + 1).clamp(1, 12);
    let day = jdn - first(year, month) + 1;
    (year, month as u32, day.max(1) as u32)
}

/// «9 جمادى الأولى 1448» / «9 Jumada al-awwal 1448 AH»
pub fn text(days: i64, arabic: bool) -> String {
    let (y, m, d) = from_days(days);
    let i = (m - 1) as usize;
    if arabic {
        format!("{d} {} {y}", AR[i])
    } else {
        format!("{d} {} {y} AH", EN[i])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn days(y: i32, m: u32, d: u32) -> i64 {
        crate::widgets::days_from_civil(y, m, d)
    }

    #[test]
    fn known_dates_of_the_tabular_calendar() {
        // 16 июля 622 (юлианский) — 1 мухаррама 1 года: 622-07-19 по григорианскому, далеко до 1970 — проверяем по берегам
        assert_eq!(from_days(days(2000, 1, 1)), (1420, 9, 24), "24 Ramadan 1420");
        assert_eq!(from_days(days(2024, 3, 11)).1, 9, "March 2024 is Ramadan 1445");
        assert_eq!(from_days(days(2024, 3, 11)).0, 1445);
        assert_eq!(from_days(days(2026, 11, 19)).0, 1448);
    }

    #[test]
    fn every_day_has_a_valid_month_and_day() {
        let start = days(2020, 1, 1);
        let mut last = (0, 0, 0);
        for i in 0..4000 {
            let (y, m, d) = from_days(start + i);
            assert!((1..=12).contains(&m) && (1..=30).contains(&d), "{y} {m} {d}");
            // подряд: день растёт на 1 или начинается новый месяц с 1
            if i > 0 {
                assert!((y, m, d) > last, "{last:?} -> {:?}", (y, m, d));
                assert!(d == last.2 + 1 || d == 1, "{last:?} -> {:?}", (y, m, d));
            }
            last = (y, m, d);
        }
    }

    #[test]
    fn text_in_both_scripts() {
        let n = days(2000, 1, 1);
        assert_eq!(text(n, false), "24 Ramadan 1420 AH");
        assert_eq!(text(n, true), "24 رمضان 1420");
    }
}
