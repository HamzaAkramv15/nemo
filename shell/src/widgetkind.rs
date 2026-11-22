// Реестр видов виджетов: одна таблица вместо списков, разбросанных по десятку файлов. Вид — это имя (русская строка, как везде в шелле),
// размеры трёх классов, то, что ему нужно от шелла (`needs`: тик раз в секунду, погода, плеер), умолчания нового виджета и функция
// рисования одной и той же сигнатуры. Всё, что раньше перечисляло виды вручную (список «добавить», размеры, тик, нужна ли погода, нужен ли
// плеер, мастер первой настройки, подпись в режиме расстановки), теперь спрашивает эту таблицу. Новый вид — строка здесь и функция рисования.

use crate::app::State;
use crate::config::Widget;
use crate::gfx::{C, R};
use crate::ui::Ui;
use crate::widgets as w;

/// что виджету нужно от шелла: по этому тик, опрос погоды и плеера включаются только когда такой виджет включён
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Need {
    /// перерисовка раз в секунду (часы с секундами, живые графики)
    Seconds,
    /// опрос погоды
    Weather,
    /// данные плеера
    Media,
}

/// рисует виджет в прямоугольнике r (весь холст без поля под тень) цветом c; подложку рисует сам вид
pub type DrawFn = fn(&mut Ui, &State, &Widget, R, C);

pub struct Kind {
    pub id: &'static str,
    /// имя вида — русская строка, переводится при показе
    pub name: &'static str,
    /// размеры холста для классов s / m / l (без поля +4 на тень)
    pub sizes: [(i32, i32); 3],
    pub needs: &'static [Need],
    /// новый виджет этого вида с подложкой и датой (карточки «небо», «состояние», «неделя»)
    pub card: bool,
    /// в списке мастера первой настройки (скрипту там нечего показать)
    pub in_wizard: bool,
    pub draw: DrawFn,
}

use Need::*;

pub const REGISTRY: &[Kind] = &[
    Kind { id: "clock", name: "часы", sizes: [(260, 100), (360, 128), (540, 190)], needs: &[Seconds], card: false, in_wizard: true, draw: w::draw_clock },
    Kind { id: "graph", name: "график", sizes: [(220, 96), (300, 124), (440, 180)], needs: &[Seconds], card: false, in_wizard: true, draw: w::draw_graph },
    Kind { id: "net", name: "сеть", sizes: [(220, 96), (300, 124), (440, 180)], needs: &[Seconds], card: false, in_wizard: true, draw: w::draw_net },
    Kind { id: "media", name: "музыка", sizes: [(320, 112), (400, 136), (500, 170)], needs: &[Media], card: false, in_wizard: true, draw: w::draw_media },
    Kind { id: "cal", name: "календарь", sizes: [(236, 220), (280, 252), (350, 316)], needs: &[], card: false, in_wizard: true, draw: w::draw_cal },
    Kind { id: "weather", name: "погода", sizes: [(230, 96), (300, 150), (390, 176)], needs: &[Weather], card: false, in_wizard: true, draw: w::draw_weather },
    Kind { id: "bt", name: "bluetooth", sizes: [(230, 76), (300, 120), (380, 164)], needs: &[], card: false, in_wizard: true, draw: w::draw_bt },
    Kind { id: "script", name: "скрипт", sizes: [(220, 64), (300, 110), (400, 160)], needs: &[], card: false, in_wizard: false, draw: w::draw_script },
    Kind { id: "sky", name: "небо", sizes: [(300, 164), (380, 204), (480, 252)], needs: &[Seconds, Weather], card: true, in_wizard: true, draw: w::draw_sky },
    Kind { id: "vitals", name: "состояние", sizes: [(284, 124), (324, 168), (380, 212)], needs: &[Seconds], card: true, in_wizard: true, draw: w::draw_vitals },
    Kind { id: "week", name: "неделя", sizes: [(340, 116), (424, 132), (540, 156)], needs: &[], card: true, in_wizard: true, draw: w::draw_week },
];

/// размеры неизвестного вида (виджет из чужого конфига): как раньше
const UNKNOWN: [(i32, i32); 3] = [(200, 100), (300, 140), (400, 200)];

pub fn get(id: &str) -> Option<&'static Kind> {
    REGISTRY.iter().find(|k| k.id == id)
}

/// все виды: (id, имя)
pub fn kinds() -> impl Iterator<Item = (&'static str, &'static str)> + Clone {
    REGISTRY.iter().map(|k| (k.id, k.name))
}

/// имя вида; неизвестный вид — как раньше первый в списке
pub fn name(id: &str) -> &'static str {
    get(id).unwrap_or(&REGISTRY[0]).name
}

/// нужно ли виджету что-то из `need`
pub fn needs(id: &str, need: Need) -> bool {
    get(id).is_some_and(|k| k.needs.contains(&need))
}

/// включён ли хоть один виджет, которому нужно `need`
pub fn any_needs(widgets: &[Widget], need: Need) -> bool {
    widgets.iter().any(|w| w.on && needs(&w.kind, need))
}

/// размер холста виджета с полем +4 под тень: класс s / m (любой другой) / l
pub fn canvas(kind: &str, size: &str) -> (i32, i32) {
    let sizes = get(kind).map_or(UNKNOWN, |k| k.sizes);
    let (w, h) = sizes[match size {
        "s" => 0,
        "l" => 2,
        _ => 1,
    }];
    (w + 4, h + 4)
}

/// умолчания нового виджета вида `kind`
pub fn apply_defaults(w: &mut Widget) {
    if w.kind == "clock" {
        w.frame = false;
    }
    if get(&w.kind).is_some_and(|k| k.card) {
        w.frame = true;
        w.date = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_has_a_unique_id_a_name_and_three_growing_sizes() {
        for (i, k) in REGISTRY.iter().enumerate() {
            assert!(!k.name.is_empty());
            assert!(REGISTRY[i + 1..].iter().all(|o| o.id != k.id), "{} twice", k.id);
            assert!(k.sizes[0].0 <= k.sizes[1].0 && k.sizes[1].0 <= k.sizes[2].0, "{} widths", k.id);
            assert!(k.sizes[0].1 <= k.sizes[1].1 && k.sizes[1].1 <= k.sizes[2].1, "{} heights", k.id);
        }
    }

    #[test]
    fn sizes_match_the_old_table() {
        // значения до реестра: s / m / l с полем 4 под тень
        assert_eq!(canvas("clock", "s"), (264, 104));
        assert_eq!(canvas("clock", "m"), (364, 132));
        assert_eq!(canvas("clock", "l"), (544, 194));
        assert_eq!(canvas("sky", "m"), (384, 208));
        assert_eq!(canvas("nope", "l"), (404, 204));
    }

    #[test]
    fn needs_come_from_the_table() {
        assert!(needs("clock", Need::Seconds) && needs("sky", Need::Seconds) && needs("vitals", Need::Seconds));
        assert!(!needs("cal", Need::Seconds) && !needs("week", Need::Seconds));
        assert!(needs("weather", Need::Weather) && needs("sky", Need::Weather) && !needs("clock", Need::Weather));
        assert!(needs("media", Need::Media) && !needs("sky", Need::Media));
    }

    #[test]
    fn cards_default_to_a_panel_and_the_clock_to_none() {
        let mut c = Widget::new(1, "clock");
        assert!(!c.frame);
        let mut s = Widget::new(2, "sky");
        assert!(s.frame && s.date);
        c.frame = true;
        apply_defaults(&mut s);
        assert!(s.frame);
    }
}
