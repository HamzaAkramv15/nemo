// Перевод единиц в лаунчере: «5 km in miles», «72 f to c», «2 gb в mb», «3 ч в мин». Таблица зашита (длина,
// масса, температура, объём, время, данные, скорость), без сети. Ответ — число и единица, Enter копирует.

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Length,
    Mass,
    Volume,
    Time,
    Data,
    Speed,
    Temp,
}

/// (названия через пробел, вид, сколько в базовой единице вида; для температуры не используется)
const UNITS: &[(&str, Kind, f64)] = &[
    ("mm millimeter millimeters мм", Kind::Length, 0.001), // i18n-ok
    ("cm centimeter centimeters см", Kind::Length, 0.01), // i18n-ok
    ("m meter meters metre metres м", Kind::Length, 1.0), // i18n-ok
    ("km kilometer kilometers kilometre kilometres км", Kind::Length, 1000.0), // i18n-ok
    ("in inch inches дюйм дюйма дюймов", Kind::Length, 0.0254), // i18n-ok
    ("ft foot feet фут фута футов", Kind::Length, 0.3048), // i18n-ok
    ("yd yard yards ярд ярда ярдов", Kind::Length, 0.9144), // i18n-ok
    ("mi mile miles миля мили миль", Kind::Length, 1609.344), // i18n-ok
    ("mg milligram milligrams мг", Kind::Mass, 0.000001), // i18n-ok
    ("g gram grams gramme г гр", Kind::Mass, 0.001), // i18n-ok
    ("kg kilogram kilograms кг", Kind::Mass, 1.0), // i18n-ok
    ("t tonne tonnes т", Kind::Mass, 1000.0), // i18n-ok
    ("oz ounce ounces унция унций", Kind::Mass, 0.028349523125), // i18n-ok
    ("lb lbs pound pounds фунт фунта фунтов", Kind::Mass, 0.45359237), // i18n-ok
    ("ml milliliter milliliters мл", Kind::Volume, 0.001), // i18n-ok
    ("l liter liters litre litres л", Kind::Volume, 1.0), // i18n-ok
    ("tsp teaspoon teaspoons", Kind::Volume, 0.00492892159375),
    ("tbsp tablespoon tablespoons", Kind::Volume, 0.01478676478125),
    ("cup cups", Kind::Volume, 0.2365882365),
    ("pt pint pints", Kind::Volume, 0.473176473),
    ("gal gallon gallons галлон галлона галлонов", Kind::Volume, 3.785411784), // i18n-ok
    ("s sec second seconds с сек секунда секунд", Kind::Time, 1.0), // i18n-ok
    ("min minute minutes мин минута минут", Kind::Time, 60.0), // i18n-ok
    ("h hr hour hours ч час часа часов", Kind::Time, 3600.0), // i18n-ok
    ("d day days день дня дней", Kind::Time, 86400.0), // i18n-ok
    ("wk week weeks неделя недели недель", Kind::Time, 604800.0), // i18n-ok
    ("b byte bytes байт", Kind::Data, 1.0), // i18n-ok
    ("kb kilobyte kilobytes кб", Kind::Data, 1000.0), // i18n-ok
    ("mb megabyte megabytes мб", Kind::Data, 1_000_000.0), // i18n-ok
    ("gb gigabyte gigabytes гб", Kind::Data, 1_000_000_000.0), // i18n-ok
    ("tb terabyte terabytes тб", Kind::Data, 1_000_000_000_000.0), // i18n-ok
    ("kib", Kind::Data, 1024.0),
    ("mib", Kind::Data, 1_048_576.0),
    ("gib", Kind::Data, 1_073_741_824.0),
    ("tib", Kind::Data, 1_099_511_627_776.0),
    ("m/s mps", Kind::Speed, 1.0),
    ("km/h kmh kph км/ч", Kind::Speed, 1.0 / 3.6), // i18n-ok
    ("mph", Kind::Speed, 0.44704),
    ("kn knot knots узел узлов", Kind::Speed, 0.514444), // i18n-ok
    ("c °c celsius", Kind::Temp, 0.0),
    ("f °f fahrenheit", Kind::Temp, 0.0),
    ("k kelvin", Kind::Temp, 0.0),
];

fn find(name: &str) -> Option<(usize, Kind, f64)> {
    let n = name.trim().to_lowercase();
    UNITS.iter().enumerate().find(|(_, (names, ..))| names.split(' ').any(|x| x == n)).map(|(i, (_, k, f))| (i, *k, *f))
}

