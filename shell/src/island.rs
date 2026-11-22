// Остров: часть бара, а не плавающее окно. В середине бара появляется таблетка с тем, что происходит сейчас:
// короткое уведомление одной строкой, громкость и яркость шкалой, раскладка, шторка. Если у уведомления есть
// что показать (картинка, кнопки, длинный текст), под таблеткой на баре висит карточка вплотную (notif.rs), и
// бар с ней читаются одной фигурой. Таблетка раскрывается из середины, звук и яркость идут без анимации.
// Работает при горизонтальном баре; у бара сбоку, спрятанного бара или при выключенной настройке всё как раньше.

use crate::app::State;
use crate::font::PX;
use crate::gfx::R;
use crate::notif::{pill, Note};
use crate::ui::{Act, Cur, Ui};

thread_local! {
    /// свободное место бара между левой и правой группами модулей (по последней отрисовке)
    static SPAN: std::cell::Cell<(i32, i32)> = const { std::cell::Cell::new((0, i32::MAX)) };
}

pub fn set_span(x0: i32, x1: i32) {
    SPAN.with(|s| s.set((x0, x1)));
}

/// самая узкая таблетка, которую ещё стоит показывать
pub const MIN_PILL: i32 = 150;

/// остров включён, есть где его показать и в баре хватает свободного места: когда открыто много окон и
/// модули занимают весь бар, таблетка не лезет поверх них, уведомления идут карточкой, громкость — обычной плашкой
pub fn on(st: &State) -> bool {
    st.set.island && !crate::bar::vert(&st.set) && !st.bar_hidden && fits()
}

pub fn fits() -> bool {
    let (a, b) = SPAN.with(|s| s.get());
    b.saturating_sub(a) >= MIN_PILL
}

/// сколько места под таблетку: середина свободного отрезка бара, ширина не больше него
pub fn span_pill(full: i32, (x0, x1): (i32, i32), bar_w: i32) -> (i32, i32) {
    let free = (x1 - x0).max(0);
    let w = full.min(free);
    let centre = bar_w / 2;
    let x = (centre - w / 2).clamp(x0, (x1 - w).max(x0));
    (x, w)
}

/// у уведомления есть что показать кроме строки: картинка, кнопки, длинный или многострочный текст
pub fn rich(n: &Note) -> bool {
    n.image.is_some() || n.actions.iter().any(|a| a.0 != "default") || n.body.contains('\n') || n.body.chars().count() > 70
}

/// что показывает таблетка сейчас
pub enum Pick<'a> {
    /// громкость, яркость, раскладка или шторка
    Osd(&'static str),
    /// уведомление, сколько ещё ждут за ним и как далеко ушло (0 — показывается, 0..1 — уходит: таблетка сжимается и гаснет, а не пропадает разом)
    Note(&'a Note, usize, f32),
    Nothing,
}

/// на баре сейчас таблетка
pub fn shows(st: &State) -> bool {
    on(st) && !matches!(pick(st), Pick::Nothing)
}

pub fn pick(st: &State) -> Pick<'_> {
    if let Some((kind, _)) = st.osd {
        return Pick::Osd(kind);
    }
    let live: Vec<&Note> = st.notifs.popups.iter().filter(|id| st.notifs.leaving_since(**id).is_none()).filter_map(|id| st.notifs.get(*id)).collect();
    if let Some(n) = live.first() {
        return Pick::Note(n, live.len() - 1, 0.0);
    }
    // последнее уходящее: таблетка доигрывает уход вместе с карточкой
    let gone = st.notifs.popups.iter().find_map(|id| Some((st.notifs.get(*id)?, st.notifs.leaving_since(*id)?)));
    match gone {
        Some((n, at)) => Pick::Note(n, 0, (at.elapsed().as_secs_f32() / st.notifs.leave_s.max(0.01)).clamp(0.01, 1.0)),
        None => Pick::Nothing,
    }
}

thread_local! {
    /// последняя нарисованная таблетка: какое уведомление, с какой ширины оно начало раскрываться, какой ширины сейчас и когда
    static LAST: std::cell::Cell<Option<(u32, i32, i32, std::time::Instant)>> = const { std::cell::Cell::new(None) };
}

/// с какой ширины раскрывается таблетка уведомления `id`: если секунду назад на баре была таблетка другого уведомления — с её ширины
/// (одна таблетка перетекает в другую), иначе с 45% полной
fn pill_start(id: u32, full: i32) -> i32 {
    match LAST.with(|l| l.get()) {
        Some((last, start, _, _)) if last == id => start,
        Some((_, _, cur, at)) if at.elapsed().as_secs_f32() < 1.0 => cur,
        _ => (full as f32 * 0.45).round() as i32,
    }
}

