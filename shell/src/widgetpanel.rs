// Подложки виджетов. Раньше подложка была одной тёмной плашкой с настройками: углы-фаски брались из темы, тень — тоже из темы, и сочетание
// «пиксельные фаски + мягкая размытая тень + тонкая холодная рамка + строки развёртки + плавающая линия» давало шершавые края, точки на рамке
// и линию, висящую внутри карточки. Теперь у каждого стиля свой законченный рецепт: форма углов, рамка, тень, заливка и текстура подобраны вместе
// и не зависят от общих настроек углов и тени. Отделка («вид») — отдельная и только украшает: метка, волна на верхней кромке, прицел.
//
// Стили:   стекло (мягкая тень, скруглённые углы со сглаживанием), иней (плотнее, с бликом), плотная (плоская карточка), пиксель (фаски, жёсткая
//          тень по сетке пикселей, рамка 2 px — подпись Nemo), контур (прозрачная, уголки), свечение (тёмная с цветным ореолом).
// Правила:  жёсткая тень и фаски — только в пиксельном стиле; сглаживание — только в «гладких»; полупрозрачная заливка не просвечивает тенью
//          (тень рисуется только снаружи формы); строки развёртки — только на заливке пиксельного стиля, внутри рамки.

use crate::app::State;
use crate::config::Widget;
use crate::gfx::{C, R};
use crate::ui::Ui;

pub const STYLES: &[(&str, &str)] = &[("glass", "стекло"), ("frost", "иней"), ("solid", "плотная"), ("pixel", "пиксельный"), ("outline", "контур"), ("glow", "свечение")];
pub const LOOKS: &[(&str, &str)] = &[("plain", "чисто"), ("accent", "метка"), ("reef", "рифовый"), ("hud", "прицел")];

