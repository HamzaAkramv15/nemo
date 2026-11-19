// Уведомления: карточки справа под баром, лентой. ЛКМ — действие по
// умолчанию, × — убрать, наведение — не гаснет, срочные висят. Все — в истории (календарь).
// Тут же всплывашка громкости и раскладки (OSD).

use crate::app::{App, SKind, State};
use crate::clock::local_now;
use crate::font::{self, PX};
use crate::gfx::{BoxStyle, Img, R};
use crate::ui::{Act, Cur, Tone, Ui};
use smithay_client_toolkit::shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer};
use smithay_client_toolkit::shell::WaylandSurface;
use std::rc::Rc;
use std::time::Instant;

pub use crate::dbus::Incoming;

pub const W: i32 = 370;
/// самая большая картинка в карточке (превью скриншота); больше — уменьшается при отрисовке
pub const IMG_W: i32 = 112;
pub const IMG_H: i32 = 72;

/// место под картинку уведомления: её размер, вписанный в IMG_W×IMG_H
fn img_size(img: &Img) -> (i32, i32) {
    let (w, h) = (img.w.max(1), img.h.max(1));
    if w <= IMG_W && h <= IMG_H {
        return (w, h);
    }
    let k = (IMG_W as f32 / w as f32).min(IMG_H as f32 / h as f32);
    (((w as f32 * k).round() as i32).max(1), ((h as f32 * k).round() as i32).max(1))
}

pub struct Note {
    pub id: u32,
    pub app: String,
    pub icon: String,
    pub summary: String,
    pub body: String,
    pub actions: Vec<(String, String)>,
    pub critical: bool,
    pub image: Option<Rc<Img>>,
    pub born: Instant,
    pub time: String,
    pub life: f32,
    pub left: f32,
}

#[derive(Default)]
pub struct Notifs {
    pub popups: Vec<u32>,
    pub history: Vec<Note>,
    pub hover: bool,
    last: Option<Instant>,
    /// карточки, которые уходят (анимация закрытия): id и с какого мгновения
    pub leaving: Vec<(u32, Instant)>,
    /// сколько длится уход, с; 0 — карточка пропадает сразу (prefs: настройки → анимации)
    pub leave_s: f32,
}

impl Notifs {
    pub fn get(&self, id: u32) -> Option<&Note> {
        self.history.iter().find(|n| n.id == id)
    }
    pub fn remove(&mut self, id: u32, from_history: bool) {
        // из истории — сразу (рисовать уже нечего), иначе карточка сперва уходит
        if self.leave_s > 0.0 && !from_history && self.popups.contains(&id) {
            if !self.leaving.iter().any(|x| x.0 == id) {
                self.leaving.push((id, Instant::now()));
            }
        } else {
            self.popups.retain(|x| *x != id);
            self.leaving.retain(|x| x.0 != id);
        }
        if from_history {
            self.history.retain(|n| n.id != id);
        }
    }
    /// с какого мгновения карточка уходит
    pub fn leaving_since(&self, id: u32) -> Option<Instant> {
        self.leaving.iter().find(|x| x.0 == id).map(|x| x.1)
    }
    pub fn resume(&mut self) {
        self.last = Some(Instant::now());
    }
}

/// сколько уходит карточка, с (0 — сразу): закрытие группы «уведомления»
pub fn leave_s(s: &crate::config::Settings) -> f32 {
    let on = s.anim() && s.anim_close && s.reveal_for("notifs", true) != "none";
    if on { s.anim_ms("notifs") * 0.7 / 1000.0 } else { 0.0 }
}

/// высота карточки
fn card_h(ui_f: &mut font::Fonts, n: &Note) -> i32 {
    let (iw, ih) = n.image.as_deref().map(img_size).unwrap_or((0, 0));
    let tw = W - 24 - if iw > 0 { iw + 10 } else { 0 };
    let mut h = 18 + 6;
    let mut body = ui_f.wrap_height(PX, &n.summary, tw, 2);
    if !n.body.is_empty() {
        body += 3 + ui_f.wrap_height(PX, &n.body, tw, 4);
    }
    h += body.max(ih);
    if n.actions.iter().any(|a| a.0 != "default") {
        h += 6 + 22;
    }
    h + 22
}