fn pill_remember(id: u32, start: i32, cur: i32) {
    LAST.with(|l| l.set(Some((id, start, cur, std::time::Instant::now()))));
}

/// одна строка для таблетки: программа, заголовок и, если влезает как строка, текст; без переводов строк
pub fn line(n: &Note) -> String {
    let mut s = format!("{} · {}", if n.app.is_empty() { crate::i18n::tr("система").to_string() } else { n.app.to_lowercase() }, n.summary);
    if !n.body.is_empty() && !rich(n) {
        s.push_str(" — ");
        s.push_str(&n.body);
    }
    s.replace(['\n', '\r'], " ")
}

/// плавное раскрытие: 0..1 по возрасту
fn ease(age: f32) -> f32 {
    let k = (age / 0.16).min(1.0);
    1.0 - (1.0 - k) * (1.0 - k)
}

/// ширина таблетки с полным содержимым `inner` px: поля с обеих сторон, не уже 150 и не шире 520 или бара
pub fn pill_width(inner: i32, bar_w: i32) -> i32 {
    (inner + 36).clamp(150, 520.min((bar_w - 40).max(150)))
}

pub fn draw(ui: &mut Ui, st: &State, w: i32, bar_h: i32, span: (i32, i32)) {
    if !on(st) {
        return;
    }
    let t = ui.t;
    let ph = bar_h - 10;
    let py = (bar_h - ph) / 2;
    match pick(st) {
        Pick::Nothing => {}
        Pick::Note(n, more, gone) => {
            let icon_w = 22;
            let (count, extra) = (if n.count > 1 { format!("×{}", n.count) } else { String::new() }, if more > 0 { format!("+{more}") } else { String::new() });
            let chips = if count.is_empty() { 0 } else { ui.w(PX, &count) + 8 } + if extra.is_empty() { 0 } else { ui.w(PX, &extra) + 8 };
            let room = (span.1 - span.0).min(520) - 36 - icon_w - chips;
            let text = ui.f.elide(PX, &line(n), room);
            let inner = icon_w + ui.w(PX, &text) + chips;
            let (px, full) = span_pill(pill_width(inner, w), span, w);
            let cx = px + full / 2;
            let age = n.born.elapsed().as_secs_f32();
            let e = ease(age);
            if age < 0.16 || gone > 0.0 {
                ui.anim = true;
            }
            // раскрытие: с ширины прежней таблетки (или 45%) до полной; уход: сжатие и угасание
            let start = pill_start(n.id, full);
            let cur = if gone > 0.0 {
                let k = 1.0 - gone * gone;
                (full as f32 * (0.45 + 0.55 * k)).round() as i32
            } else {
                (start as f32 + (full - start) as f32 * e).round() as i32
            };
            pill_remember(n.id, start, cur);
            let prev_opacity = ui.cv.opacity;
            if gone > 0.0 {
                ui.cv.opacity = prev_opacity * (1.0 - gone * gone);
            }
            let r = R::new(cx - cur / 2, py, cur, ph);
            let edge = if n.critical { t.ember } else { t.red };
            pill(ui, r, ph / 2, t.surface, edge);
            // содержимое — в полном размере, раскрывается из середины
            ui.cv.push_clip(r);
            let x = cx - full / 2 + 18;
            if n.icon.is_empty() {
                ui.cv.icon("bell", x, py + (ph - 14) / 2, 2, edge);
            } else {
                let icon = n.icon.clone();
                ui.app_icon(&icon, x, py + (ph - 16) / 2, 16, 1.0);
            }
            ui.text_in(PX, R::new(x + icon_w, py, full, ph), &text, t.text);
            let mut cxr = cx + full / 2 - 18;
            for (chip, col) in [(&extra, t.text_faint), (&count, t.red)] {
                if !chip.is_empty() {
                    let cw = ui.w(PX, chip);
                    cxr -= cw;
                    ui.text_in(PX, R::new(cxr, py, cw + 4, ph), chip, col);
                    cxr -= 8;
                }
            }
            ui.cv.pop_clip();
            ui.cv.opacity = prev_opacity;
            // уходящая таблетка уже не ловит нажатий
            if gone == 0.0 {
                ui.zone(r, Act::NotifClick(n.id), Cur::Pointer);
            }
        }
        Pick::Osd(kind) => {
            let (icon, label, value, warn): (&str, String, Option<f32>, bool) = match kind {
                "volume" => {
                    let a = &st.audio;
                    (if a.muted { "mute" } else { "speaker" }, if a.muted { crate::i18n::tr("тишина").to_string() } else { format!("{}%", (a.volume * 100.0).round()) }, Some(if a.muted { 0.0 } else { a.volume as f32 }), a.volume > 1.0)
                }
                "bright" => {
                    let v = st.bright.unwrap_or(0.0);
                    ("sun", format!("{}%", (v * 100.0).round()), Some(v), false)
                }
                "curtain" => {
                    let on = st.curtain.is_some();
                    (if on { "lock" } else { "eye" }, crate::i18n::tr(if on { "экран скрыт в трансляции" } else { "экран снова виден" }).to_string(), None, false)
                }
                _ => ("kb", st.niri.kb_names.get(st.niri.kb_idx).cloned().unwrap_or_else(|| st.niri.kb_short()), None, false),
            };
            let gauge_w = if value.is_some() { 120 } else { 0 };
            let lw = ui.w(PX, &label);
            let inner = 22 + if gauge_w > 0 { gauge_w + 10 } else { 0 } + lw;
            let (px, full) = span_pill(pill_width(inner, w), span, w);
            let cx = px + full / 2;
            let r = R::new(cx - full / 2, py, full, ph);
            pill(ui, r, ph / 2, t.surface, t.border);
            let x = cx - full / 2 + 18;
            ui.cv.icon(icon, x, py + (ph - 14) / 2, 2, t.red);
            let mut x2 = x + 22;
            if let Some(v) = value {
                ui.cv.gauge(R::new(x2, py + (ph - 8) / 2, gauge_w, 8), 12, v.clamp(0.0, 1.0), if warn { t.ember } else { t.red }, t.surface_hi.lerp(t.border, 0.45), 2);
                x2 += gauge_w + 10;
            }
            ui.text_in(PX, R::new(x2, py, full - (x2 - r.x) - 12, ph), &label, if value.is_some() { t.text } else { t.text_dim });
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_pill_stays_inside_the_free_part_of_a_crowded_bar() {
        assert_eq!(super::span_pill(300, (0, 1920), 1920), (810, 300), "empty bar: centred");
        // много окон слева: свободно только 700..1500 — таблетка сдвигается и не заходит на модули
        let (x, w) = super::span_pill(300, (700, 1500), 1920);
        assert!(x >= 700 && x + w <= 1500, "{x} {w}");
        let (x, w) = super::span_pill(900, (700, 1000), 1920);
        assert_eq!((x, w), (700, 300), "never wider than the free span");
    }

    use super::*;
    use std::time::Instant;

    fn note(app: &str, summary: &str, body: &str) -> Note {
        Note { id: 1, app: app.into(), icon: String::new(), summary: summary.into(), body: body.into(), actions: vec![], critical: false, image: None, born: Instant::now(), time: "12:00".into(), life: 7.0, left: 7.0, count: 1 }
    }

    #[test]
    fn a_short_note_is_a_single_line_pill_and_a_long_one_hangs_a_card() {
        assert!(!rich(&note("mail", "New mail", "From Sam")));
        assert!(!rich(&note("nemo", "Plugged in", "")));
        assert!(rich(&note("mail", "Report", &"x".repeat(80))), "a long body needs room");
        assert!(rich(&note("mail", "Report", "line one\nline two")));
        let mut n = note("mail", "Report", "");
        n.actions = vec![("open".into(), "open".into())];
        assert!(rich(&n), "buttons need a card");
        n.actions = vec![("default".into(), "open".into())];
        assert!(!rich(&n), "the default action is the click on the pill itself");
    }

    #[test]
    fn the_line_joins_app_summary_and_a_short_text_without_breaks() {
        assert_eq!(line(&note("Steam", "Download complete", "Cyberpunk is ready")), "steam · Download complete — Cyberpunk is ready");
        assert_eq!(line(&note("steam", "Done", "")), "steam · Done");
        let l = line(&note("a", "multi\nline", ""));
        assert!(!l.contains('\n'));
        // длинный текст не склеивается в строку: его место — карточка
        assert_eq!(line(&note("a", "s", &"y".repeat(80))), "a · s");
    }

    #[test]
    fn the_pill_is_never_tiny_or_wider_than_the_bar() {
        assert_eq!(pill_width(10, 1920), 150);
        assert_eq!(pill_width(2000, 1920), 520);
        assert_eq!(pill_width(2000, 400), 360, "a narrow bar bounds it");
        assert_eq!(pill_width(200, 1920), 236);
    }

    #[test]
    fn it_opens_smoothly_and_is_full_after_the_animation() {
        assert_eq!(ease(0.0), 0.0);
        assert!(ease(0.08) > 0.5 && ease(0.08) < 1.0);
        assert_eq!(ease(0.5), 1.0);
    }
}
