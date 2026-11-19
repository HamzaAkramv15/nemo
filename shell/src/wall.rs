// Обои: своя картинка или пиксельные из палитры (nemo-shell recolor пишет wallpaper.png). Своя
// готовится в нити — декод, обрезка или вписывание под экран, клетки, окраска в палитру,
// затемнение — и приходит в Msg::Wall уже готовой: экран рисует её без пересчёта. Тот же
// результат идёт и экрану входа (login.rs → /var/lib/nemo-greet/look/wall.png). Здесь же вкладка «обои».

use crate::app::{App, Msg, SKind, State};
use crate::config::{self, Settings};
use crate::font::{self, PX};
use crate::gfx::{BoxStyle, Img, C, R};
use crate::settings::{opt_btn, title};
use crate::theme::Theme;
use crate::ui::{Act, Tone, Ui};
use image::RgbaImage;
use std::path::{Path, PathBuf};

pub const FITS: &[(&str, &str)] = &[("fill", "заполнить"), ("fit", "вписать")];
pub const TONES: &[(&str, &str)] = &[("", "как есть"), ("tint", "оттенок акцента"), ("palette", "тона палитры")];
const BAYER4: [u8; 16] = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];

/// размер клетки в пикселях экрана для wallPixel 1..8
pub fn cell(p: i32) -> u32 {
    [1, 2, 3, 4, 6, 8, 12, 16][(p.clamp(1, 8) - 1) as usize]
}

/// картинки, которые шелл декодирует сам: обои, выбор обоев, кадры skwd-wall
pub const IMAGES: &[&str] = &["png", "jpg", "jpeg", "webp"];

pub fn decodable(p: &Path) -> bool {
    p.extension().is_some_and(|e| IMAGES.contains(&e.to_string_lossy().to_ascii_lowercase().as_str()))
}

/// «как обложка»: наибольший кусок из середины картинки sw×sh с пропорциями tw×th — x, y, ширина, высота
pub fn cover_rect(sw: u32, sh: u32, tw: u32, th: u32) -> (u32, u32, u32, u32) {
    let (cw, ch) = if sw as u64 * th as u64 > sh as u64 * tw as u64 { ((sh as u64 * tw as u64 / th as u64) as u32, sh) } else { (sw, (sw as u64 * th as u64 / tw as u64) as u32) };
    let (cw, ch) = (cw.clamp(1, sw.max(1)), ch.clamp(1, sh.max(1)));
    ((sw - cw.min(sw)) / 2, (sh - ch.min(sh)) / 2, cw, ch)
}

/// своя картинка (с раскрытым ~) или None — пиксельные из палитры
pub fn own(s: &Settings) -> Option<PathBuf> {
    let p = s.wallpaper.trim();
    if p.is_empty() {
        return None;
    }
    Some(nemo_px::paths::expand(p))
}

/// что рисовать: картинка, выбранная в skwd-wall (если он включён и что-то выбрал), иначе своя
pub fn source(st: &State) -> Option<PathBuf> {
    match (&st.skwd.pick, st.set.wall_skwd) {
        (Some((p, _)), true) => Some(p.clone()),
        _ => own(&st.set),
    }
}

/// акцент берётся из картинки: «палитра из обоев», если палитру не ведёт skwd-wall
fn accent_from_image(s: &Settings) -> bool {
    s.wall_accent && !(s.wall_skwd && s.wall_skwd_colors)
}

pub fn generated() -> PathBuf {
    config::state_dir().join("wallpaper.png")
}