pub fn size(st: &State, f: &mut font::Fonts) -> (i32, i32) {
    let mut h = 0;
    for id in &st.notifs.popups {
        if let Some(n) = st.notifs.get(*id) {
            h += card_h(f, n) + 8;
        }
    }
    (if st.set.island { W + 16 } else { W + 56 }, h.max(1) + 8)
}

pub fn draw(ui: &mut Ui, st: &State, w: i32, _h: i32) {
    let t = ui.t;
    let mut y = 0;
    for id in &st.notifs.popups {
        let Some(n) = st.notifs.get(*id) else { continue };
        let ch = card_h(ui.f, n);
        // въезд справа ступенями по 8 px и проявление узором группы «уведомления»; уход — наоборот
        let leaving = st.notifs.leaving_since(n.id);
        let style = st.set.reveal_for("notifs", leaving.is_some());
        let k = if !st.set.anim() || style == "none" {
            if leaving.is_some() { 0.0 } else { 1.0 }
        } else if let Some(at) = leaving {
            let t = (at.elapsed().as_secs_f32() / st.notifs.leave_s.max(0.01)).min(1.0);
            1.0 - t * t
        } else {
            (n.born.elapsed().as_secs_f32() * 1000.0 / st.set.anim_ms("notifs")).min(1.0)
        };
        let step = (k * 4.0).ceil() / 4.0;
        if k < 1.0 {
            ui.anim = true;
        }
        let moves = st.set.anims.get("notifs").is_none_or(|a| a.motion);
        let x0 = if moves { 48 - ((step * 48.0 / 8.0).round() as i32) * 8 + 4 } else { 4 };
        let pixel = !matches!(style, "fade" | "none");
        ui.cv.opacity = if pixel { (0.35 + 0.65 * k).min(1.0) } else { step };
        let z0 = ui.zones.len();
        let island = st.set.island;
        // остров: по содержимому и по центру поверхности; появляется из середины, а не въезжает сбоку
        let cw = if island { island_w(ui.f, n) } else { W };
        let e = 1.0 - (1.0 - k) * (1.0 - k);
        let r = if island {
            let rw = (cw as f32 * (0.55 + 0.45 * e)).round() as i32;
            R::new((w - rw) / 2, y - ((1.0 - e) * 10.0).round() as i32, rw, ch)
        } else {
            R::new(x0, y, W, ch)
        };
        let hov = ui.hover(r);
        let edge = if n.critical { t.ember } else if hov { t.red } else { t.border };
        if island {
            pill(ui, r, (ch / 2).min(22), t.glass_panel(), edge);
        } else {
            ui.cv.pxbox(r, &BoxStyle::new(t.glass_panel(), edge).shadow(4));
            ui.cv.fill(R::new(r.x + 2, r.y + 2, 3, r.h - 4), if n.critical { t.ember } else { t.red });
        }
        if !n.critical && n.life > 0.0 {
            // остаток — по часам, а не по тику (100 мс): полоса идёт шагами по 2 px каждый кадр;
            // под курсором время стоит, и перерисовывать нечего
            let ticking = !st.notifs.hover;
            let left = n.left - if ticking { st.notifs.last.map(|l| l.elapsed().as_secs_f32()).unwrap_or(0.0) } else { 0.0 };
            let lw = (((r.w - if island { 28 } else { 9 }) as f32 * (left / n.life).clamp(0.0, 1.0)) / 2.0).round() as i32 * 2;
            ui.cv.fill(R::new(r.x + if island { 14 } else { 5 }, r.bottom() - 4, lw.min(r.w - if island { 28 } else { 9 }), 2), t.red.fade(0.6));
            if ticking {
                ui.anim = true;
            }
        }
        ui.zone(r, Act::NotifClick(n.id), Cur::Pointer);
        if island {
            // содержимое — в полном размере, открывается из середины плашки
            ui.cv.push_clip(r);
            let full_x = (w - cw) / 2;
            // ширина как у карточки (cw - 24): перенос строк тот же, по которому посчитана высота
            draw_card(ui, n, R::new(full_x + 16, r.y + 8, cw - 24, r.h - 16), true);
            ui.cv.pop_clip();
        } else {
            draw_card(ui, n, R::new(r.x + 14, r.y + 8, r.w - 24, r.h - 16), true);
        }
        if pixel && k < 1.0 && !island {
            ui.cv.dissolve_rect_as(R::new(r.x - 4, r.y - 4, r.w + 12, r.h + 12), k, 2, yami_px::gfx::Reveal::from_id(style), st.set.reveal_grain);
        }
        // уходящая карточка уже не ловит нажатий
        if leaving.is_some() {
            ui.zones.truncate(z0);
        }
        y += ch + 8;
    }
    ui.cv.opacity = 1.0;
}

