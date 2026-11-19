// Подсказки бара: курсор постоял на значке — рядом на секунду-другую встаёт маленькая надпись, что
// значок показывает и что делают щелчок и правая кнопка. Своя поверхность без ввода: она не мешает
// кликам и не перехватывает курсор. Только то, чего не видно по самому значку (кофе, ночной свет,
// батарея): остальное и так подписано.

use crate::app::{App, SKind, State};
use crate::font::PX;
use crate::gfx::R;
use crate::ui::Ui;
use smithay_client_toolkit::shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer};
use smithay_client_toolkit::shell::WaylandSurface;
use std::time::{Duration, Instant};

/// сколько курсор стоит на значке, прежде чем встанет подсказка
const DWELL: Duration = Duration::from_millis(550);

/// строки подсказки по ключу, который модуль бара назвал в Ui::tip; None — подсказки нет (или нечего сказать)
pub fn text(st: &State, key: &str) -> Option<Vec<String>> {
    match key {
        "caffeine" => {
            let c = &st.caffeine;
            let head = if !c.on {
                crate::tf!("кофе выключен", "caffeine is off", "café desligado")
            } else {
                match c.left_min() {
                    Some(m) => crate::tf!("кофе включён: ещё {}", "caffeine is on: {} left", "café ligado: {} restantes", span(m)),
                    None => crate::tf!("кофе включён, пока не выключишь", "caffeine is on until you turn it off", "café ligado até você desligar"),
                }
            };
            Some(vec![head, crate::tf!("щелчок: вкл/выкл · правая: срок", "click: on/off · right: duration", "clique: liga/desliga · direito: duração")])
        }
        "night" => {
            let s = &st.set;
            let head = if s.night { crate::tf!("ночной свет включён: {} К", "night light is on: {} K", "luz noturna ligada: {} K", s.night_temp) } else { crate::tf!("ночной свет выключен", "night light is off", "luz noturna desligada") };
            Some(vec![head, crate::tf!("щелчок: вкл/выкл · правая: температура", "click: on/off · right: temperature", "clique: liga/desliga · direito: temperatura")])
        }
        // батарея и профиль питания — разные модули: у каждого своя подсказка
        "battery" => {
            let b = st.sys.bat?;
            let state = if b.charging {
                crate::tf!("заряжается", "charging", "carregando")
            } else if b.plugged {
                crate::tf!("от сети", "plugged in", "na tomada")
            } else {
                crate::tf!("от батареи", "on battery", "na bateria")
            };
            let mut head = format!("{}% · {}", b.pct, state);
            if let Some(m) = b.mins {
                head += &crate::tf!(" · {}", " · {} left", " · {} restantes", span(m as u64));
            }
            Some(vec![head])
        }
        "profile" => {
            if !st.prof.present {
                return None;
            }
            let name = match st.prof.active.as_str() {
                "power-saver" => crate::tf!("экономия", "power saver", "economia"),
                "performance" => crate::tf!("производительность", "performance", "desempenho"),
                _ => crate::tf!("сбалансированный", "balanced", "balanceado"),
            };
            let mut v = vec![crate::tf!("профиль питания: {}", "power profile: {}", "perfil de energia: {}", name), crate::tf!("щелчок: выбрать · правая: следующий", "click: choose · right: next", "clique: escolher · direito: próximo")];
            if !st.prof.degraded.is_empty() {
                v.insert(1, st.prof.degraded.clone());
            }
            Some(v)
        }
        "net" => {
            if !st.sys.online {
                return Some(vec![crate::tf!("нет сети", "no network", "sem rede")]);
            }
            let kind: String = if st.sys.wifi { "Wi-Fi".into() } else { crate::tf!("кабель", "wired", "cabo") };
            let name = st.net.active.first().map(|a| a.0.clone()).filter(|n| !n.is_empty());
            let head = match name {
                Some(n) => format!("{n} · {kind}"),
                None => kind,
            };
            let mut v = vec![head];
            if !st.sys.ip.is_empty() {
                v.push(format!("{} · ↓{} ↑{}", st.sys.ip, crate::sys::rate(st.sys.rx), crate::sys::rate(st.sys.tx)));
            }
            Some(v)
        }
        "bt" => {
            if !st.bt.present {
                return None;
            }
            if !st.bt.powered {
                return Some(vec![crate::tf!("Bluetooth выключен", "Bluetooth is off", "Bluetooth desligado")]);
            }
            let names: Vec<String> = st.bt.devices.iter().filter(|d| d.connected).map(|d| match d.battery {
                Some(p) => format!("{} · {p}%", d.name),
                None => d.name.clone(),
            }).collect();
            if names.is_empty() {
                return Some(vec![crate::tf!("Bluetooth включён, ничего не подключено", "Bluetooth is on, nothing connected", "Bluetooth ligado, nada conectado")]);
            }
            Some(names.into_iter().take(4).collect())
        }
        "vol" => {
            let a = &st.audio;
            if !a.ready {
                return None;
            }
            let head = if a.muted { crate::tf!("звук выключен", "sound is muted", "som mudo") } else { crate::tf!("громкость {}%", "volume {}%", "volume {}%", (a.volume * 100.0).round() as i32) };
            let mut v = vec![head];
            if let Some(d) = a.sinks.iter().find(|d| d.name == a.sink) {
                v.push(d.desc.clone());
            }
            Some(v)
        }
        "kb" => {
            let n = st.niri.kb_names.get(st.niri.kb_idx)?.clone();
            Some(vec![n, crate::tf!("щелчок: следующая раскладка", "click: next layout", "clique: próximo layout")])
        }
        "clock" => {
            let now = crate::clock::local_now();
            let mut v = vec![yami_px::i18n::long_date(crate::i18n::tr(crate::clock::RU_DAY_FULL[now.wday as usize]), now.day, (now.mon - 1) as usize, crate::i18n::tr(crate::clock::RU_MON[(now.mon - 1) as usize]))];
            if st.set.hijri {
                let days = crate::widgets::days_from_civil(now.year, now.mon, now.day);
                v.push(crate::hijri::text(days, crate::i18n::code() == "ar"));
            }
            Some(v)
        }
        "privacy" => {
            let (m, c, s) = crate::privacy::lit(st);
            let mut v = vec![];
            if m {
                v.push(crate::tf!("микрофон: {}", "microphone: {}", "microfone: {}", st.mic.join(", ")));
            }
            if c {
                v.push(crate::tf!("камера: {}", "camera: {}", "câmera: {}", st.camera.join(", ")));
            }
            if s {
                v.push(crate::tf!("экран показывается: {}", "screen is shared: {}", "tela compartilhada: {}", st.niri.casts.len()));
            }
            (!v.is_empty()).then_some(v)
        }
        _ => None,
    }
}

