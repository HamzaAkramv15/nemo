// Тестовый вход для сквозных проверок (tools/e2e.sh): живой шелл во вложенном niri получает «нажатия» и значения ползунков по IPC и
// пишет в файл, что видно на экране. Только при NEMO_TEST_IPC=1 (его выставляет вложенная сессия tools/nested-session.sh): на настоящем
// столе этой лазейки нет. Команды (после `t:`):
//   zones KIND            — записать зоны поверхности (x y w h Act) в $XDG_RUNTIME_DIR/nemo-test-out.txt
//   click KIND ТЕКСТ [r]  — нажать (r — правой кнопкой) первую сверху зону, чей Act содержит ТЕКСТ
//   slider КЛЮЧ ЗНАЧЕНИЕ  — выставить ползунок настроек так, как это делает перетаскивание
//   nav next|prev|activate|left|right — обход с клавиатуры в окне настроек; пишет выбранный узел
//   tree                  — дерево доступности окна настроек
//   island                — id уведомления на таблетке, начальная и текущая ширина, мс с последней отрисовки, показана ли таблетка сейчас (1/0)
// KIND: modal | bar | popup | dock | menu | widget:ID. Нажатие идёт тем же путём, что и мышь (`on_click`), с центром зоны.

use crate::app::{App, SKind};

fn kind_of(k: &str) -> Option<SKind> {
    Some(match k {
        "modal" => SKind::Modal,
        "bar" => SKind::Bar,
        "popup" => SKind::Popup,
        "dock" => SKind::Dock,
        "menu" => SKind::Menu,
        w => SKind::Widget(w.strip_prefix("widget:")?.parse().ok()?),
    })
}

impl App {
    pub fn test_cmd(&mut self, c: &str) {
        if std::env::var_os("NEMO_TEST_IPC").is_none() {
            return;
        }
        let out = std::path::PathBuf::from(std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into())).join("nemo-test-out.txt");
        let mut it = c.split_whitespace();
        match it.next() {
            Some("zones") => {
                let Some(kind) = it.next().and_then(kind_of) else { return };
                let mut s = String::new();
                for (sid, surf) in self.surfs.iter().filter(|(_, s)| s.kind == kind) {
                    for z in &surf.zones {
                        s.push_str(&format!("{sid} {} {} {} {} {:?}\n", z.r.x, z.r.y, z.r.w, z.r.h, z.act));
                    }
                }
                let _ = std::fs::write(out, s);
            }
            Some("click") => {
                let Some(kind) = it.next().and_then(kind_of) else { return };
                let pat = it.next().unwrap_or("");
                let right = it.next() == Some("r");
                let hit = self.surfs.iter().filter(|(_, s)| s.kind == kind).find_map(|(sid, s)| s.zones.iter().rev().find(|z| format!("{:?}", z.act).contains(pat)).map(|z| (*sid, z.act.clone(), z.r)));
                match hit {
                    Some((sid, act, r)) => {
                        let _ = std::fs::write(&out, format!("clicked {sid} {act:?}\n"));
                        // 0x110 — левая кнопка, 0x111 — правая (как в evdev)
                        self.on_click(act, if right { 0x111 } else { 0x110 }, sid, r, false);
                    }
                    None => {
                        let _ = std::fs::write(&out, format!("no zone matches {pat}\n"));
                    }
                }
            }
            Some("nav") => {
                use crate::a11y::Nav;
                let nav = match it.next() {
                    Some("next") => Nav::Next,
                    Some("prev") => Nav::Prev,
                    Some("activate") => Nav::Activate,
                    Some("left") => Nav::Left,
                    Some("right") => Nav::Right,
                    _ => return,
                };
                self.a11y_nav(nav);
                let _ = std::fs::write(out, format!("{}\n", self.a11y_focus_name().unwrap_or_else(|| "none".into())));
            }
            Some("tree") => {
                let nodes = self.st.a11y_nodes.borrow();
                let s: String = nodes.iter().map(|n| format!("{:?} | {} | {} | {},{} {}x{} | checked={} selected={}\n", n.role, n.name, n.value, n.rect.x, n.rect.y, n.rect.w, n.rect.h, n.checked, n.selected)).collect();
                let _ = std::fs::write(out, s);
            }
            Some("island") => {
                let _ = std::fs::write(out, format!("{}\n", crate::island::debug_state(&self.st)));
            }
            Some("slider") => {
                if let (Some(key), Some(v)) = (it.next(), it.next().and_then(|v| v.parse::<f32>().ok())) {
                    self.slider_set(key, v);
                }
            }
            _ => {}
        }
    }
}