/// содержимое карточки (и в истории внутри календаря)
pub fn draw_card(ui: &mut Ui, n: &Note, r: R, close: bool) {
    let t = ui.t;
    let mut x = r.x;
    if !n.icon.is_empty() {
        let icon = n.icon.clone();
        ui.app_icon(&icon, x, r.y + 1, 16, 1.0);
        x += 22;
    }
    let app = if n.app.is_empty() { "система".to_string() } else { n.app.to_lowercase() };
    let aw = ui.text_in(PX, R::new(x, r.y, r.w / 2, 18), &app, if n.critical { t.ember } else { t.red });
    ui.text_in(PX, R::new(x + aw + 8, r.y, 60, 18), &n.time, t.text_faint);
    if close {
        ui.button(R::new(r.right() - 18, r.y, 18, 18), "", "close", Tone::Ghost, false, true, Act::NotifDel(n.id));
    }
    let mut y = r.y + 24;
    let mut tx = r.x;
    let (iw, ih) = n.image.as_deref().map(img_size).unwrap_or((0, 0));
    if let Some(img) = &n.image {
        if (iw, ih) == (img.w, img.h) {
            ui.cv.image(img, r.x, y, 1.0);
        } else {
            ui.cv.image_scaled(img, R::new(r.x, y, iw, ih), 1.0);
        }
        tx += iw + 10;
    }
    let tw = r.right() - tx;
    y += ui.wrap(PX, R::new(tx, y, tw, 40), &n.summary, t.text, 2);
    if !n.body.is_empty() {
        y += 3;
        y += ui.wrap(PX, R::new(tx, y, tw, 80), &n.body, t.text_dim, 4);
    }
    y = y.max(r.y + 24 + ih);
    let acts: Vec<&(String, String)> = n.actions.iter().filter(|a| a.0 != "default").collect();
    if !acts.is_empty() {
        y += 6;
        let mut ax = r.x;
        for (k, label) in acts {
            let bw = ui.w(PX, label) + 20;
            ui.button(R::new(ax, y, bw, 22), label, "", Tone::Plain, false, true, Act::NotifAct(n.id, k.clone()));
            ax += bw + 6;
        }
    }
}

/// левый край острова шириной `sw` с центром в `cx` на экране шириной `ow`: не выходит за экран
pub fn island_left_of(cx: i32, sw: i32, ow: i32) -> i32 {
    (cx - sw / 2).clamp(0, (ow - sw).max(0))
}