/// ключ подсказки для значка вертикального бара по его щелчку (vstack); горизонтальные модули называют ключ сами
pub fn key_of(a: &crate::ui::Act) -> Option<&'static str> {
    use crate::ui::Act;
    match a {
        Act::CaffeineToggle => Some("caffeine"),
        Act::NightToggle => Some("night"),
        Act::Popup("net") => Some("net"),
        Act::Popup("bt") => Some("bt"),
        Act::Popup("profile") => Some("profile"),
        Act::Popup("sound") => Some("vol"),
        _ => None,
    }
}

/// «45m», «1h 20m»
fn span(m: u64) -> String {
    if m >= 60 {
        if m % 60 == 0 { format!("{}h", m / 60) } else { format!("{}h {}m", m / 60, m % 60) }
    } else {
        format!("{m}m")
    }
}

pub fn draw(ui: &mut Ui, st: &State, w: i32, h: i32) {
    let Some(t) = &st.tip else { return };
    let r = R::new(0, 0, w, h);
    ui.frame(r, false);
    let th = ui.t;
    for (i, l) in t.lines.iter().enumerate() {
        ui.text_in(PX, R::new(12, 7 + i as i32 * 18, w - 20, 18), l, if i == 0 { th.text } else { th.text_faint });
    }
}

/// подсказка, что показана или ждёт: что за значок, где, с какого момента курсор на нём
pub struct Tip {
    pub key: &'static str,
    pub lines: Vec<String>,
    pub out: String,
    /// точка под значком в координатах монитора (как у всплывашек)
    pub at: (i32, i32),
    pub since: Instant,
    pub shown: bool,
}