fn name_of(list: &[(&str, &'static str)], id: &str) -> &'static str {
    list.iter().find(|x| x.0 == id).map(|x| x.1).unwrap_or(list[0].1)
}

fn next_of(list: &[(&'static str, &str)], id: &str) -> &'static str {
    let i = list.iter().position(|x| x.0 == id).unwrap_or(0);
    list[(i + 1) % list.len()].0
}

/// всё, от чего зависит готовая картинка; одинаковый ключ — пересчитывать нечего
pub struct Job {
    path: PathBuf,
    own: bool,
    w: u32,
    h: u32,
    cell: u32,
    fit: bool,
    tone: String,
    dim: f32,
    accent: bool,
    /// схема палитры из обоев (#31): tonal | vibrant | content | muted | mono
    scheme: String,
    /// тона палитры от фона к тексту
    ramp: [u32; 6],
}

impl Job {
    /// src — картинка (wall::source) или None: пиксельные из палитры
    pub fn new(s: &Settings, t: &Theme, w: i32, h: i32, src: Option<PathBuf>) -> Job {
        let own = src;
        Job {
            own: own.is_some(),
            path: own.unwrap_or_else(generated),
            w: w.max(1) as u32,
            h: h.max(1) as u32,
            cell: cell(s.wall_pixel),
            fit: s.wall_fit == "fit",
            tone: s.wall_tone.clone(),
            dim: s.wall_dim.clamp(0.0, 0.8),
            accent: accent_from_image(s),
            scheme: s.wall_scheme.clone(),
            ramp: [t.bg.0, t.dim.0, t.blood.0, t.red.0, t.ember.0, t.text.0],
        }
    }

    pub fn key(&self) -> String {
        let m = std::fs::metadata(&self.path).and_then(|m| m.modified()).ok();
        // и «палитра из обоев»: включили её на той же картинке — акцент надо вычислить заново
        format!("{:?}|{m:?}|{}|{}x{}|{}|{}|{}|{}|{:x?}|{}|{}", self.path, self.own, self.w, self.h, self.cell, self.fit, self.tone, self.dim, self.ramp, self.accent, self.scheme)
    }
}

/// готовая картинка и (если просили) акцент из неё
pub fn prepare(j: &Job) -> (Option<Img>, Option<u32>) {
    let Ok(src) = image::open(&j.path) else { return (None, None) };
    let bg = C(j.ramp[0]);
    if !j.own {
        // сгенерированные уже в палитре и в четверть экрана: только затемнение
        let mut i = src.to_rgba8();
        drop(src);
        dim(&mut i, bg, j.dim);
        return (Some(crate::icons::from_rgba(&i)), None);
    }
    let src = src.to_rgba8();
    let accent = if j.accent { dominant(&image::imageops::thumbnail(&src, 64, 64), &j.scheme) } else { None };
    let (sw, sh) = src.dimensions();
    let (tw, th) = ((j.w / j.cell).max(1), (j.h / j.cell).max(1));
    let filt = image::imageops::FilterType::Triangle;
    let mut out = if j.fit {
        let k = (tw as f32 / sw as f32).min(th as f32 / sh as f32);
        let (rw, rh) = (((sw as f32 * k).round() as u32).clamp(1, tw), ((sh as f32 * k).round() as u32).clamp(1, th));
        let mut pic = image::imageops::resize(&src, rw, rh, filt);
        drop(src);
        tone(&mut pic, j);
        let (r, g, b) = bg.rgbf();
        let mut o = RgbaImage::from_pixel(tw, th, image::Rgba([(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, 255]));
        image::imageops::overlay(&mut o, &pic, ((tw - rw) / 2) as i64, ((th - rh) / 2) as i64);
        o
    } else {
        // «как обложка»: вырезать из середины кусок с пропорциями экрана
        let (cx, cy, cw, ch) = cover_rect(sw, sh, tw, th);
        let crop = image::imageops::crop_imm(&src, cx, cy, cw, ch);
        let mut o = image::imageops::resize(&*crop, tw, th, filt);
        drop(src);
        tone(&mut o, j);
        o
    };
    dim(&mut out, bg, j.dim);
    // большой декод уже не нужен: вернуть память системе
    nemo_px::trim_heap();
    (Some(crate::icons::from_rgba(&out)), accent)
}

fn lum(p: &image::Rgba<u8>) -> f32 {
    (0.299 * p.0[0] as f32 + 0.587 * p.0[1] as f32 + 0.114 * p.0[2] as f32) / 255.0
}

/// окраска: яркость картинки → тона палитры (растянутая между 2-м и 98-м процентилем)
fn tone(i: &mut RgbaImage, j: &Job) {
    if j.tone != "tint" && j.tone != "palette" {
        return;
    }
    let mut hist = [0u32; 256];
    for p in i.pixels() {
        hist[(lum(p) * 255.0) as usize] += 1;
    }
    let n = i.width() * i.height();
    let at = |q: f32| {
        let (mut acc, lim) = (0, (n as f32 * q) as u32);
        hist.iter().position(|c| {
            acc += c;
            acc > lim
        })
        .unwrap_or(0) as f32
            / 255.0
    };
    let (lo, hi) = (at(0.02), at(0.98));
    let span = (hi - lo).max(0.1);
    let r = j.ramp.map(C);
    let w = i.width();
    for (k, p) in i.pixels_mut().enumerate() {
        let l = ((lum(p) - lo) / span).clamp(0.0, 1.0);
        let c = if j.tone == "palette" {
            // клетка = пиксель картинки: байеровский дизеринг между соседними тонами
            let (x, y) = (k as u32 % w, k as u32 / w);
            let d = BAYER4[((y & 3) * 4 + (x & 3)) as usize] as f32 / 16.0 - 0.47;
            r[(l * 5.0 + d).round().clamp(0.0, 5.0) as usize]
        } else if l < 0.6 {
            r[0].lerp(r[3], l / 0.6)
        } else {
            r[3].lerp(r[5], (l - 0.6) / 0.4)
        };
        let (cr, cg, cb) = c.rgbf();
        p.0 = [(cr * 255.0) as u8, (cg * 255.0) as u8, (cb * 255.0) as u8, 255];
    }
}

fn dim(i: &mut RgbaImage, bg: C, k: f32) {
    if k <= 0.0 {
        return;
    }
    let (r, g, b) = bg.rgbf();
    let to = [r * 255.0, g * 255.0, b * 255.0];
    for p in i.pixels_mut() {
        for c in 0..3 {
            p.0[c] = (p.0[c] as f32 + (to[c] - p.0[c] as f32) * k).round() as u8;
        }
    }
}

/// схемы палитры из обоев (#31): id и имя
pub const SCHEMES: [(&str, &str); 5] = [("tonal", "тональная"), ("vibrant", "яркая"), ("content", "как на картинке"), ("muted", "приглушённая"), ("mono", "монохром")];

/// главный цвет картинки. Цвета картинки собираются в 16 групп (k-средних), и у каждой группы свой
/// вес: доля картинки в степени 0,65 (большая однотонная область не забивает маленькую яркую),
/// насыщенность и яркость; серые и почти чёрные группы почти ничего не весят. Берётся лучшая
/// группа вместе с соседями того же тона (±21°), и её цвет доводится до насыщенности и яркости, на
/// которых акцент читается на тёмных панелях: схема решает, насколько яркий и насыщенный он нужен.
/// Раньше весом был сумма «насыщенность² × яркость» по пикселям по 24 корзинам тона, и широкое тусклое
/// небо перевешивало малое яркое облако: на закате с розовым облаком получался синий.
/// Почти серая картинка цвета не даёт (None): акцент остаётся прежним
fn dominant(i: &RgbaImage, scheme: &str) -> Option<u32> {
    const K: usize = 16;
    let px: Vec<[f32; 3]> = i.pixels().map(|p| [p.0[0] as f32 / 255.0, p.0[1] as f32 / 255.0, p.0[2] as f32 / 255.0]).collect();
    if px.len() < K {
        return None;
    }
    // начальные центры — по яркости, ровными долями: без случайности, один и тот же ответ на одну картинку
    let lum = |c: &[f32; 3]| 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2];
    let mut order: Vec<usize> = (0..px.len()).collect();
    order.sort_by(|&a, &b| lum(&px[a]).total_cmp(&lum(&px[b])));
    let mut cen: Vec<[f32; 3]> = (0..K).map(|k| px[order[(k * 2 + 1) * px.len() / (K * 2)]]).collect();
    let mut count = vec![0usize; K];
    for _ in 0..10 {
        let mut sum = [[0f32; 3]; K];
        count = vec![0; K];
        for p in &px {
            let k = (0..K).min_by(|&a, &b| dist2(p, &cen[a]).total_cmp(&dist2(p, &cen[b]))).unwrap_or(0);
            count[k] += 1;
            for c in 0..3 {
                sum[k][c] += p[c];
            }
        }
        for k in 0..K {
            if count[k] > 0 {
                cen[k] = sum[k].map(|v| v / count[k] as f32);
            }
        }
    }
    // как схема взвешивает насыщенность: «яркая» тянется к самому насыщенному цвету
    let se = if scheme == "vibrant" { 2.0 } else { 1.2 };
    let total = px.len() as f32;
    let hsv_of = |c: &[f32; 3]| {
        let (mx, mn) = (c[0].max(c[1]).max(c[2]), c[0].min(c[1]).min(c[2]));
        (hue(c[0], c[1], c[2], mx, mn) / 360.0, if mx > 0.0 { (mx - mn) / mx } else { 0.0 }, mx)
    };
    // два прохода: сначала настоящие цвета, потом (для бледных картинок, вроде фото) мягче
    for (min_s, min_v, floor) in [(0.16f32, 0.16f32, 0.05f32), (0.08, 0.2, 0.0)] {
        let mut groups: Vec<(f32, f32, f32, f32)> = vec![]; // вес, тон (0..1), насыщенность, яркость
        let mut mass = 0f32;
        for k in 0..K {
            let (h, s, v) = hsv_of(&cen[k]);
            let p = count[k] as f32 / total;
            if p == 0.0 || v < min_v {
                continue;
            }
            if s >= 0.25 && v >= 0.3 {
                mass += p;
            }
            let mut w = p.powf(0.65) * s.powf(se) * (0.35 + 0.65 * v) * if s < min_s { floor } else { 1.0 };
            if s > 0.45 && v > 0.6 {
                w *= 1.15;
            }
            groups.push((w, h, s, v));
        }
        let Some(&(best, h0, ..)) = groups.iter().max_by(|a, b| a.0.total_cmp(&b.0)) else { continue };
        // настоящих цветов мало (почти серая картинка): на первом проходе ничего не берём, на втором — тоже
        if best <= 0.0 || (min_s > 0.1 && mass < 0.05) {
            continue;
        }
        if min_s <= 0.1 && !groups.iter().any(|g| g.2 >= 0.1) {
            return None;
        }
        let (mut wsum, mut hx, mut hy, mut ss, mut vs) = (0f32, 0f32, 0f32, 0f32, 0f32);
        for &(w, h, s, v) in &groups {
            let d = (h - h0).abs();
            if d.min(1.0 - d) <= 0.06 {
                wsum += w;
                hx += (h * std::f32::consts::TAU).cos() * w;
                hy += (h * std::f32::consts::TAU).sin() * w;
                ss += s * w;
                vs += v * w;
            }
        }
        let h = hy.atan2(hx).rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * 360.0;
        let (s0, v0) = (ss / wsum, vs / wsum);
        let (s, v) = match scheme {
            "vibrant" => (s0.clamp(0.8, 1.0), v0.clamp(0.85, 0.96)),
            "content" => (s0.clamp(0.3, 0.95), v0.clamp(0.6, 0.95)),
            "muted" => (s0.clamp(0.22, 0.38), v0.clamp(0.66, 0.82)),
            "mono" => (0.06, v0.clamp(0.7, 0.85)),
            _ => (s0.clamp(0.5, 0.85), v0.clamp(0.74, 0.95)),
        };
        return Some(hsv(h, s, v));
    }
    None
}

fn dist2(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)
}

fn hue(r: f32, g: f32, b: f32, mx: f32, mn: f32) -> f32 {
    let d = (mx - mn).max(1e-6);
    let h = if mx == r { (g - b) / d } else if mx == g { 2.0 + (b - r) / d } else { 4.0 + (r - g) / d };
    (h * 60.0).rem_euclid(360.0)
}

fn hsv(h: f32, s: f32, v: f32) -> u32 {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let (r, g, b) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    C::from_f(r + v - c, g + v - c, b + v - c).0 & 0xffffff
}

/// длинная сторона копии обоев, что остаётся в памяти (wall_park)
const PARK: i32 = 640;

/// уменьшение в целое число раз средним по квадрату k×k
fn shrink(img: &Img, max: i32) -> Img {
    if img.w <= 0 || img.h <= 0 || img.px.len() < (img.w * img.h) as usize {
        return Img { w: 1, h: 1, px: vec![0xff00_0000] };
    }
    let k = ((img.w.max(img.h) + max - 1) / max).max(1);
    // по узкой стороне квадрат не шире самой картинки (полоса 4000×3 не выйдет за буфер)
    let (kx, ky) = (k.min(img.w), k.min(img.h));
    let (w, h) = (img.w / kx, img.h / ky);
    let n = (kx * ky) as u32;
    let mut px = Vec::with_capacity((w * h) as usize);
    for y in 0..h {
        for x in 0..w {
            let [mut a, mut r, mut g, mut b] = [0u32; 4];
            for j in 0..ky {
                let row = ((y * ky + j) * img.w + x * kx) as usize;
                for &p in &img.px[row..row + kx as usize] {
                    a += p >> 24;
                    r += (p >> 16) & 255;
                    g += (p >> 8) & 255;
                    b += p & 255;
                }
            }
            px.push((a / n) << 24 | (r / n) << 16 | (g / n) << 8 | (b / n));
        }
    }
    Img { w, h, px }
}

/// обои на весь экран «как обложка» (картинка уже в пропорциях экрана — без обрезки)
pub fn cover(ui: &mut Ui, img: &Img, w: i32, h: i32) {
    nemo_px::face::cover(ui, img, w, h);
}

pub fn draw(ui: &mut Ui, st: &State, w: i32, h: i32) {
    let t = ui.t;
    ui.cv.fill(R::new(0, 0, w, h), t.bg);
    if let Some(img) = &st.wall {
        cover(ui, img, w, h);
    }
    if st.set.wall_kanji {
        // знак Nemo в правом нижнем углу: рыбка и слово (раньше здесь был иероглиф 闇); ключ
        // wallKanji остался как был, чтобы настройки не потерялись
        let face = font::px(32);
        let word = nemo_px::brand::NAME.to_lowercase();
        let (tw, th) = (ui.w(face, &word), ui.f.line(face));
        let (fw, fh) = nemo_px::brand::fish_size(3);
        let (x, y) = (w - 40 - tw - 14 - fw, h - 32 - th.max(fh));
        nemo_px::brand::fish(ui, x, y + (th.max(fh) - fh) / 2, 3, 0.55);
        ui.text(face, x + fw + 14, y + (th.max(fh) - th) / 2, &word, t.red.fade(0.45));
    }
}

// ── вкладка «обои» ──────────────────────────────────────────────────────────
/// превью в пропорциях экрана: то, что сейчас на рабочем столе
pub(crate) fn preview(ui: &mut Ui, st: &State, pv: R) {
    let t = ui.t;
    ui.cv.pxbox(pv, &BoxStyle::new(t.code_bg, t.border));
    let inner = pv.inset(3);
    ui.cv.push_clip(inner);
    ui.cv.fill(inner, t.bg);
    if let Some(img) = &st.wall {
        let k = (inner.w as f32 / img.w as f32).max(inner.h as f32 / img.h as f32);
        let (dw, dh) = ((img.w as f32 * k).ceil() as i32, (img.h as f32 * k).ceil() as i32);
        ui.cv.image_scaled(img, R::new(inner.x + (inner.w - dw) / 2, inner.y + (inner.h - dh) / 2, dw, dh), 1.0);
    }
    ui.cv.pop_clip();
}

pub fn tab(ui: &mut Ui, st: &State, b: R) {
    let s = &st.set;
    let t = ui.t;
    let half = (b.w - 20) / 2;
    let mut y = title(ui, b.x, b.y, b.w, "картинка");
    let pv = R::new(b.x, y, 256, 144);
    preview(ui, st, pv);
    let (rx, rw) = (pv.right() + 16, b.right() - pv.right() - 16);
    let mut ry = y + 2;
    let skwd = s.wall_skwd.then_some(st.skwd.pick.as_ref()).flatten();
    match source(st) {
        Some(p) => {
            // имя файла — как есть, без перевода
            let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let f = &mut *ui.f;
            ui.cv.push_clip(R::new(rx, ry, rw, 20));
            ui.cv.text(f, PX, rx, ry, &name, t.text);
            ui.cv.pop_clip();
            ry += 20;
            let what = match skwd.map(|k| k.1.as_str()) {
                Some("video") => "skwd-wall: видео, здесь — кадр",
                Some("we") => "skwd-wall: сцена, здесь — кадр",
                Some(_) => "выбрано в skwd-wall",
                None if p.is_file() => "своя картинка",
                None => "файл не найден",
            };
            ui.text_in(PX, R::new(rx, ry, rw, 18), what, if p.is_file() { t.text_faint } else { t.warn });
        }
        None => {
            ui.text_in(PX, R::new(rx, ry, rw, 20), "пиксельные из палитры", t.text);
            ry += 20;
            ui.text_in(PX, R::new(rx, ry, rw, 18), "рисуются заново при смене цвета", t.text_faint);
        }
    }
    ry += 30;
    ui.button(R::new(rx, ry, rw, 28), "выбрать картинку…", "folder", Tone::Primary, false, true, Act::SWall("pick"));
    ry += 34;
    ui.button(R::new(rx, ry, rw, 28), "пиксельные из палитры", "image", Tone::Plain, false, own(s).is_some(), Act::SWall("reset"));
    y = pv.bottom() + 10;
    let focused = st.field_focus == Some("wall");
    ui.field(R::new(b.x, y, b.w - 130, 30), &st.wall_field, "или путь к файлу или папке", focused, false, Act::SField("wall"));
    ui.button(R::new(b.right() - 120, y + 2, 120, 26), "применить", "check", Tone::Primary, false, true, Act::SCycle("wall_apply"));
    y += 40;
    // папка обоев (wallDir): из неё сетка выбора; пусто — находится сама (walls.rs dir). Путь — как
    // есть, без перевода; выбрать — окно nemo-files, без него папку вписывают в поле выше
    let auto = s.wall_dir.trim().is_empty();
    ui.cv.icon("folder", b.x + 2, y + 9, 2, t.text_faint);
    let lw = ui.text_in(PX, R::new(b.x + 22, y, 160, 30), "папка", t.text_faint);
    // кнопки — по ширине надписей (португальские длиннее)
    let (pw, aw) = (ui.w(PX, "выбрать папку…") + 44, ui.w(PX, "авто") + 44);
    let bw = if auto { pw } else { pw + 8 + aw };
    let px0 = b.x + 22 + lw + 12;
    let pr = R::new(px0, y, b.right() - bw - 12 - px0, 30);
    if auto {
        ui.text_in(PX, pr, "авто: Pictures/Wallpapers", t.text_dim);
    } else {
        ui.cv.push_clip(pr);
        ui.cv.text_in(ui.f, PX, pr, &nemo_px::paths::pretty(&nemo_px::paths::expand(&s.wall_dir)), t.text);
        ui.cv.pop_clip();
    }
    let mut bx = b.right() - bw;
    if !auto {
        ui.button(R::new(bx, y + 2, aw, 26), "авто", "reboot", Tone::Ghost, false, true, Act::SWall("dir_auto"));
        bx += aw + 8;
    }
    ui.button(R::new(bx, y + 2, pw, 26), "выбрать папку…", "folder", Tone::Plain, false, st.mods_here.files, Act::SWall("dir"));
    y += 44;
    y = title(ui, b.x, y, b.w, "как показывать");
    opt_btn(ui, R::new(b.x, y, half, 26), "подгонка", name_of(FITS, &s.wall_fit), Act::SWall("fit"));
    opt_btn(ui, R::new(b.x + half + 20, y, half, 26), "окраска", name_of(TONES, &s.wall_tone), Act::SWall("tone"));
    y += 38;
    let c = cell(s.wall_pixel);
    let ct = if c == 1 { crate::i18n::tr("как есть").to_string() } else { format!("{c} px") };
    ui.slider(R::new(b.x, y, half, 40), "пиксельные клетки", (s.wall_pixel as f32 - 1.0) / 7.0, &ct, 7, Act::SSlider("wall_pixel", 1.0, 8.0, 7));
    ui.slider(R::new(b.x + half + 20, y, half, 40), "затемнение", s.wall_dim / 0.8, &format!("{:.0}%", s.wall_dim * 100.0), 16, Act::SSlider("wall_dim", 0.0, 0.8, 16));
    y += 54;
    // skwd-wall: раздел виден, когда он стоит (или уже включён)
    let has_skwd = s.wall_skwd || st.skwd_here;
    y = title(ui, b.x, y, b.w, if has_skwd { "цвета и skwd-wall" } else { "цвета" });
    let (lx, rx2) = (b.x, b.x + half + 20);
    let ph = if s.wall_skwd && s.wall_skwd_colors { "палитру ведёт skwd-wall" } else { "акцент из картинки" };
    ui.toggle(R::new(lx, y, half, 40), "палитра из обоев", ph, s.wall_accent, Act::SWall("accent"));
    ui.toggle(R::new(rx2, y, half, 40), "знак Nemo на обоях", "в правом нижнем углу", s.wall_kanji, Act::SBool("wall_kanji"));
    y += 46;
    if has_skwd {
        // кто рисует: skwd-paper (видео, сцены, переходы) или шелл (картинка в клетках и тонах)
        let hint = if !s.wall_skwd {
            "выбор и видео-обои — в skwd-wall"
        } else if st.skwd.pick.is_none() {
            "ждёт выбора в skwd-wall"
        } else if st.skwd.drawn.is_empty() {
            "картинку рисует шелл"
        } else {
            "рисует skwd-paper"
        };
        ui.toggle(R::new(lx, y, half, 40), "обои из skwd-wall", hint, s.wall_skwd, Act::SBool("wall_skwd"));
        if s.wall_skwd {
            ui.toggle(R::new(rx2, y, half, 40), "его палитра", "акцент и светлая / тёмная тема", s.wall_skwd_colors, Act::SBool("wall_skwd_colors"));
        }
    }
}

impl App {
    /// пересчитать обои, если изменилось то, от чего они зависят (настройки, палитра, файл,
    /// размер экрана). Пока тянут ползунок обоев — ждём, когда отпустят.
    pub fn sync_wall(&mut self) {
        if matches!(&self.press, Some(p) if matches!(p.act, Act::SSlider(k, ..) if k.starts_with("wall"))) {
            return;
        }
        if self.wall_kanji != self.st.set.wall_kanji {
            self.wall_kanji = self.st.set.wall_kanji;
            self.dirty(&SKind::Wall);
        }
        let (w, h) = self.output_size(&self.focus_out());
        let job = Job::new(&self.st.set, &self.st.t, w, h, source(&self.st));
        let key = job.key();
        // и обои экрана входа: сменили их вид — пересчитать, даже если стол прежний
        let lmode = crate::login::wall_mode(&self.st.set);
        let full = format!("{key}|{lmode}");
        if full == self.wall_key {
            return;
        }
        self.wall_key = full;
        self.wall_job += 1;
        let (n, tx) = (self.wall_job, self.tx.clone());
        let ljob = (lmode == "pixel").then(|| Job::new(&self.st.set, &self.st.t, w, h, None));
        std::thread::spawn(move || {
            let (img, accent) = prepare(&job);
            // экрану входа — та же картинка (или своя: greetWall); PNG пишется уже после отправки
            // картинки шеллу
            let login = if lmode == "desktop" && crate::login::wanted(&key) { img.clone() } else { None };
            let _ = tx.send(Msg::Wall(n, img, accent));
            if let Some(i) = login {
                crate::login::wall(&i, &key);
            } else if lmode != "desktop" {
                crate::login::wall_other(ljob);
            }
        });
    }

    pub fn wall_ready(&mut self, n: u64, img: Option<Img>, accent: Option<u32>) {
        if n != self.wall_job {
            return;
        }
        self.st.wall = img.map(std::rc::Rc::new);
        self.st.wall_parked = false;
        self.wall_reload = false;
        self.wall_at = std::time::Instant::now();
        self.st.wall_stamp += 1;
        for s in self.surfs.values_mut() {
            if matches!(s.kind, SKind::Wall | SKind::Curtain) {
                s.dirty = true;
                s.frame_pending = false;
            }
        }
        for l in self.lock_surfs.iter_mut() {
            l.dirty = true;
        }
        self.dirty(&SKind::Modal);
        // новые обои: виджеты на них — в тон
        self.widget_contrast();
        let s = &mut self.st.set;
        if let (Some(a), true) = (accent, accent_from_image(s)) {
            let hex = format!("#{a:06x}");
            if hex != s.accent {
                s.accent = hex.clone();
                self.st.hex.set(&hex);
                self.settings_changed();
            }
        }
    }

    /// обои уже у niri: держать полную картинку незачем (1080p — 8 МБ, 4K — 33 МБ). Остаётся копия
    /// не больше PARK точек по длинной стороне: превью во вкладке, замер под виджетами, первый кадр
    /// блокировки и шторки. Полную пересчитывает wall_need, когда она снова нужна
    pub fn wall_park(&mut self) {
        let Some(img) = self.st.wall.as_ref() else { return };
        if self.st.wall_parked || self.st.locked || self.st.curtain.is_some() || img.w.max(img.h) <= PARK {
            return;
        }
        // пока обои не показаны (буфер ещё свой) или их только что сменили — рано
        if self.wall_at.elapsed().as_secs() < 20 || self.surfs.values().any(|s| s.kind == SKind::Wall && (s.dirty || s.pool.is_some())) {
            return;
        }
        let small = shrink(img, PARK);
        self.st.wall = Some(std::rc::Rc::new(small));
        self.st.wall_parked = true;
        nemo_px::trim_heap();
    }

    /// полная картинка снова нужна (обои перерисовать, блокировка, шторка): пересчитать её в нити.
    /// Экран входа не трогается — ключ тот же
    pub fn wall_need(&mut self) {
        if !self.st.wall_parked || self.wall_reload {
            return;
        }
        self.wall_reload = true;
        self.wall_key.clear();
        self.sync_wall();
    }

    /// новый путь (пусто — пиксельные из палитры): из настроек, IPC и диалога выбора
    pub fn set_wall(&mut self, p: &str) {
        let p = p.trim();
        let p = if p.is_empty() { String::new() } else { nemo_px::paths::expand(p).to_string_lossy().to_string() };
        self.st.set.wallpaper = p.clone();
        self.st.wall_field.set(&p);
        self.st.field_focus = None;
        self.settings_changed();
    }

    pub fn wall_click(&mut self, a: &Act) -> bool {
        let Act::SWall(k) = a else { return false };
        let s = &mut self.st.set;
        match *k {
            // сетка картинок папки обоев (walls.rs); другой файл и папка — оттуда через nemo-files
            "pick" => {
                self.walls_open(true);
                return true;
            }
            // из панели управления: выбор обоев сам по себе, без возврата в настройки
            "browse" => {
                self.close_popup();
                self.walls_open(false);
                return true;
            }
            "reset" => {
                self.set_wall("");
                return true;
            }
            // папка обоев: выбрать в окне nemo-files или вернуть «авто»; сохранится само (flush)
            "dir" => {
                self.wall_dir_ask();
                return true;
            }
            // из выбора обоев: фото без клеток
            "cells_off" => {
                s.wall_pixel = 1;
                self.settings_changed();
                self.dirty(&SKind::Modal);
                return true;
            }
            "dir_auto" => {
                s.wall_dir.clear();
                self.dirty(&SKind::Modal);
                return true;
            }
            "fit" => s.wall_fit = next_of(FITS, &s.wall_fit).into(),
            "tone" => s.wall_tone = next_of(TONES, &s.wall_tone).into(),
            "accent" => s.wall_accent = !s.wall_accent,
            "scheme" => s.wall_scheme = next_of(&SCHEMES, &s.wall_scheme).into(),
            _ => return false,
        }
        self.settings_changed();
        true
    }
}

#[cfg(test)]
mod tests {
    use nemo_px::gfx::Img;
    #[test]
    fn shrink_odd_sizes() {
        for (w, h) in [(4000, 3), (3, 4000), (1, 1), (1920, 1080), (641, 2)] {
            let img = Img { w, h, px: vec![0xff10_2030; (w * h) as usize] };
            let s = super::shrink(&img, 640);
            assert!(s.w >= 1 && s.h >= 1 && s.w <= 640 && s.h <= 640 && s.px.len() == (s.w * s.h) as usize);
            assert_eq!(s.px[0], 0xff10_2030);
        }
    }

    /// картинка из долей: (доля, цвет); остальное — почти чёрное
    fn pic(parts: &[(f32, [u8; 3])]) -> image::RgbaImage {
        let n = 64usize;
        let mut img = image::RgbaImage::from_pixel(n as u32, n as u32, image::Rgba([8, 8, 10, 255]));
        let mut at = 0usize;
        for (share, c) in parts {
            let cells = (share * (n * n) as f32) as usize;
            for k in at..(at + cells).min(n * n) {
                img.put_pixel((k % n) as u32, (k / n) as u32, image::Rgba([c[0], c[1], c[2], 255]));
            }
            at += cells;
        }
        img
    }

    /// тон результата в градусах
    fn hue_of(c: u32) -> f32 {
        let (r, g, b) = (((c >> 16) & 255) as f32 / 255.0, ((c >> 8) & 255) as f32 / 255.0, (c & 255) as f32 / 255.0);
        let (mx, mn) = (r.max(g).max(b), r.min(g).min(b));
        super::hue(r, g, b, mx, mn)
    }

    /// закат с розовым облаком: широкое тусклое сине-серое небо не должно перевесить малое яркое облако
    /// (раньше акцентом получался синий)
    #[test]
    fn a_small_vivid_cloud_beats_a_wide_dull_sky() {
        let img = pic(&[(0.45, [88, 106, 140]), (0.14, [242, 112, 150]), (0.20, [120, 90, 150])]);
        for scheme in ["tonal", "content", "vibrant"] {
            let h = hue_of(super::dominant(&img, scheme).expect("a colour"));
            assert!(!(150.0..290.0).contains(&h), "{scheme}: hue {h} is the dull sky's blue, not the cloud");
        }
    }

    #[test]
    fn plain_colours_stay_themselves() {
        let red = hue_of(super::dominant(&pic(&[(0.9, [200, 20, 50])]), "tonal").unwrap());
        assert!(red < 12.0 || red > 340.0, "red came out as {red}");
        let green = hue_of(super::dominant(&pic(&[(0.9, [60, 180, 80])]), "tonal").unwrap());
        assert!((100.0..150.0).contains(&green), "green came out as {green}");
    }

    /// серое — цвета нет; бледное, но цветное (кожа, песок) — цвет есть
    #[test]
    fn grey_gives_nothing_and_soft_colours_still_give_something() {
        assert!(super::dominant(&pic(&[(0.5, [120, 120, 120]), (0.5, [200, 200, 200])]), "tonal").is_none());
        assert!(super::dominant(&pic(&[(0.0, [0, 0, 0])]), "tonal").is_none());
        let skin = super::dominant(&pic(&[(0.9, [190, 160, 140])]), "tonal").expect("soft colour");
        let h = hue_of(skin);
        assert!((10.0..45.0).contains(&h), "skin tone came out as hue {h}");
    }

    /// один ответ на одну картинку, и схема действительно меняет насыщенность
    #[test]
    fn the_answer_is_stable_and_the_scheme_matters() {
        let img = pic(&[(0.4, [60, 130, 200]), (0.2, [220, 90, 60])]);
        assert_eq!(super::dominant(&img, "tonal"), super::dominant(&img, "tonal"));
        let sat = |c: u32| {
            let (r, g, b) = (((c >> 16) & 255) as f32, ((c >> 8) & 255) as f32, (c & 255) as f32);
            (r.max(g).max(b) - r.min(g).min(b)) / r.max(g).max(b)
        };
        assert!(sat(super::dominant(&img, "muted").unwrap()) < sat(super::dominant(&img, "vibrant").unwrap()));
        assert!(sat(super::dominant(&img, "mono").unwrap()) < 0.12);
    }

    /// `NEMO_TEST_WALL=/path/a.png cargo test -p nemo-shell live_wallpaper -- --ignored --nocapture`: что выбрано для настоящей картинки
    #[test]
    #[ignore]
    fn live_wallpaper() {
        let Ok(path) = std::env::var("NEMO_TEST_WALL") else { return };
        for p in path.split(':') {
            let src = image::open(p).expect("image").to_rgba8();
            let small = image::imageops::thumbnail(&src, 64, 64);
            let show = |s: &str| super::dominant(&small, s).map(|c| format!("#{c:06x}")).unwrap_or_else(|| "none".into());
            println!("{p}: tonal {} vibrant {} content {} muted {}", show("tonal"), show("vibrant"), show("content"), show("muted"));
        }
    }
}