/// ширина карточки-острова: по самому длинному, что в ней есть (шапка, строки текста, кнопки, картинка), не уже
/// 260 и не шире W; перенос строк считается по полной ширине, так что сужение его не меняет
fn island_w(f: &mut font::Fonts, n: &Note) -> i32 {
    let (iw, _) = n.image.as_deref().map(img_size).unwrap_or((0, 0));
    let img = if iw > 0 { iw + 10 } else { 0 };
    let tw = W - 24 - img;
    let app = if n.app.is_empty() { "система".to_string() } else { n.app.to_lowercase() };
    let head = if n.icon.is_empty() { 0 } else { 22 } + f.width(PX, &app) + 8 + f.width(PX, &n.time).min(60) + 24;
    let mut text = 0;
    for l in f.wrap(PX, &n.summary, tw, 2) {
        text = text.max(f.width(PX, &l));
    }
    for l in f.wrap(PX, &n.body, tw, 4) {
        text = text.max(f.width(PX, &l));
    }
    let acts: i32 = n.actions.iter().filter(|a| a.0 != "default").map(|a| f.width(PX, &a.1) + 26).sum();
    ((text + img).max(head).max(acts) + 32).clamp(260, W)
}

/// ширина острова громкости/яркости/раскладки/шторки
fn island_osd_w(f: &mut font::Fonts, st: &State) -> i32 {
    let kind = st.osd.map(|o| o.0).unwrap_or("volume");
    match kind {
        "curtain" => {
            let on = st.curtain.is_some();
            let a = f.width(PX, crate::i18n::tr(if on { "экран скрыт в трансляции" } else { "экран снова виден" }));
            let b = f.width(PX, crate::i18n::tr(if on { "зрители видят шторку, ты — экран" } else { "трансляция показывает рабочий стол" }));
            (a.max(b) + 60).clamp(260, 300)
        }
        "volume" | "bright" => 260,
        _ => {
            let name = st.niri.kb_names.get(st.niri.kb_idx).cloned().unwrap_or_default();
            (f.width(PX, &name) + 90).clamp(220, 300)
        }
    }
}

/// закруглённая плашка «острова»: строки с отступом по окружности радиуса `rad` (ступени в пиксель, как всё
/// в Nemo), заливка и тонкая кромка
pub fn pill(ui: &mut Ui, r: R, rad: i32, fill: crate::gfx::C, edge: crate::gfx::C) {
    let spans = pill_spans(r.h, rad);
    for (dy, (a, b)) in spans.iter().enumerate() {
        let y = r.y + dy as i32;
        ui.cv.fill(R::new(r.x + a, y, r.w - a - b, 1), fill);
    }
    for (dy, (a, b)) in spans.iter().enumerate() {
        let y = r.y + dy as i32;
        let top = dy < spans.len() / 2;
        // кромка: на первой и последней строке — вся длина, дальше — ровно столько, сколько ступенька
        if dy == 0 || dy + 1 == spans.len() {
            ui.cv.fill(R::new(r.x + a, y, r.w - a - b, 1), edge);
            continue;
        }
        let near = if top { spans[dy - 1] } else { spans[dy + 1] };
        let (w_l, w_r) = (((near.0 - a).max(1)), ((near.1 - b).max(1)));
        ui.cv.fill(R::new(r.x + a, y, w_l, 1), edge);
        ui.cv.fill(R::new(r.right() - b - w_r, y, w_r, 1), edge);
    }
}

/// отступы слева и справа для каждой строки плашки высотой `h` с радиусом `rad`
pub fn pill_spans(h: i32, rad: i32) -> Vec<(i32, i32)> {
    let rad = rad.clamp(0, h / 2);
    (0..h)
        .map(|dy| {
            let d = if dy < rad { rad - dy } else if dy >= h - rad { dy - (h - rad - 1) } else { 0 };
            let inset = if d == 0 { 0 } else { rad - (((rad * rad - (d - 1) * (d - 1)) as f32).sqrt().floor() as i32).clamp(0, rad) };
            (inset, inset)
        })
        .collect()
}

