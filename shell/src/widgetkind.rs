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

/// список значений выбора: (значение, имя — русская строка)
pub type Values = &'static [(&'static str, &'static str)];

/// настройка вида: из этих описаний собраны панель в настройках, меню по правой кнопке и переключение щелчком; новая настройка — строка здесь
#[derive(Clone, Copy)]
pub enum Opt {
    /// выбор из списка; key — поле виджета (`source`, `style`, `anim`, `size`, `color`)
    Choice { key: &'static str, label: &'static str, icon: &'static str, values: Values },
    /// включено / выключено; key — `frame`, `label`, `seconds`, `date`
    Toggle { key: &'static str, label: &'static str },
    /// файл скрипта из ~/.config/nemo/scripts (имя не переводится)
    Script,
}

const SOURCE: Opt = Opt::Choice { key: "source", label: "источник", icon: "chart", values: w::SOURCES };
const STYLE: Opt = Opt::Choice { key: "style", label: "вид графика", icon: "grid", values: w::STYLES };
const EQUALIZER: Opt = Opt::Choice { key: "style", label: "эквалайзер", icon: "music", values: w::STYLES };
const ANIM: Opt = Opt::Choice { key: "anim", label: "анимация", icon: "eye", values: w::ANIMS };
const LABEL: Opt = Opt::Toggle { key: "label", label: "подпись" };
const SECONDS: Opt = Opt::Toggle { key: "seconds", label: "секунды" };
const DATE: Opt = Opt::Toggle { key: "date", label: "дата" };
/// у всех видов
pub const SIZE: Opt = Opt::Choice { key: "size", label: "размер", icon: "max", values: w::SIZES };
pub const COLOR: Opt = Opt::Choice { key: "color", label: "цвет", icon: "image", values: w::COLORS };
pub const FRAME: Opt = Opt::Toggle { key: "frame", label: "подложка" };

pub struct Kind {
    pub id: &'static str,
    /// имя вида — русская строка, переводится при показе
    pub name: &'static str,
    /// размеры видимой карточки для классов s / m / l (поле под тень и всплытие добавляет `canvas`)
    pub sizes: [(i32, i32); 3],
    pub needs: &'static [Need],
    /// новый виджет этого вида с подложкой и датой (карточки «небо», «состояние», «неделя»)
    pub card: bool,
    /// в списке мастера первой настройки (скрипту там нечего показать)
    pub in_wizard: bool,
    /// настройки вида, кроме общих (размер, цвет, подложка)
    pub opts: &'static [Opt],
    pub draw: DrawFn,
}

use Need::*;

pub const REGISTRY: &[Kind] = &[
    Kind { id: "clock", name: "часы", sizes: [(252, 130), (260, 152), (320, 196)], needs: &[Seconds], card: false, in_wizard: true, opts: &[SECONDS, DATE], draw: w::draw_clock },
    Kind { id: "graph", name: "график", sizes: [(220, 80), (300, 108), (440, 164)], needs: &[Seconds], card: false, in_wizard: true, opts: &[SOURCE, STYLE, ANIM, LABEL], draw: w::draw_graph },
    Kind { id: "net", name: "сеть", sizes: [(220, 80), (300, 108), (440, 164)], needs: &[Seconds], card: false, in_wizard: true, opts: &[STYLE, ANIM, LABEL], draw: w::draw_net },
    Kind { id: "media", name: "музыка", sizes: [(320, 96), (400, 120), (500, 154)], needs: &[Media], card: false, in_wizard: true, opts: &[EQUALIZER, LABEL], draw: w::draw_media },
    Kind { id: "cal", name: "календарь", sizes: [(236, 204), (280, 236), (350, 300)], needs: &[], card: false, in_wizard: true, opts: &[], draw: w::draw_cal },
    Kind { id: "weather", name: "погода", sizes: [(230, 80), (300, 142), (390, 166)], needs: &[Weather], card: false, in_wizard: true, opts: &[], draw: w::draw_weather },
    Kind { id: "bt", name: "bluetooth", sizes: [(230, 60), (300, 104), (380, 148)], needs: &[], card: false, in_wizard: true, opts: &[], draw: w::draw_bt },
    Kind { id: "script", name: "скрипт", sizes: [(220, 48), (300, 94), (400, 144)], needs: &[], card: false, in_wizard: false, opts: &[Opt::Script], draw: w::draw_script },
    Kind { id: "sky", name: "небо", sizes: [(300, 148), (380, 188), (480, 236)], needs: &[Seconds, Weather], card: true, in_wizard: true, opts: &[], draw: w::draw_sky },
    Kind { id: "vitals", name: "состояние", sizes: [(284, 108), (324, 152), (380, 196)], needs: &[Seconds], card: true, in_wizard: true, opts: &[], draw: w::draw_vitals },
    Kind { id: "week", name: "неделя", sizes: [(340, 108), (424, 120), (540, 144)], needs: &[], card: true, in_wizard: true, opts: &[], draw: w::draw_week },
];

/// размеры неизвестного вида (виджет из чужого конфига): как раньше
const UNKNOWN: [(i32, i32); 3] = [(200, 84), (300, 124), (400, 184)];

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

/// размер окна виджета: карточка класса s / m (любой другой) / l, по бокам и снизу поле под тень (MARGIN: мягкая тень уходит на 12 px, раньше
/// окно давало ей 4 и обрезало), сверху запас под всплытие (RISE)
pub fn canvas(kind: &str, size: &str) -> (i32, i32) {
    let sizes = get(kind).map_or(UNKNOWN, |k| k.sizes);
    let (w, h) = sizes[match size {
        "s" => 0,
        "l" => 2,
        _ => 1,
    }];
    (w + 2 * w::MARGIN, h + w::RISE + w::MARGIN)
}

/// настройки виджета в порядке показа: выборы вида, затем размер и цвет, затем переключатели (подложка первой)
pub fn options(kind: &str) -> (Vec<Opt>, Vec<Opt>) {
    let own = get(kind).map_or(&[][..], |k| k.opts);
    let mut choices: Vec<Opt> = own.iter().copied().filter(|o| !matches!(o, Opt::Toggle { .. })).collect();
    choices.extend([SIZE, COLOR]);
    let mut toggles = vec![FRAME];
    toggles.extend(own.iter().copied().filter(|o| matches!(o, Opt::Toggle { .. })));
    (choices, toggles)
}

/// значение переключателя по ключу
pub fn flag(w: &Widget, key: &str) -> bool {
    match key {
        "frame" => w.frame,
        "label" => w.label,
        "seconds" => w.seconds,
        "date" => w.date,
        "on" => w.on,
        _ => false,
    }
}

/// значение выбора по ключу
pub fn value<'a>(w: &'a Widget, key: &str) -> &'a str {
    match key {
        "source" => &w.source,
        "style" => &w.style,
        "anim" => &w.anim,
        "size" => &w.size,
        "color" => &w.color,
        _ => "",
    }
}