thread_local! {
    /// рисуются образцы в окне настроек: размытие под ними не просить (оно легло бы пятном под само окно)
    static PREVIEW: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub fn set_preview(on: bool) {
    PREVIEW.with(|p| p.set(on));
}

pub fn valid_style(s: &str) -> bool {
    STYLES.iter().any(|x| x.0 == s)
}
pub fn valid_look(s: &str) -> bool {
    LOOKS.iter().any(|x| x.0 == s)
}

/// стиль виджета: свой, если задан, иначе общий
pub fn style_of<'a>(w: &'a Widget, st: &'a State) -> &'a str {
    if valid_style(&w.panel) { &w.panel } else { &st.set.widget_panel }
}

#[derive(Clone, Copy)]
enum Shape {
    /// скругление радиуса r со сглаживанием
    Round(i32),
    /// фаска в n клеток, без сглаживания
    Notch(i32),
    /// только уголки
    Brackets,
}

#[derive(Clone, Copy)]
enum Shade {
    None,
    /// мягкая: наибольшая прозрачность, ширина, сдвиг вниз
    Soft(f32, i32, i32),
    /// жёсткая со сдвигом вниз-вправо
    Hard(i32),
    /// цветной ореол: цвет, ширина
    Glow(C, i32),
}

struct Recipe {
    shape: Shape,
    fill: C,
    border: C,
    bw: i32,
    shade: Shade,
    sheen: bool,
    scan: bool,
}

/// акцент, читаемый на подложке (widgets::readable)
fn accent(ui: &Ui) -> C {
    crate::widgets::readable(ui.t.red, ui.t.bg, 4.5)
}

fn recipe(ui: &Ui, st: &State, style: &str, alpha: f32) -> Recipe {
    let t = ui.t;
    let a = |default: f32| if alpha > 0.0 { alpha } else { default };
    let hi = if t.light { C(0xff000000) } else { t.text_hi };
    let amp = |v: f32| if t.light { v * 0.5 } else { v };
    match style {
        "frost" => Recipe { shape: Shape::Round(10), fill: t.surface.alpha(a(0.62)), border: hi.alpha(0.22), bw: 1, shade: Shade::Soft(amp(0.28), 10, 2), sheen: true, scan: false },
        "solid" => Recipe { shape: Shape::Round(5), fill: t.panel.alpha(a(1.0)), border: t.border.lerp(hi, 0.10), bw: 1, shade: Shade::Soft(amp(0.30), 6, 2), sheen: false, scan: false },
        "pixel" => Recipe {
            shape: Shape::Notch(3),
            fill: t.surface.alpha(a(1.0)),
            border: t.border.lerp(accent(ui), 0.55),
            bw: 2,
            shade: Shade::Hard(3),
            sheen: false,
            scan: st.set.scanlines,
        },
        "outline" => Recipe { shape: Shape::Brackets, fill: t.bg.alpha(a(0.26)), border: accent(ui), bw: 2, shade: Shade::None, sheen: false, scan: false },
        "glow" => {
            let ac = accent(ui);
            Recipe { shape: Shape::Round(8), fill: t.bg.alpha(a(0.88)), border: ac.alpha(0.85), bw: 1, shade: Shade::Glow(ac.alpha(0.5), 9), sheen: false, scan: false }
        }
        // glass
        _ => Recipe { shape: Shape::Round(10), fill: t.bg.alpha(a(0.72)), border: hi.alpha(0.16), bw: 1, shade: Shade::Soft(amp(0.34), 12, 3), sheen: false, scan: false },
    }
}

// ── геометрия ───────────────────────────────────────────────────────────────

/// расстояние со знаком от центра пикселя до скруглённого прямоугольника (минус — внутри)
fn sdf(px: f32, py: f32, r: R, rad: f32, dy: f32) -> f32 {
    let (cx, cy) = (r.x as f32 + r.w as f32 / 2.0, r.y as f32 + r.h as f32 / 2.0 + dy);
    let (hw, hh) = (r.w as f32 / 2.0, r.h as f32 / 2.0);
    let (qx, qy) = ((px - cx).abs() - (hw - rad), (py - cy).abs() - (hh - rad));
    (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt() + qx.max(qy).min(0.0) - rad
}

/// покрытие пикселя (0..1) формой радиуса rad
fn cover(d: f32) -> f32 {
    (0.5 - d).clamp(0.0, 1.0)
}

/// строки скруглённой заливки: на каждой строке закрашенный отрезок и сглаженные крайние пиксели
fn round_fill(ui: &mut Ui, r: R, rad: i32, c: C) {
    let rad = rad.min(r.w / 2).min(r.h / 2).max(0);
    let rf = rad as f32;
    for y in 0..r.h {
        let dyc = y.min(r.h - 1 - y);
        if dyc >= rad {
            // середина: целые строки одним махом, ниже
            let mid = (r.h - 2 * rad).max(0);
            if y == rad && mid > 0 {
                ui.cv.fill(R::new(r.x, r.y + rad, r.w, mid), c);
            }
            continue;
        }
        let d = rf - dyc as f32 - 0.5;
        let ins = rf - (rf * rf - d * d).max(0.0).sqrt();
        let fl = ins.floor();
        let frac = ins - fl;
        let start = fl as i32 + if frac > 0.0 { 1 } else { 0 };
        ui.cv.fill(R::new(r.x + start, r.y + y, r.w - 2 * start, 1), c);
        if frac > 0.0 {
            let aa = c.alpha(c.a() as f32 / 255.0 * (1.0 - frac));
            ui.cv.fill(R::new(r.x + fl as i32, r.y + y, 1, 1), aa);
            ui.cv.fill(R::new(r.right() - fl as i32 - 1, r.y + y, 1, 1), aa);
        }
    }
}

/// рамка скруглённой формы: прямые стороны заливками, углы — по пикселям с покрытием «наружный круг минус внутренний»
fn round_ring(ui: &mut Ui, r: R, rad: i32, bw: i32, c: C) {
    let rad = rad.min(r.w / 2).min(r.h / 2).max(0);
    if rad == 0 {
        ui.cv.outline(r, c, bw);
        return;
    }
    ui.cv.fill(R::new(r.x + rad, r.y, r.w - 2 * rad, bw), c);
    ui.cv.fill(R::new(r.x + rad, r.bottom() - bw, r.w - 2 * rad, bw), c);
    ui.cv.fill(R::new(r.x, r.y + rad, bw, r.h - 2 * rad), c);
    ui.cv.fill(R::new(r.right() - bw, r.y + rad, bw, r.h - 2 * rad), c);
    let (rf, inner) = (rad as f32, (rad - bw).max(0) as f32);
    let base = c.a() as f32 / 255.0;
    for (ox, oy, sx, sy) in [(r.x, r.y, 1, 1), (r.right() - rad, r.y, -1, 1), (r.x, r.bottom() - rad, 1, -1), (r.right() - rad, r.bottom() - rad, -1, -1)] {
        for j in 0..rad {
            for i in 0..rad {
                // координаты относительно центра дуги: он на (rad, rad) от угла
                let (dx, dy) = (if sx > 0 { rf - i as f32 - 0.5 } else { i as f32 + 0.5 }, if sy > 0 { rf - j as f32 - 0.5 } else { j as f32 + 0.5 });
                let dist = (dx * dx + dy * dy).sqrt();
                let ring = (rf - dist + 0.5).clamp(0.0, 1.0) * (1.0 - (inner - dist + 0.5).clamp(0.0, 1.0));
                if ring > 0.02 {
                    ui.cv.fill(R::new(ox + i, oy + j, 1, 1), c.alpha(base * ring));
                }
            }
        }
    }
}

/// строка фаски: отрезок [x0, x1) на строке y у прямоугольника с фаской n
fn notch_span(r: R, n: i32, y: i32) -> Option<(i32, i32)> {
    if y < r.y || y >= r.bottom() {
        return None;
    }
    let cut = r.w > 2 * n && r.h > 2 * n && (y < r.y + n || y >= r.bottom() - n);
    Some(if cut { (r.x + n, r.right() - n) } else { (r.x, r.right()) })
}

fn notch_fill(ui: &mut Ui, r: R, n: i32, c: C) {
    if r.w <= 2 * n || r.h <= 2 * n {
        return ui.cv.fill(r, c);
    }
    ui.cv.fill(R::new(r.x + n, r.y, r.w - 2 * n, r.h), c);
    ui.cv.fill(R::new(r.x, r.y + n, n, r.h - 2 * n), c);
    ui.cv.fill(R::new(r.right() - n, r.y + n, n, r.h - 2 * n), c);
}

/// рамка фаски шириной bw: внешняя фигура минус внутренняя (фаска внутри на клетку меньше), без одиночных пикселей по углам
fn notch_ring(ui: &mut Ui, r: R, n: i32, bw: i32, c: C) {
    let inner = R::new(r.x + bw, r.y + bw, r.w - 2 * bw, r.h - 2 * bw);
    let ni = (n - 1).max(0);
    for y in r.y..r.bottom() {
        let Some((a0, a1)) = notch_span(r, n, y) else { continue };
        match notch_span(inner, ni, y) {
            Some((b0, b1)) if b1 > b0 => {
                if b0 > a0 {
                    ui.cv.fill(R::new(a0, y, b0 - a0, 1), c);
                }
                if a1 > b1 {
                    ui.cv.fill(R::new(b1, y, a1 - b1, 1), c);
                }
            }
            _ => ui.cv.fill(R::new(a0, y, a1 - a0, 1), c),
        }
    }
}

/// жёсткая тень: фигура со сдвигом (d, d) без той части, что закрыта самой панелью (иначе под стеклом тень просвечивает второй рамкой)
fn hard_shadow(ui: &mut Ui, r: R, n: i32, d: i32, c: C) {
    let s = R::new(r.x + d, r.y + d, r.w, r.h);
    for y in s.y..s.bottom() {
        let Some((a0, a1)) = notch_span(s, n, y) else { continue };
        match notch_span(r, n, y) {
            Some((b0, b1)) if b0 < a1 && b1 > a0 => {
                if b0 > a0 {
                    ui.cv.fill(R::new(a0, y, b0 - a0, 1), c);
                }
                if b1 < a1 {
                    ui.cv.fill(R::new(b1, y, a1 - b1, 1), c);
                }
            }
            _ => ui.cv.fill(R::new(a0, y, a1 - a0, 1), c),
        }
    }
}

/// мягкая тень снаружи формы: по строкам, отрезками одинаковой прозрачности; сама форма тень не получает
fn soft_shadow(ui: &mut Ui, r: R, rad: i32, color: C, amp: f32, spread: i32, dy: i32) {
    let (rf, sp) = (rad as f32, spread.max(1) as f32);
    let (x0, x1) = (r.x - spread, r.right() + spread);
    let (y0, y1) = (r.y - spread + dy.min(0), r.bottom() + spread + dy.max(0));
    for y in y0..y1 {
        let py = y as f32 + 0.5;
        let mut run: Option<(i32, u32)> = None;
        let flush = |ui: &mut Ui, run: &mut Option<(i32, u32)>, end: i32| {
            if let Some((start, q)) = run.take() {
                if q > 0 {
                    ui.cv.fill(R::new(start, y, end - start, 1), color.alpha(q as f32 / 255.0));
                }
            }
        };
        for x in x0..x1 {
            let px = x as f32 + 0.5;
            let own = cover(sdf(px, py, r, rf, 0.0));
            let d = sdf(px, py, r, rf, dy as f32);
            let fall = if d <= 0.0 { 1.0 } else { (1.0 - d / sp).max(0.0).powi(2) };
            let q = ((amp * fall * (1.0 - own)).clamp(0.0, 1.0) * 255.0).round() as u32;
            match run {
                Some((_, rq)) if rq == q => {}
                _ => {
                    flush(ui, &mut run, x);
                    run = Some((x, q));
                }
            }
        }
        flush(ui, &mut run, x1);
    }
}

/// диагональный блик иней-стиля: светлая россыпь у верхней кромки, идущая на нет книзу (шахматка, без градиента — края не мылятся)
fn sheen(ui: &mut Ui, r: R, rad: i32, c: C) {
    let depth = (r.h / 4).clamp(6, 16);
    for j in 0..depth {
        let a = 0.14 * (1.0 - j as f32 / depth as f32);
        let mut x = r.x + rad + (j & 1);
        while x < r.right() - rad {
            ui.cv.fill(R::new(x, r.y + 1 + j, 1, 1), c.alpha(a));
            x += 2;
        }
    }
}

fn brackets(ui: &mut Ui, r: R, len: i32, th: i32, c: C) {
    for (x, y, sx, sy) in [(r.x, r.y, 1, 1), (r.right(), r.y, -1, 1), (r.x, r.bottom(), 1, -1), (r.right(), r.bottom(), -1, -1)] {
        let hx = if sx > 0 { x } else { x - len };
        let vy = if sy > 0 { y } else { y - len };
        let hy = if sy > 0 { y } else { y - th };
        let vx = if sx > 0 { x } else { x - th };
        ui.cv.fill(R::new(hx, hy, len, th), c);
        ui.cv.fill(R::new(vx, vy, th, len), c);
    }
}

// ── рисование ───────────────────────────────────────────────────────────────

/// нарисовать подложку стилем `style` в прямоугольнике r (карточка; поле под тень — вокруг, окно виджета его вмещает: widgets::MARGIN)
pub fn paint(ui: &mut Ui, st: &State, style: &str, alpha: f32, r: R) {
    paint_with(ui, st, style, alpha, r, true)
}

fn paint_with(ui: &mut Ui, st: &State, style: &str, alpha: f32, r: R, blur: bool) {
    let blur = blur && !PREVIEW.with(|p| p.get());
    let rc = recipe(ui, st, style, alpha);
    let shade_c = C(0xff000000);
    match rc.shape {
        Shape::Round(rad) => {
            match rc.shade {
                Shade::Soft(amp, spread, dy) => soft_shadow(ui, r, rad, shade_c, amp, spread, dy),
                Shade::Glow(c, spread) => soft_shadow(ui, r, rad, c.alpha(1.0), c.a() as f32 / 255.0, spread, 0),
                _ => {}
            }
            if blur && rc.fill.a() < 255 {
                ui.cv.blur_rect(r, rad);
            }
            round_fill(ui, r, rad, rc.fill);
            if rc.sheen {
                sheen(ui, r, rad, ui.t.text_hi);
            }
            // блик у кромки внутри рамки: общий для гладких стилей, одна линия на прямом участке
            if matches!(style, "glass" | "glow") && r.w > 2 * rad + 4 {
                ui.cv.fill(R::new(r.x + rad, r.y + 1, r.w - 2 * rad, 1), ui.t.text_hi.alpha(if ui.t.light { 0.0 } else { 0.07 }));
            }
            round_ring(ui, r, rad, rc.bw, rc.border);
        }
        Shape::Notch(n) => {
            if let Shade::Hard(d) = rc.shade {
                hard_shadow(ui, r, n, d, shade_c.alpha(if ui.t.light { 0.28 } else { 0.5 }));
            }
            if blur && rc.fill.a() < 255 {
                ui.cv.blur_rect(r, n);
            }
            notch_fill(ui, r, n, rc.fill);
            if rc.scan {
                let k = if ui.t.light { 0.06 } else { 0.22 };
                // scanlines_in считает в координатах буфера и сдвиг холста (ox, oy) не применяет: прибавляем сами, иначе полосы уезжают влево-вверх
                ui.cv.scanlines_in(R::new(r.x + rc.bw + ui.cv.ox, r.y + rc.bw + ui.cv.oy, r.w - 2 * rc.bw, r.h - 2 * rc.bw), k);
            }
            notch_ring(ui, r, n, rc.bw, rc.border);
        }
        Shape::Brackets => {
            ui.cv.fill(r, rc.fill);
            ui.cv.outline(r, rc.border.alpha(0.16), 1);
            brackets(ui, r, 14, rc.bw, rc.border);
        }
    }
}

/// образец для настроек: двухцветный фон (чтобы была видна прозрачность) и подложка тем же кодом, что рисует её на столе; без размытия под окном
pub fn swatch(ui: &mut Ui, st: &State, style: &str, alpha: f32, look: &str, r: R) {
    let t = ui.t;
    ui.cv.fill(r, t.surface_hi);
    ui.cv.fill(R::new(r.x + r.w / 2, r.y, r.w - r.w / 2, r.h), t.red.lerp(t.bg, 0.55));
    // диагональная россыпь на границе половин: прозрачность видна на обоих тонах
    for j in 0..r.h {
        ui.cv.fill(R::new(r.x + r.w / 2 - 3 + (j % 6), r.y + j, 1, 1), t.text_faint.alpha(0.25));
    }
    let card = R::new(r.x + 12, r.y + 12, r.w - 24, r.h - 24);
    paint_with(ui, st, style, alpha, card, false);
    decorate(ui, st, look, style, card, true);
}

/// подложка с отделкой для примера в настройках (без размытия под окном)
pub fn paint_preview(ui: &mut Ui, st: &State, style: &str, alpha: f32, look: &str, r: R) {
    paint_with(ui, st, style, alpha, r, false);
    decorate(ui, st, look, style, r, true);
}

/// кромка формы для отделки: радиус скругления и отступ от угла, куда можно ставить украшения (у фасок и уголков он другой)
fn edge_inset(style: &str) -> i32 {
    match style {
        "pixel" => 8,
        "solid" => 9,
        "outline" => 18,
        "glow" => 12,
        _ => 14,
    }
}

/// отделка: украшение поверх подложки. Всё сидит на самой кромке или за ней, ничего не «плавает» внутри карточки
pub fn decorate(ui: &mut Ui, st: &State, look: &str, style: &str, r: R, framed: bool) {
    let ac = accent(ui);
    let inset = edge_inset(style);
    match look {
        "accent" if framed => {
            // метка в левом верхнем углу, на кромке
            ui.cv.fill(R::new(r.x + inset, r.y, 28, 3), ac);
        }
        "reef" => {
            if framed && r.w > 2 * inset + 8 {
                // верхняя кромка, подсвеченная акцентом: одна сплошная линия на самой рамке (раньше была рваная в два уровня и читалась как сбой)
                ui.cv.fill(R::new(r.x + inset, r.y, r.w - 2 * inset, 1), ac.alpha(0.6));
            }
            // пузырьки всплывают над правым верхним углом, в запасе над карточкой
            let ring = |ui: &mut Ui, x: i32, y: i32, n: i32, c: C| {
                ui.cv.fill(R::new(x + 1, y, n - 2, 1), c);
                ui.cv.fill(R::new(x + 1, y + n - 1, n - 2, 1), c);
                ui.cv.fill(R::new(x, y + 1, 1, n - 2), c);
                ui.cv.fill(R::new(x + n - 1, y + 1, 1, n - 2), c);
            };
            let c = ac.alpha(0.85);
            let (bx, by) = (r.right() - 26, r.y - 8);
            ring(ui, bx, by, 6, c);
            ring(ui, bx + 8, by - 6, 5, c.alpha(0.65));
            ring(ui, bx + 15, by - 2, 4, c.alpha(0.5));
        }
        "hud" => {
            // прицел: уголки на самой рамке, по всем углам
            let l = 8;
            brackets(ui, R::new(r.x - 1, r.y - 1, r.w + 2, r.h + 2), l, 1, ac);
            let _ = st;
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_style_and_look_has_a_name_and_validates() {
        for (id, name) in STYLES.iter().chain(LOOKS.iter()) {
            assert!(!id.is_empty() && !name.is_empty());
        }
        assert!(valid_style("pixel") && !valid_style("neon"));
        assert!(valid_look("reef") && !valid_look("tape"));
    }

    #[test]
    fn the_signed_distance_is_negative_inside_zero_on_the_edge_positive_outside() {
        let r = R::new(10, 10, 100, 60);
        assert!(sdf(60.0, 40.0, r, 10.0, 0.0) < -20.0, "centre is well inside");
        assert!(sdf(10.0, 40.0, r, 10.0, 0.0).abs() < 0.01, "the left edge");
        assert!(sdf(130.0, 40.0, r, 10.0, 0.0) > 15.0, "outside");
        // у угла скруглённая форма короче прямоугольной: точка в самом углу снаружи
        assert!(sdf(10.5, 10.5, r, 10.0, 0.0) > 0.0);
    }

    #[test]
    fn the_notch_ring_covers_the_edge_without_stray_pixels() {
        // каждая строка рамки: отрезок внешней фигуры минус отрезок внутренней; в средних строках — две полосы bw, на краях — вся строка
        let r = R::new(0, 0, 40, 30);
        let (n, bw) = (3, 2);
        let inner = R::new(bw, bw, 40 - 2 * bw, 30 - 2 * bw);
        for y in 0..30 {
            let (a0, a1) = notch_span(r, n, y).unwrap();
            let (b0, b1) = notch_span(inner, n - 1, y).filter(|_| y >= bw && y < 30 - bw).unwrap_or((a0, a0));
            assert!(a0 <= b0 && b1 <= a1, "row {y}: inner span {b0}..{b1} leaves outer {a0}..{a1}");
            if y >= n + bw && y < 30 - n - bw {
                assert_eq!((b0 - a0, a1 - b1), (bw, bw), "straight rows have a border of exactly {bw} px on both sides");
            }
        }
    }
}