pub fn draw_osd(ui: &mut Ui, st: &State, w: i32, _h: i32) {
    let t = ui.t;
    let Some((kind, _)) = st.osd else { return };
    let island = st.set.island;
    let ox0 = ui.cv.ox;
    let pw = if island { island_osd_w(ui.f, st) } else { w - 8 };
    // остров: плашка по ширине содержимого, по центру поверхности
    if island {
        ui.cv.ox += (w - pw) / 2;
    }
    let fr = R::new(0, 0, pw, 56);
    if island {
        pill(ui, fr, 28, t.glass_panel(), t.border);
        // содержимое отступает от круглых концов
        ui.cv.ox += 8;
    } else {
        ui.cv.pxbox(fr, &BoxStyle::new(t.glass_panel(), t.border).shadow(4));
    }
    let fr = if island { R::new(0, 0, pw - 16, 56) } else { fr };
    if kind == "volume" {
        let a = &st.audio;
        ui.cv.icon(if a.muted { "mute" } else { "speaker" }, 12, 11, 2, t.red);
        ui.text_in(PX, R::new(34, 8, 100, 20), "звук", t.text_dim);
        let v = if a.muted { "тишина".to_string() } else { format!("{}%", (a.volume * 100.0).round()) };
        ui.text_right(PX, R::new(12, 8, fr.w - 24, 20), &v, if a.muted { t.text_faint } else { t.text });
        ui.cv.gauge(R::new(12, 34, fr.w - 24, 10), 20, if a.muted { 0.0 } else { a.volume as f32 }, if a.volume > 1.0 { t.ember } else { t.red }, t.surface_hi.lerp(t.border, 0.45), 2);
    } else if kind == "bright" {
        let v = st.bright.unwrap_or(0.0);
        ui.cv.icon("sun", 12, 11, 2, t.red);
        ui.text_in(PX, R::new(34, 8, 120, 20), "яркость", t.text_dim);
        ui.text_right(PX, R::new(12, 8, fr.w - 24, 20), &format!("{}%", (v * 100.0).round()), t.text);
        ui.cv.gauge(R::new(12, 34, fr.w - 24, 10), 20, v, t.red, t.surface_hi.lerp(t.border, 0.45), 2);
    } else if kind == "curtain" {
        let on = st.curtain.is_some();
        ui.cv.icon(if on { "lock" } else { "eye" }, 12, 11, 2, t.red);
        ui.text_in(PX, R::new(34, 8, fr.w - 46, 20), if on { "экран скрыт в трансляции" } else { "экран снова виден" }, t.text);
        ui.text_in(PX, R::new(12, 32, fr.w - 24, 18), if on { "зрители видят шторку, ты — экран" } else { "трансляция показывает рабочий стол" }, t.text_faint);
    } else {
        let b = R::new(10, 12, 44, 32);
        ui.cv.pxbox(b, &BoxStyle::new(t.red, t.ember).bevel(true));
        ui.text_center(font::PX24, b, &st.niri.kb_short(), t.on_accent);
        let name = st.niri.kb_names.get(st.niri.kb_idx).cloned().unwrap_or_default();
        ui.text_in(PX, R::new(66, 10, fr.w - 76, 20), &name, t.text);
        let mut x = 66;
        for i in 0..st.niri.kb_names.len() {
            let on = i == st.niri.kb_idx;
            ui.cv.fill(R::new(x, 36, if on { 16 } else { 8 }, 4), if on { t.red } else { t.border });
            x += if on { 20 } else { 12 };
        }
    }
    ui.cv.ox = ox0;
}