/// следующее значение по щелчку: для выбора — следующий в списке по кругу, для скрипта — следующий файл, для переключателя — пусто
/// (`widgets::set_opt` сама переворачивает флаг)
pub fn next_value(w: &Widget, key: &str) -> String {
    if key == "source" && w.kind == "script" {
        let v = crate::plugins::scripts();
        return v.iter().position(|n| *n == w.source).map(|i| v[(i + 1) % v.len()].clone()).or_else(|| v.first().cloned()).unwrap_or_default();
    }
    let (choices, _) = options(&w.kind);
    for o in choices {
        if let Opt::Choice { key: k, values, .. } = o {
            if k == key {
                let cur = value(w, key);
                let i = values.iter().position(|v| v.0 == cur).unwrap_or(values.len() - 1);
                return values[(i + 1) % values.len()].0.to_string();
            }
        }
    }
    String::new()
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
        // карточка как была (ширина и высота за вычетом всплытия), окно — карточка + поле под тень и всплытие
        assert_eq!(canvas("clock", "s"), (252 + 24, 130 + 16 + 12));
        assert_eq!(canvas("clock", "m"), (260 + 24, 152 + 16 + 12));
        assert_eq!(canvas("clock", "l"), (320 + 24, 196 + 16 + 12));
        assert_eq!(canvas("sky", "m"), (380 + 24, 188 + 16 + 12));
        assert_eq!(canvas("nope", "l"), (400 + 24, 184 + 16 + 12));
    }

    #[test]
    fn needs_come_from_the_table() {
        assert!(needs("clock", Need::Seconds) && needs("sky", Need::Seconds) && needs("vitals", Need::Seconds));
        assert!(!needs("cal", Need::Seconds) && !needs("week", Need::Seconds));
        assert!(needs("weather", Need::Weather) && needs("sky", Need::Weather) && !needs("clock", Need::Weather));
        assert!(needs("media", Need::Media) && !needs("sky", Need::Media));
    }

    #[test]
    fn options_come_from_the_schema_in_the_old_order() {
        let keys = |kind: &str| -> (Vec<&'static str>, Vec<&'static str>) {
            let (c, t) = options(kind);
            let name = |o: &Opt| match o {
                Opt::Choice { key, .. } | Opt::Toggle { key, .. } => *key,
                Opt::Script => "script",
            };
            (c.iter().map(name).collect(), t.iter().map(name).collect())
        };
        assert_eq!(keys("graph"), (vec!["source", "style", "anim", "size", "color"], vec!["frame", "label"]));
        assert_eq!(keys("net"), (vec!["style", "anim", "size", "color"], vec!["frame", "label"]));
        assert_eq!(keys("media"), (vec!["style", "size", "color"], vec!["frame", "label"]));
        assert_eq!(keys("clock"), (vec!["size", "color"], vec!["frame", "seconds", "date"]));
        assert_eq!(keys("script"), (vec!["script", "size", "color"], vec!["frame"]));
        assert_eq!(keys("cal"), (vec!["size", "color"], vec!["frame"]));
    }

    #[test]
    fn a_click_cycles_to_the_next_value_and_wraps() {
        let mut g = Widget::new(1, "graph");
        g.style = "bars".into();
        assert_eq!(next_value(&g, "style"), "line");
        g.style = "dots".into();
        assert_eq!(next_value(&g, "style"), "bars", "wraps around");
        g.size = "l".into();
        assert_eq!(next_value(&g, "size"), "s");
        assert_eq!(next_value(&g, "frame"), "", "a toggle has no value to pick");
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
