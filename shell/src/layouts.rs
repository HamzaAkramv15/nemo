// Раскладки виджетов: набор виджетов с положением в долях монитора, видом подложек и подписью. Четыре поставляются внутри программы
// (config/layouts/*.json), свои лежат в ~/.config/nemo/layouts/*.json и сохраняются кнопкой «сохранить текущую». Применение раскладки ничего не
// удаляет: прежние виджеты выключаются, а перед этим весь набор и вид запоминаются, чтобы вернуть кнопкой «вернуть как было».

use crate::config::{Settings, Widget};
use std::path::PathBuf;
use std::sync::Mutex;

/// поставляемые раскладки: (id, содержимое)
const BUILTIN: &[(&str, &str)] = &[
    ("aurora", include_str!("../../config/layouts/aurora.json")),
    ("reef", include_str!("../../config/layouts/reef.json")),
    ("minimal", include_str!("../../config/layouts/minimal.json")),
    ("focus", include_str!("../../config/layouts/focus.json")),
];

#[derive(Clone, Debug)]
pub struct Layout {
    pub id: String,
    pub name: String,
    pub about: String,
    pub about_ru: String,
    /// отделка (widgetpanel::LOOKS)
    pub look: String,
    /// стиль подложек (widgetpanel::STYLES)
    pub panel: String,
    /// виджеты как JSON-объекты: поля `Widget` и положение в долях монитора fx, fy
    pub widgets: Vec<serde_json::Value>,
    /// лежит в папке пользователя (можно удалить)
    pub user: bool,
}

impl Layout {
    /// подпись на языке интерфейса: русская, если она есть и язык русский
    pub fn about_text(&self) -> &str {
        if crate::i18n::code() == "ru" && !self.about_ru.is_empty() {
            &self.about_ru
        } else {
            &self.about
        }
    }
}

pub fn parse(id: &str, text: &str, user: bool) -> Option<Layout> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    let widgets = v.get("widgets")?.as_array()?.clone();
    if widgets.is_empty() || widgets.iter().any(|w| w.get("kind").and_then(|k| k.as_str()).is_none_or(|k| crate::widgetkind::get(k).is_none())) {
        return None;
    }
    let name = s("name");
    Some(Layout {
        id: id.to_string(),
        name: if name.is_empty() { id.to_string() } else { name },
        about: s("about"),
        about_ru: s("about_ru"),
        look: if crate::widgetpanel::valid_look(&s("look")) { s("look") } else { "plain".into() },
        panel: if crate::widgetpanel::valid_style(&s("panel")) { s("panel") } else { "glass".into() },
        widgets,
        user,
    })
}

pub fn user_dir() -> PathBuf {
    nemo_px::paths::config_dir().join("layouts")
}

fn load_all() -> Vec<Layout> {
    let mut v: Vec<Layout> = BUILTIN.iter().filter_map(|(id, t)| parse(id, t, false)).collect();
    if let Ok(rd) = std::fs::read_dir(user_dir()) {
        let mut files: Vec<_> = rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "json")).collect();
        files.sort();
        for p in files {
            let id = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            if let Some(l) = std::fs::read_to_string(&p).ok().and_then(|t| parse(&id, &t, true)) {
                // свой файл с именем поставляемой раскладки заменяет её
                v.retain(|o| o.id != l.id);
                v.push(l);
            }
        }
    }
    v
}

static CACHE: Mutex<Option<Vec<Layout>>> = Mutex::new(None);

/// все раскладки (читаются один раз; `invalidate` после сохранения своей)
pub fn all() -> Vec<Layout> {
    let mut c = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    c.get_or_insert_with(load_all).clone()
}