impl App {
    /// курсор встал на значок бара (или ушёл с него, или перешёл на другой)
    pub fn tip_hover(&mut self, sid: u32, zone: Option<(&'static str, R)>) {
        let on_bar = self.surf_kind(sid) == Some(SKind::Bar);
        let Some((key, r)) = zone.filter(|_| on_bar) else { return self.tip_off() };
        if self.st.tip.as_ref().is_some_and(|t| t.key == key) {
            return;
        }
        self.tip_off();
        let Some(lines) = text(&self.st, key) else { return };
        let s = self.surfs.get(&sid);
        let out = s.map(|s| s.output.clone()).unwrap_or_else(|| self.focus_out());
        let (mx, my) = s.map(|s| s.margin).unwrap_or((0, 0));
        self.st.tip = Some(Tip { key, lines, out, at: (mx + r.x + r.w / 2, my + r.bottom()), since: Instant::now(), shown: false });
    }

    pub fn tip_off(&mut self) {
        // подсказка не «проявляется» и не «растворяется»: появилась — есть, ушла — нет (иначе узор появления
        // и всплытие на маленькой надписи выглядят как сбой)
        if self.st.tip.take().is_some() {
            for id in self.find(&SKind::Tip) {
                self.destroy(id);
            }
        }
    }

    /// раз в тик: постоял достаточно — показать; текст живой (осталось время кофе), обновляется на ходу
    pub fn tip_tick(&mut self) {
        if self.st.popup.is_some() || self.st.menu.is_some() || self.st.modal.is_some() {
            return self.tip_off();
        }
        let Some(t) = self.st.tip.as_ref() else { return };
        if !t.shown {
            if t.since.elapsed() < DWELL {
                return;
            }
            let (out, at, key) = (t.out.clone(), t.at, t.key);
            let Some(lines) = text(&self.st, key) else { return self.tip_off() };
            let (w, h) = measure(&mut self.res.fonts, &lines);
            let (x, top) = self.tip_place(&out, at, w, h);
            let id = self.mk(SKind::Tip, &out, Layer::Overlay, "nemo-tip", Anchor::TOP | Anchor::LEFT, (w, h), (top, 0, 0, x), -1, KeyboardInteractivity::None);
            if let Some(s) = self.surfs.get_mut(&id) {
                s.margin = (x, top);
                s.applied = s.margin;
                if let Ok(r) = smithay_client_toolkit::compositor::Region::new(&self.compositor) {
                    s.layer.wl_surface().set_input_region(Some(r.wl_region()));
                }
            }
            if let Some(t) = self.st.tip.as_mut() {
                t.shown = true;
                t.lines = lines;
            }
        } else if let Some(lines) = text(&self.st, t.key) {
            if lines != t.lines {
                if let Some(t) = self.st.tip.as_mut() {
                    t.lines = lines;
                }
                self.dirty(&SKind::Tip);
            }
        }
    }

    /// угол подсказки: под значком у верхнего бара, над ним у нижнего, сбоку у бокового
    fn tip_place(&self, out: &str, at: (i32, i32), w: i32, h: i32) -> (i32, i32) {
        let (ow, oh) = self.output_size(out);
        let (bt, br, bb, bl) = self.bar_reserve(out);
        if bl > 0 || br > 0 {
            let x = if bl > 0 { bl + 6 } else { ow - br - 6 - w };
            (x.max(0), (at.1 - h).clamp(6, (oh - h - 6).max(6)))
        } else if bb > 0 {
            ((at.0 - w / 2).clamp(6, (ow - w - 6).max(6)), (oh - bb - 6 - h).max(0))
        } else {
            ((at.0 - w / 2).clamp(6, (ow - w - 6).max(6)), bt + 6)
        }
    }
}

/// размер подсказки по шрифтам (до того, как есть поверхность для Ui)
pub fn measure(fonts: &mut crate::font::Fonts, lines: &[String]) -> (i32, i32) {
    let w = lines.iter().map(|l| fonts.width(PX, l)).max().unwrap_or(0) + 24;
    (w, lines.len() as i32 * 18 + 14)
}

#[cfg(test)]
mod tests {
    #[test]
    fn spans_read_naturally() {
        assert_eq!(super::span(5), "5m");
        assert_eq!(super::span(60), "1h");
        assert_eq!(super::span(80), "1h 20m");
    }
}