impl App {
    pub fn on_notif(&mut self, n: Incoming) {
        let tm = local_now();
        let critical = n.urgency >= 2;
        let life = if n.timeout > 0 { n.timeout as f32 / 1000.0 } else { self.st.set.notif_timeout as f32 };
        let note = Note {
            id: n.id,
            app: n.app,
            icon: n.icon,
            summary: n.summary,
            body: n.body,
            actions: n.actions,
            critical,
            image: n.image.filter(|_| self.st.set.notif_images).map(Rc::new),
            born: Instant::now(),
            time: format!("{:02}:{:02}", tm.hour, tm.min),
            life,
            left: life,
        };
        let id = note.id;
        let quiet = self.quiet();
        let ns = &mut self.st.notifs;
        if let Some(x) = ns.history.iter_mut().find(|x| x.id == id) {
            // замена (replaces_id): обновить на месте, без нового въезда
            let born = x.born;
            *x = note;
            x.born = born;
        } else {
            ns.history.insert(0, note);
            ns.history.truncate(self.st.set.notif_history.clamp(10, 500) as usize);
        }
        if (!quiet || critical) && !self.st.locked {
            ns.popups.retain(|x| *x != id);
            ns.leaving.retain(|x| x.0 != id);
            ns.popups.insert(0, id);
            // лишние уходят в историю
            ns.popups.truncate(self.st.set.notif_max.clamp(1, 8) as usize);
            ns.resume();
        }
        self.sync_notifs();
        self.dirty(&SKind::Bar);
        self.dirty(&SKind::Popup);
    }

    /// таймер: жизнь карточек; true — что-то истекло
    pub fn notif_tick(&mut self) -> bool {
        let ns = &mut self.st.notifs;
        if ns.popups.is_empty() {
            ns.last = None;
            return false;
        }
        let now = Instant::now();
        let dt = ns.last.map(|l| now.duration_since(l).as_secs_f32()).unwrap_or(0.0);
        ns.last = Some(now);
        let mut gone = vec![];
        // ушедшие до конца — из стопки
        let left: Vec<u32> = ns.leaving.iter().filter(|x| x.1.elapsed().as_secs_f32() >= ns.leave_s).map(|x| x.0).collect();
        let shrunk = !left.is_empty();
        for id in left {
            ns.popups.retain(|x| *x != id);
            ns.leaving.retain(|x| x.0 != id);
        }
        if !ns.hover {
            for id in ns.popups.clone() {
                if ns.leaving.iter().any(|x| x.0 == id) {
                    continue;
                }
                if let Some(n) = ns.history.iter_mut().find(|n| n.id == id) {
                    if n.critical {
                        continue;
                    }
                    n.left -= dt;
                    if n.left <= 0.0 {
                        gone.push(id);
                    }
                }
            }
        }
        for id in &gone {
            ns.remove(*id, false);
            crate::dbus::notif_closed(*id, 1);
        }
        let changed = shrunk || (!gone.is_empty() && ns.leave_s <= 0.0);
        self.dirty(&SKind::Notifs);
        changed
    }

    /// карточки на своём месте (notifPos): у края, с отступом 10; исключённые зоны бара и дока
    /// niri учитывает сам (exclusive 0), поэтому сверху — под баром, снизу — над доком
    fn notif_place(&self, out: &str, sw: i32) -> (Anchor, (i32, i32, i32, i32)) {
        // от края — notifOffset; бар niri сам вычитает (исключительная зона), карточки — под ним
        let o = self.st.set.num("notif_offset");
        if self.st.set.island {
            // остров: под серединой бара (бар сверху и не на весь экран) или по центру экрана
            return (Anchor::TOP | Anchor::LEFT, (o, 0, 0, self.island_left(out, sw)));
        }
        let pos = if self.st.set.island { "top" } else { self.st.set.notif_pos.as_str() };
        match pos {
            "top-left" => (Anchor::TOP | Anchor::LEFT, (o, 0, 0, 6)),
            "top" => (Anchor::TOP, (o, 0, 0, 0)),
            "bottom-right" => (Anchor::BOTTOM | Anchor::RIGHT, (0, 6, o, 0)),
            "bottom-left" => (Anchor::BOTTOM | Anchor::LEFT, (0, 0, o, 6)),
            "bottom" => (Anchor::BOTTOM, (0, 0, o, 0)),
            _ => (Anchor::TOP | Anchor::RIGHT, (o, 6, 0, 0)),
        }
    }

