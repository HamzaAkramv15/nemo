// Где человек: без GeoClue и без IP-адреса. Часовой пояс системы (/etc/localtime → Africa/Cairo) уже
// называет город, а геокодер Open-Meteo по имени города отвечает несколькими местами (Cairo в Египте
// и Cairo в Иллинойсе): берётся то, чьё время совпадает с часовым поясом системы. Наружу уходит одно
// слово — имя города, то же, что и при вводе его руками. Ответ лежит в ~/.cache/yami/location.json
// и обновляется, когда сменился часовой пояс или прошёл месяц. Для погоды (`weatherCity` = auto) и
// ночного света по солнцу (`nightGeo` = auto).
// i18n: inline pairs

use serde_json::Value;

const MONTH: u64 = 30 * 24 * 3600;

/// идентификатор часового пояса системы: «Africa/Cairo»
pub fn tz_id() -> Option<String> {
    let link = std::fs::read_link("/etc/localtime").ok()?;
    let s = link.to_string_lossy();
    let id = s.split("zoneinfo/").nth(1)?;
    (!id.is_empty() && id != "UTC" && id != "Etc/UTC").then(|| id.to_string())
}

/// «America/Argentina/Buenos_Aires» → «Buenos Aires»; зоны без города (Etc/…) — None
pub fn city_of(tz: &str) -> Option<String> {
    if tz.starts_with("Etc/") {
        return None;
    }
    let c = tz.rsplit('/').next()?.replace('_', " ");
    (!c.is_empty() && c.chars().any(|ch| ch.is_alphabetic())).then_some(c)
}

/// из ответа геокодера — место с часовым поясом системы, иначе первое
pub fn pick<'a>(results: &'a Value, tz: &str) -> Option<&'a Value> {
    let list = results.as_array()?;
    list.iter().find(|r| r["timezone"].as_str() == Some(tz)).or_else(|| list.first())
}

fn cache_path() -> std::path::PathBuf {
    yami_px::paths::cache_dir().join("yami/location.json")
}

/// сохранённое место, если оно для нынешнего часового пояса и не старше месяца
pub fn cached() -> Option<(f64, f64, String)> {
    let tz = tz_id()?;
    let v: Value = serde_json::from_str(&std::fs::read_to_string(cache_path()).ok()?).ok()?;
    let fresh = crate::clock::unix_now().saturating_sub(v["at"].as_u64()?) < MONTH;
    (v["tz"].as_str() == Some(&tz) && fresh).then(|| (v["lat"].as_f64().unwrap_or(0.0), v["lon"].as_f64().unwrap_or(0.0), v["name"].as_str().unwrap_or("").to_string()))
}

/// место по часовому поясу системы: из кеша или спросить геокодер
pub fn resolve() -> Result<(f64, f64, String), String> {
    if let Some(p) = cached() {
        return Ok(p);
    }
    let tz = tz_id().ok_or_else(|| crate::tf!("часовой пояс системы не называет город (timedatectl set-timezone Страна/Город)", "the system time zone names no city (timedatectl set-timezone Country/City)", "o fuso horário do sistema não indica uma cidade (timedatectl set-timezone País/Cidade)"))?;
    let city = city_of(&tz).ok_or_else(|| crate::tf!("часовой пояс {} не называет город", "the time zone {} names no city", "o fuso horário {} não indica uma cidade", tz))?;
    let lang = if crate::i18n::pt_br() { "pt" } else if crate::i18n::en() { "en" } else { "ru" };
    let v = crate::weather::get("https://geocoding-api.open-meteo.com/v1/search", &[("name", city.clone()), ("count", "10".into()), ("language", lang.into()), ("format", "json".into())])?;
    let r = pick(&v["results"], &tz).ok_or_else(|| crate::tf!("город не найден: {}", "city not found: {}", "cidade não encontrada: {}", city))?;
    let (Some(la), Some(lo)) = (r["latitude"].as_f64(), r["longitude"].as_f64()) else {
        return Err(crate::tf!("город не найден: {}", "city not found: {}", "cidade não encontrada: {}", city));
    };
    let name: String = r["name"].as_str().unwrap_or(&city).chars().take(40).collect();
    let body = serde_json::json!({ "tz": tz, "lat": la, "lon": lo, "name": name, "at": crate::clock::unix_now() });
    let _ = crate::config::write_atomic(&cache_path(), &(body.to_string() + "\n"));
    Ok((la, lo, name))
}

/// «auto» в настройке места
pub fn is_auto(s: &str) -> bool {
    s.trim().eq_ignore_ascii_case("auto")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn the_time_zone_names_the_city() {
        assert_eq!(super::city_of("Africa/Cairo").as_deref(), Some("Cairo"));
        assert_eq!(super::city_of("America/Argentina/Buenos_Aires").as_deref(), Some("Buenos Aires"));
        assert_eq!(super::city_of("Europe/Berlin").as_deref(), Some("Berlin"));
        assert_eq!(super::city_of("Etc/GMT+3"), None);
    }

    /// Cairo в Египте, а не первый в списке Cairo из Иллинойса
    #[test]
    fn the_place_in_the_system_time_zone_wins() {
        let r = json!([
            { "name": "Cairo", "country": "United States", "timezone": "America/Chicago", "latitude": 37.0, "longitude": -89.2 },
            { "name": "Cairo", "country": "Egypt", "timezone": "Africa/Cairo", "latitude": 30.06, "longitude": 31.25 },
        ]);
        assert_eq!(super::pick(&r, "Africa/Cairo").unwrap()["country"], "Egypt");
        // часового пояса среди ответов нет — первое, как раньше
        assert_eq!(super::pick(&r, "Asia/Tokyo").unwrap()["country"], "United States");
        assert!(super::pick(&json!([]), "Africa/Cairo").is_none());
    }

    #[test]
    fn auto_is_spelled_auto() {
        assert!(super::is_auto(" Auto "));
        assert!(!super::is_auto("Cairo"));
    }

    /// с сетью и своим часовым поясом: `cargo test -p yami-shell live_location -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn live_location() {
        let tz = super::tz_id();
        let p = super::resolve();
        println!("{tz:?} -> {p:?}");
        assert!(p.is_ok());
    }
}
