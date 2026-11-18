// Ночной свет одним местом: аргументы wlsunset для шелла (night.rs) и экрана входа (yami-greet).
// Значения приходят и из чужого файла (look.json читает экран входа, где видны пароли всех), поэтому
// всё зажато в пределы: температура 2500–6000 K, часы 0–23, координаты в своих границах.

/// аргументы wlsunset. `always` — без расписания: wlsunset не принимает равные температуры, а
/// разница в 1 K глазу не видна; иначе — по солнцу, если известно место, или по часам
pub fn args(temp: i32, always: bool, place: Option<(f64, f64)>, on_h: i32, off_h: i32) -> Vec<String> {
    let temp = temp.clamp(2500, 6000);
    if always {
        return vec!["-t".into(), temp.to_string(), "-T".into(), (temp + 1).to_string()];
    }
    // ночная температура всегда ниже дневной (6500): иначе wlsunset не запустится
    let mut a = vec!["-t".to_string(), temp.to_string(), "-T".into(), "6500".into()];
    match place.filter(|(la, lo)| (-90.0..=90.0).contains(la) && (-180.0..=180.0).contains(lo)) {
        Some((lat, lon)) => a.extend(["-l".into(), format!("{lat}"), "-L".into(), format!("{lon}")]),
        None => {
            let (on, off) = (on_h.clamp(0, 23), off_h.clamp(0, 23));
            // одинаковые часы — ночь в один час: включить на час позже, чем выключить
            let on = if on == off { (off + 1) % 24 } else { on };
            let h = |x: i32| format!("{x:02}:00");
            a.extend(["-S".into(), h(off), "-s".into(), h(on)]);
        }
    }
    a
}

#[cfg(test)]
mod tests {
    use super::args;

    #[test]
    fn by_the_clock_by_the_sun_and_all_day() {
        assert_eq!(args(4000, false, None, 20, 7), ["-t", "4000", "-T", "6500", "-S", "07:00", "-s", "20:00"]);
        assert_eq!(args(4000, false, None, 7, 7)[4..], ["-S", "07:00", "-s", "08:00"]);
        assert_eq!(args(4000, false, Some((30.06, 31.25)), 20, 7), ["-t", "4000", "-T", "6500", "-l", "30.06", "-L", "31.25"]);
        assert_eq!(args(3500, true, Some((30.0, 31.0)), 20, 7), ["-t", "3500", "-T", "3501"]);
    }

    /// чужой файл не выведет значения за границы
    #[test]
    fn untrusted_values_are_clamped() {
        assert_eq!(args(99999, true, None, 0, 0), ["-t", "6000", "-T", "6001"]);
        assert_eq!(args(-5, false, None, 99, -4)[..2], ["-t", "2500"]);
        // место вне границ — как будто его нет: по часам
        assert_eq!(args(4000, false, Some((95.0, 0.0)), 20, 7)[4..], ["-S", "07:00", "-s", "20:00"]);
        assert_eq!(args(4000, false, Some((f64::NAN, 0.0)), 20, 7)[4], "-S");
    }
}