    /// левый край острова шириной `sw`: центр бара, если он сверху, иначе центр экрана; не за край экрана
    pub fn island_left(&self, out: &str, sw: i32) -> i32 {
        let (ow, _) = self.output_size(out);
        let (bt, ..) = self.bar_reserve(out);
        let cx = match self.bar_rect(out) {
            Some(r) if bt > 0 => r.x + r.w / 2,
            _ => ow / 2,
        };
        island_left_of(cx, sw, ow)
    }

    /// место или отступ карточек (OSD) поменялись: поверхность — заново, якорь layer-shell на лету
    /// не меняется
    pub fn renew_notif(&mut self, kind: SKind) {
        for id in self.find(&kind) {
            self.destroy(id);
        }
        if kind == SKind::Notifs {
            self.sync_notifs();
        }
    }

    pub fn sync_notifs(&mut self) {
        let empty = self.st.notifs.popups.is_empty() || self.st.locked;
        let ids = self.find(&SKind::Notifs);
        if empty {
            for id in ids {
                self.destroy(id);
            }
            self.st.notifs.hover = false;
            return;
        }
        let (w, h) = size(&self.st, &mut self.res.fonts);
        if ids.is_empty() {
            let out = self.focus_out();
            let (anchor, margin) = self.notif_place(&out, w);
            self.mk(SKind::Notifs, &out, Layer::Overlay, "yami-notif", anchor, (w, h), margin, 0, KeyboardInteractivity::None);
        } else {
            for id in ids {
                if let Some(s) = self.surfs.get_mut(&id) {
                    if s.h != h || s.w != w {
                        s.layer.set_size(w as u32, h as u32);
                        s.layer.commit();
                    }
                    s.dirty = true;
                }
            }
        }
    }

    /// карточка-пример, когда место уведомлений меняют в настройках: видно, куда они встанут
    pub fn notif_preview(&mut self) {
        self.toast_note("уведомления — здесь", "так встанут новые карточки");
    }

    /// своя карточка в обход D-Bus: короткое сообщение от самого шелла
    pub fn toast_note(&mut self, summary: &str, body: &str) {
        self.toast_note_plus(summary, body, None);
    }

    /// то же, с репликой рыбки второй строкой (personality.rs)
    pub fn toast_note_plus(&mut self, summary: &str, body: &str, quip: Option<String>) {
        // свои номера по кругу 9 500 000…9 999 999: не совпадают друг с другом и с карточкой обновления (update.rs NOTE_ID)
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let id = 9_500_000 + NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % 500_000;
        let n = crate::dbus::Incoming { id, app: "yami".into(), icon: String::new(), summary: crate::i18n::tr(summary).into(), body: match quip { Some(q) => format!("{}\n{q}", crate::i18n::tr(body)), None => crate::i18n::tr(body).into() }, actions: vec![], urgency: 1, image: None, timeout: -1 };
        self.on_notif(n);
    }

    pub fn notif_click(&mut self, id: u32, default: bool) {
        if default {
            if let Some(n) = self.st.notifs.get(id) {
                if n.actions.iter().any(|a| a.0 == "default") {
                    crate::dbus::notif_action(id, "default");
                }
            }
        }
        self.st.notifs.remove(id, false);
        crate::dbus::notif_closed(id, 2);
        self.sync_notifs();
    }
}

/// места карточек уведомлений (notifPos) и OSD (osdPos) по кругу в настройках: значение, подпись
pub const POS: &[(&str, &str)] =
    &[("top-right", "справа сверху"), ("top-left", "слева сверху"), ("top", "сверху по центру"), ("bottom-right", "справа снизу"), ("bottom-left", "слева снизу"), ("bottom", "снизу по центру")];
pub const OSD_POS: &[(&str, &str)] = &[
    ("bottom", "снизу"),
    ("top", "сверху"),
    ("center", "по центру"),
    ("top-left", "слева сверху"),
    ("top-right", "справа сверху"),
    ("left", "слева"),
    ("right", "справа"),
    ("bottom-left", "слева снизу"),
    ("bottom-right", "справа снизу"),
];