pub fn invalidate() {
    *CACHE.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

/// виджеты раскладки с положением в пикселях монитора ow×oh; ids начинаются с `first_id`
pub fn build(l: &Layout, ow: i32, oh: i32, first_id: u32) -> Vec<Widget> {
    l.widgets
        .iter()
        .enumerate()
        .map(|(i, spec)| {
            let kind = spec.get("kind").and_then(|k| k.as_str()).unwrap_or("clock");
            let mut w = Widget::new(first_id + i as u32, kind);
            let s = |k: &str| spec.get(k).and_then(|x| x.as_str()).map(str::to_string);
            let b = |k: &str| spec.get(k).and_then(|x| x.as_bool());
            let f = |k: &str| spec.get(k).and_then(|x| x.as_f64()).unwrap_or(0.0).clamp(0.0, 1.0);
            if let Some(v) = s("size").filter(|v| ["s", "m", "l"].contains(&v.as_str())) {
                w.size = v;
            }
            for (key, slot) in [("source", &mut w.source), ("style", &mut w.style), ("anim", &mut w.anim), ("color", &mut w.color), ("text", &mut w.text)] {
                if let Some(v) = s(key) {
                    *slot = v;
                }
            }
            for (key, slot) in [("frame", &mut w.frame), ("label", &mut w.label), ("seconds", &mut w.seconds), ("date", &mut w.date)] {
                if let Some(v) = b(key) {
                    *slot = v;
                }
            }
            w.on = true;
            w.layout = l.id.clone();
            w.fx = f("fx") as f32;
            w.fy = f("fy") as f32;
            w.x = (ow as f64 * f("fx")) as i32;
            w.y = (oh as f64 * f("fy")) as i32;
            w
        })
        .collect()
}

/// текущие включённые виджеты как раскладка (в долях монитора ow×oh), в JSON для файла пользователя
pub fn snapshot_json(name: &str, s: &Settings, ow: i32, oh: i32) -> String {
    let defaults = Widget::default();
    let widgets: Vec<serde_json::Value> = s
        .widgets
        .iter()
        .filter(|w| w.on)
        .map(|w| {
            let mut m = serde_json::Map::new();
            m.insert("kind".into(), w.kind.clone().into());
            m.insert("size".into(), w.size.clone().into());
            let frac = |v: i32, total: i32| ((v.max(0) as f64 / total.max(1) as f64) * 1000.0).round() / 1000.0;
            m.insert("fx".into(), frac(w.x, ow).into());
            m.insert("fy".into(), frac(w.y, oh).into());
            for (key, val, def) in [("source", &w.source, &defaults.source), ("style", &w.style, &defaults.style), ("anim", &w.anim, &defaults.anim), ("color", &w.color, &defaults.color)] {
                if val != def {
                    m.insert(key.into(), val.clone().into());
                }
            }
            for (key, val) in [("frame", w.frame), ("label", w.label), ("seconds", w.seconds), ("date", w.date)] {
                m.insert(key.into(), val.into());
            }
            serde_json::Value::Object(m)
        })
        .collect();
    serde_json::to_string_pretty(&serde_json::json!({"name": name, "look": s.widget_look, "panel": s.widget_panel, "widgets": widgets})).unwrap_or_default() + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shipped_layout_parses_with_known_kinds_and_a_picture_of_places() {
        let all: Vec<Layout> = BUILTIN.iter().filter_map(|(id, t)| parse(id, t, false)).collect();
        assert_eq!(all.len(), BUILTIN.len(), "a shipped layout failed to parse");
        for l in &all {
            assert!(!l.name.is_empty() && !l.about.is_empty() && !l.about_ru.is_empty(), "{} needs a name and both descriptions", l.id);
            let ws = build(l, 1920, 1080, 10);
            assert_eq!(ws.len(), l.widgets.len());
            for w in &ws {
                assert!(w.on && w.layout == l.id);
                assert!((0..1920).contains(&w.x) && (0..1080).contains(&w.y), "{}: {} at {},{}", l.id, w.kind, w.x, w.y);
            }
        }
    }

    #[test]
    fn the_backup_format_round_trips_widgets_with_their_layout_marker() {
        // apply_layout writes {"widgets": [...]} with serde and restore_layout reads it back the same way
        let mut s = Settings::default();
        let mut w = Widget::new(7, "sky");
        w.layout = "aurora".into();
        w.x = 100;
        s.widgets.push(w.clone());
        let json = serde_json::json!({"widgets": s.widgets, "look": s.widget_look});
        let back: Vec<Widget> = serde_json::from_value(json.get("widgets").unwrap().clone()).unwrap();
        assert_eq!(back.last().unwrap(), &w);
        assert_eq!(back.last().unwrap().layout, "aurora");
    }

    #[test]
    fn positions_follow_the_monitor_size() {
        let l = parse("t", r#"{"widgets":[{"kind":"clock","fx":0.5,"fy":0.25}]}"#, false).unwrap();
        let a = build(&l, 1920, 1080, 1);
        let b = build(&l, 1280, 720, 1);
        assert_eq!((a[0].x, a[0].y), (960, 270));
        assert_eq!((b[0].x, b[0].y), (640, 180));
    }

    #[test]
    fn a_layout_with_an_unknown_kind_or_no_widgets_is_refused() {
        assert!(parse("x", r#"{"widgets":[{"kind":"nope"}]}"#, false).is_none());
        assert!(parse("x", r#"{"widgets":[]}"#, false).is_none());
        assert!(parse("x", "not json", false).is_none());
    }

    #[test]
    fn saving_the_current_widgets_and_loading_them_back_keeps_the_choices() {
        let mut s = Settings::default();
        s.widgets.clear();
        let mut g = Widget::new(1, "graph");
        g.on = true;
        g.x = 960;
        g.y = 540;
        g.source = "ram".into();
        g.style = "wave".into();
        g.frame = true;
        s.widgets.push(g);
        let json = snapshot_json("Mine", &s, 1920, 1080);
        let l = parse("mine", &json, true).expect("own file parses");
        let w = &build(&l, 1920, 1080, 5)[0];
        assert_eq!((w.kind.as_str(), w.source.as_str(), w.style.as_str(), w.x, w.y, w.frame), ("graph", "ram", "wave", 960, 540, true));
    }
}
