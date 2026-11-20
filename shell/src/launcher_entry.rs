// Значки на иконках дока: число (письма, сообщения) и полоса прогресса (загрузка) от самих программ. Программы шлют их
// по сессионной шине сигналом `com.canonical.Unity.LauncherEntry.Update(app_uri, свойства)`: так делают Telegram, Firefox
// (загрузки), Thunderbird, Steam и другие, а в Plasma и Unity это называется то же самое. Шелл только слушает.

use crate::app::Msg;
use calloop::channel::Sender;
use std::collections::HashMap;
use zbus::blocking::{Connection, MessageIterator};
use zbus::zvariant::OwnedValue;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Entry {
    pub count: i64,
    pub count_visible: bool,
    pub progress: f64,
    pub progress_visible: bool,
    pub urgent: bool,
}

/// «application://telegramdesktop.desktop» → «telegramdesktop» (в нижнем регистре: id в доке сравниваются без регистра)
pub fn key_of(uri: &str) -> Option<String> {
    let rest = uri.strip_prefix("application://")?;
    let r = rest.strip_suffix(".desktop").unwrap_or(rest).to_lowercase();
    (!r.is_empty() && !r.contains('/')).then_some(r)
}

/// свойства сигнала поверх прежнего состояния: что не прислали, то не меняется (так велит интерфейс)
pub fn apply(mut e: Entry, props: &HashMap<String, OwnedValue>) -> Entry {
    let int = |k: &str| props.get(k).and_then(|v| i64::try_from(v).ok());
    let flt = |k: &str| props.get(k).and_then(|v| f64::try_from(v).ok());
    let flag = |k: &str| props.get(k).and_then(|v| bool::try_from(v).ok());
    if let Some(v) = int("count") {
        e.count = v;
    }
    if let Some(v) = flag("count-visible") {
        e.count_visible = v;
    }
    if let Some(v) = flt("progress") {
        e.progress = v.clamp(0.0, 1.0);
    }
    if let Some(v) = flag("progress-visible") {
        e.progress_visible = v;
    }
    if let Some(v) = flag("urgent") {
        e.urgent = v;
    }
    e
}

/// число на значке: «7», «99+»; нуль и отрицательное — без значка
pub fn badge_text(n: i64) -> Option<String> {
    match n {
        i64::MIN..=0 => None,
        1..=99 => Some(n.to_string()),
        _ => Some("99+".into()),
    }
}

pub fn spawn(tx: Sender<Msg>) {
    let _ = std::thread::Builder::new().name("launcher-entry".into()).spawn(move || {
        let Ok(c) = Connection::session() else { return };
        let rule = zbus::MatchRule::builder().msg_type(zbus::message::Type::Signal).interface("com.canonical.Unity.LauncherEntry").and_then(|b| b.member("Update")).map(|b| b.build());
        let Ok(rule) = rule else { return };
        let Ok(it) = MessageIterator::for_match_rule(rule, &c, Some(64)) else { return };
        let mut state: HashMap<String, Entry> = HashMap::new();
        for m in it {
            let Ok(m) = m else { continue };
            let Ok((uri, props)) = m.body().deserialize::<(String, HashMap<String, OwnedValue>)>() else { continue };
            let Some(key) = key_of(&uri) else { continue };
            let e = apply(state.get(&key).cloned().unwrap_or_default(), &props);
            state.insert(key.clone(), e.clone());
            let _ = tx.send(Msg::LauncherEntry(key, e));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use zbus::zvariant::Value;

    fn props(v: &[(&str, Value)]) -> HashMap<String, OwnedValue> {
        v.iter().map(|(k, x)| (k.to_string(), OwnedValue::try_from(x.clone()).unwrap())).collect()
    }

    #[test]
    fn the_key_comes_from_the_desktop_uri() {
        assert_eq!(key_of("application://org.telegram.desktop.desktop"), Some("org.telegram.desktop".into()));
        assert_eq!(key_of("application://Firefox.desktop"), Some("firefox".into()));
        assert_eq!(key_of("file:///x"), None);
        assert_eq!(key_of("application://"), None);
        assert_eq!(key_of("application://a/b.desktop"), None);
    }

    #[test]
    fn a_signal_changes_only_what_it_carries() {
        let e = apply(Entry::default(), &props(&[("count", Value::I64(5)), ("count-visible", Value::Bool(true))]));
        assert_eq!((e.count, e.count_visible, e.progress_visible), (5, true, false));
        let e = apply(e, &props(&[("progress", Value::F64(0.4)), ("progress-visible", Value::Bool(true))]));
        assert_eq!((e.count, e.progress, e.progress_visible), (5, 0.4, true), "the count stays");
        let e = apply(e, &props(&[("progress", Value::F64(7.0))]));
        assert_eq!(e.progress, 1.0, "clamped");
        let e = apply(e, &props(&[("count-visible", Value::Bool(false))]));
        assert!(!e.count_visible && e.count == 5);
    }

    #[test]
    fn the_badge_reads_as_a_short_number() {
        assert_eq!(badge_text(0), None);
        assert_eq!(badge_text(-3), None);
        assert_eq!(badge_text(7), Some("7".into()));
        assert_eq!(badge_text(99), Some("99".into()));
        assert_eq!(badge_text(100), Some("99+".into()));
    }
}