/// градус как шкала: c, f или k по первому названию
fn scale(i: usize) -> char {
    UNITS[i].0.chars().next().unwrap_or('c')
}

fn to_celsius(v: f64, s: char) -> f64 {
    match s {
        'f' => (v - 32.0) * 5.0 / 9.0,
        'k' => v - 273.15,
        _ => v,
    }
}

fn from_celsius(c: f64, s: char) -> f64 {
    match s {
        'f' => c * 9.0 / 5.0 + 32.0,
        'k' => c + 273.15,
        _ => c,
    }
}

/// «5 km in miles» → Some(«3.107 mi»); None — это не перевод единиц
pub fn convert(q: &str) -> Option<String> {
    let q = q.trim();
    // число (можно с запятой), единица, слово-связка, единица
    let split = q.find(|c: char| !(c.is_ascii_digit() || matches!(c, '.' | ',' | '-' | '+')))?;
    let (num, rest) = q.split_at(split);
    let v: f64 = num.replace(',', ".").parse().ok()?;
    let rest = rest.trim();
    let (from, to) = [" in ", " to ", " into ", " в ", " -> ", " → ", " as "].iter().find_map(|w| rest.split_once(w))?; // i18n-ok
    let (fi, fk, ff) = find(from)?;
    let (ti, tk, tf) = find(to)?;
    if fk != tk {
        return None;
    }
    let out = if fk == Kind::Temp { from_celsius(to_celsius(v, scale(fi)), scale(ti)) } else { v * ff / tf };
    Some(format!("{} {}", fmt(out), to.trim()))
}

/// число покороче: до 4 знаков после запятой, без лишних нулей, большие — без дроби
fn fmt(v: f64) -> String {
    if v.abs() >= 1e15 || (v != 0.0 && v.abs() < 1e-4) {
        return format!("{v:e}");
    }
    let s = format!("{v:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    if s == "-0" { "0".into() } else { s }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(q: &str) -> String {
        convert(q).unwrap_or_else(|| "none".into())
    }

    #[test]
    fn lengths_masses_volumes() {
        assert_eq!(c("5 km in miles"), "3.1069 miles");
        assert_eq!(c("1 mi to km"), "1.6093 km");
        assert_eq!(c("12 in to cm"), "30.48 cm");
        assert_eq!(c("1 lb in kg"), "0.4536 kg");
        assert_eq!(c("2 kg to lb"), "4.4092 lb");
        assert_eq!(c("1 gal to l"), "3.7854 l");
        assert_eq!(c("500 ml in cup"), "2.1134 cup");
    }

    #[test]
    fn temperatures_use_their_own_scales() {
        assert_eq!(c("72 f to c"), "22.2222 c");
        assert_eq!(c("0 c in f"), "32 f");
        assert_eq!(c("100 c to k"), "373.15 k");
        assert_eq!(c("-40 c to f"), "-40 f");
    }

    #[test]
    fn time_data_and_speed() {
        assert_eq!(c("2 h in min"), "120 min");
        assert_eq!(c("1.5 d to h"), "36 h");
        assert_eq!(c("1 gb to mb"), "1000 mb");
        assert_eq!(c("1 gib to mib"), "1024 mib");
        assert_eq!(c("100 km/h to mph"), "62.1371 mph");
    }

    #[test]
    fn russian_words_and_decimal_commas_work() {
        assert_eq!(c("3 ч в мин"), "180 мин"); // i18n-ok
        assert_eq!(c("2,5 кг в г"), "2500 г"); // i18n-ok
        assert_eq!(c("10 км в мили"), "6.2137 мили"); // i18n-ok
    }

    #[test]
    fn things_that_are_not_conversions_are_left_alone() {
        for q in ["2+2", "5 km", "5 km in kg", "hello in world", "km in miles", "", "5 apples to pears"] {
            assert_eq!(convert(q), None, "{q}");
        }
    }

    #[test]
    fn numbers_are_short_and_clean() {
        assert_eq!(fmt(3.0), "3");
        assert_eq!(fmt(0.5), "0.5");
        assert_eq!(fmt(1.0 / 3.0), "0.3333");
        assert_eq!(fmt(-0.00001), "-1e-5");
        assert_eq!(fmt(0.0), "0");
    }
}