impl App {
    /// OSD громкости, яркости, раскладки, таймера снимка (osdPos)
    pub fn show_osd(&mut self, kind: &'static str) {
        if self.st.locked {
            return;
        }
        let fresh = self.st.osd.is_none();
        self.st.osd = Some((kind, Instant::now()));
        if fresh || self.find(&SKind::Osd).is_empty() {
            let out = self.focus_out();
            // osdPos: у края или в углу, отступ — от бара, если он с той стороны; по центру экрана — без отступа.
            // Исключённые зоны OSD не видит (-1): место бара прибавляем сами
            let (bt, br, bb, bl) = self.bar_reserve(&out);
            let o = self.st.set.num("osd_offset");
            let p = if self.st.set.island { "top".to_string() } else { self.st.set.osd_pos.clone() };
            let mut anchor = Anchor::empty();
            let (mut mt, mut mr, mut mb, mut ml) = (0, 0, 0, 0);
            if p.starts_with("top") {
                anchor |= Anchor::TOP;
                mt = bt + o;
            } else if p.starts_with("bottom") {
                anchor |= Anchor::BOTTOM;
                mb = bb + o;
            }
            if p.ends_with("left") {
                anchor |= Anchor::LEFT;
                ml = bl + 12;
            } else if p.ends_with("right") {
                anchor |= Anchor::RIGHT;
                mr = br + 12;
            }
            if self.st.set.island {
                anchor = Anchor::TOP | Anchor::LEFT;
                (mr, ml) = (0, self.island_left(&out, 308));
            }
            let margin = (mt, mr, mb, ml);
            let id = self.mk(SKind::Osd, &out, Layer::Overlay, "yami-osd", anchor, (308, 64 + 8), margin, -1, KeyboardInteractivity::None);
            if let Some(s) = self.surfs.get(&id) {
                if let Ok(r) = smithay_client_toolkit::compositor::Region::new(&self.compositor) {
                    s.layer.wl_surface().set_input_region(Some(r.wl_region()));
                }
            }
        }
        self.dirty(&SKind::Osd);
    }
}

#[cfg(test)]
mod pill_tests {
    use super::pill_spans;

    #[test]
    fn a_pill_is_round_at_the_ends_and_straight_between() {
        let s = pill_spans(56, 28);
        assert_eq!(s.len(), 56);
        assert!(s[0].0 > 10, "the top row is far in: {:?}", s[0]);
        assert!(s[27].0 <= 1 && s[28].0 <= 1, "the middle rows reach the edge");
        // симметрия сверху вниз и слева направо
        for i in 0..28 {
            assert_eq!(s[i], s[55 - i], "row {i}");
            assert_eq!(s[i].0, s[i].1);
        }
        // ступени идут к краю не вспять
        assert!(s[..28].windows(2).all(|w| w[0].0 >= w[1].0));
    }

    #[test]
    fn a_short_card_never_gets_a_radius_bigger_than_half_its_height() {
        let s = pill_spans(20, 18);
        assert!(s[0].0 <= 10 && s.len() == 20);
        assert_eq!(pill_spans(10, 0).iter().filter(|x| x.0 > 0).count(), 0, "no radius: a plain box");
    }
}

#[cfg(test)]
mod island_place_tests {
    use super::island_left_of;

    /// остров шириной 386 под серединой бара: его центр — центр бара
    #[test]
    fn the_island_is_centred_under_the_bar() {
        let left = island_left_of(960, 386, 1920);
        assert_eq!(left + 386 / 2, 960);
        // бар не на весь экран и смещён: остров следует за ним
        assert_eq!(island_left_of(600, 386, 1920) + 193, 600);
    }

    /// у края экрана остров не вылезает за него ни слева, ни справа
    #[test]
    fn the_island_never_leaves_the_screen() {
        assert_eq!(island_left_of(50, 386, 1920), 0);
        assert_eq!(island_left_of(1900, 386, 1920), 1920 - 386);
        assert_eq!(island_left_of(960, 3000, 1920), 0, "wider than the screen: start at the edge");
    }
}

