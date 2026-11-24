// Доступность окон шелла: обход клавишей Tab с кольцом фокуса и дерево для экранного диктора.
// Модель (роль, имя, значение, рамка) собирают помощники Ui (nemo_px::a11y); здесь — клавиши, нажатие выбранного и (за флагом `a11y`)
// отдача дерева через AccessKit по AT-SPI. Сейчас охвачено окно настроек: самое большое; остальное — по тому же образцу.

use crate::app::{App, SKind};
use crate::ui::Act;
use nemo_px::a11y::{self, Role};
use nemo_px::gfx::R;
use smithay_client_toolkit::seat::keyboard::{KeyEvent, Keysym};

/// собрана ли поддержка экранного диктора (cargo feature `a11y`)
pub fn compiled() -> bool {
    cfg!(feature = "a11y")
}

/// шаги обхода
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Nav {
    Next,
    Prev,
    Activate,
    Left,
    Right,
}

impl App {
    /// клавиши обхода в окне настроек; true — клавиша обработана
    pub fn a11y_key(&mut self, e: &KeyEvent) -> bool {
        let nav = match e.keysym {
            Keysym::Tab if self.mods.shift => Nav::Prev,
            Keysym::ISO_Left_Tab => Nav::Prev,
            Keysym::Tab => Nav::Next,
            Keysym::Return | Keysym::KP_Enter | Keysym::space => Nav::Activate,
            Keysym::Left => Nav::Left,
            Keysym::Right => Nav::Right,
            _ => return false,
        };
        self.a11y_nav(nav)
    }

    /// то же без клавиатуры: из теста (testdrive.rs) и от экранного диктора
    pub fn a11y_nav(&mut self, nav: Nav) -> bool {
        match nav {
            Nav::Next | Nav::Prev => {
                let nodes = self.st.a11y_nodes.borrow().clone();
                self.st.a11y_focus = a11y::step(&nodes, self.st.a11y_focus, if nav == Nav::Prev { -1 } else { 1 });
                self.a11y_reveal();
                self.dirty(&SKind::Modal);
                true
            }
            Nav::Activate if self.st.a11y_focus.is_some() => {
                self.a11y_activate();
                true
            }
            Nav::Left | Nav::Right if self.st.a11y_focus.is_some() => self.a11y_slide(if nav == Nav::Left { -1 } else { 1 }),
            _ => false,
        }
    }

    /// выбранный узел: роль и имя (для теста и диктора)
    pub fn a11y_focus_name(&self) -> Option<String> {
        self.st.a11y_focus.and_then(|i| self.st.a11y_nodes.borrow().get(i).map(|n| format!("{:?}:{}", n.role, n.name)))
    }

    /// выбранный узел оказался за краем страницы: прокрутить к нему (боковой список прокручивается своим колесом)
    fn a11y_reveal(&mut self) {
        let Some(n) = self.st.a11y_focus.and_then(|i| self.st.a11y_nodes.borrow().get(i).cloned()) else { return };
        let (w, h) = crate::settings::SIZE;
        let sw = crate::settings::SIDE_W;
        let body = crate::settings::body_rect(w, h, sw);
        if n.rect.x < body.x - 8 {
            return;
        }
        let off = crate::settings::scroll_of(&self.st);
        let (top, bottom) = (body.y + 8, body.bottom() - 8);
        let want = if n.rect.y < top { off - (top - n.rect.y) } else if n.rect.bottom() > bottom { off + (n.rect.bottom() - bottom) } else { return };
        let max = (self.st.sscroll_h.get() - body.h).max(0);
        self.st.sscroll = (want.clamp(0, max), self.st.stab);
    }

    /// зона, что лежит под центром узла (последняя зарегистрированная — верхняя)
    fn a11y_zone_at(&self, n: &a11y::Node) -> Option<(u32, Act, R)> {
        let (cx, cy) = (n.rect.x + n.rect.w / 2, n.rect.y + n.rect.h / 2);
        self.surfs.iter().filter(|(_, s)| s.kind == SKind::Modal).find_map(|(sid, s)| s.zones.iter().rev().find(|z| z.r.contains(cx as f64, cy as f64)).map(|z| (*sid, z.act.clone(), z.r)))
    }

    /// нажать выбранный узел так, как нажала бы мышь
    pub fn a11y_activate(&mut self) {
        let Some(n) = self.st.a11y_focus.and_then(|i| self.st.a11y_nodes.borrow().get(i).cloned()) else { return };
        match n.role {
            Role::TextInput => {
                // поле: фокус ввода тем же путём, что и щелчок
            }
            Role::Heading => return,
            _ => {}
        }
        if let Some((sid, act, r)) = self.a11y_zone_at(&n) {
            self.on_click(act, 0x110, sid, r, false);
        }
    }

    /// стрелки на ползунке: на одно деление
    fn a11y_slide(&mut self, dir: i32) -> bool {
        let Some(n) = self.st.a11y_focus.and_then(|i| self.st.a11y_nodes.borrow().get(i).cloned()) else { return false };
        if n.role != Role::Slider {
            return false;
        }
        if let Some((_, Act::SSlider(key, _, _, segs), _)) = self.a11y_zone_at(&n) {
            let step = 1.0 / segs.max(1) as f32;
            self.slider_set(key, (n.fraction + dir as f32 * step).clamp(0.0, 1.0));
            self.dirty(&SKind::Modal);
            return true;
        }
        false
    }
}

// ── экранный диктор: дерево окна настроек через AccessKit по AT-SPI (cargo feature `a11y`) ─────────────────────────────────────────
// Мост создаётся при первом открытии окна настроек: до этого он ничего не стоит. Дерево строится в главном потоке после каждой отрисовки;
// просьбы диктора (сфокусировать, нажать) приходят из потока AccessKit и уходят в цикл шелла сообщением Msg::A11y.

#[cfg(feature = "a11y")]
pub mod bridge {
    use super::*;
    use accesskit::{Action, ActionHandler, ActionRequest, ActivationHandler, DeactivationHandler, Node, NodeId, Rect, Role as KRole, Toggled, Tree, TreeUpdate};
    use std::sync::{Arc, Mutex};

    const ROOT: NodeId = NodeId(0);

    /// узел с индексом i в списке — NodeId(i + 1)
    fn id(i: usize) -> NodeId {
        NodeId(i as u64 + 1)
    }

    /// полное дерево окна: корень-окно, заголовок и элементы в порядке рисования
    pub fn tree(nodes: &[a11y::Node], focus: Option<usize>, title: &str) -> TreeUpdate {
        let mut out: Vec<(NodeId, Node)> = Vec::with_capacity(nodes.len() + 1);
        let mut kids = Vec::new();
        for (i, n) in nodes.iter().enumerate() {
            let role = match n.role {
                Role::Button if n.selected => KRole::Tab,
                Role::Button => KRole::Button,
                Role::Toggle => KRole::CheckBox,
                Role::Slider => KRole::Slider,
                Role::TextInput => KRole::TextInput,
                Role::Heading => KRole::Heading,
            };
            let mut k = Node::new(role);
            k.set_label(n.name.as_str());
            k.set_bounds(Rect { x0: n.rect.x as f64, y0: n.rect.y as f64, x1: n.rect.right() as f64, y1: n.rect.bottom() as f64 });
            match n.role {
                Role::Toggle => {
                    k.set_toggled(if n.checked { Toggled::True } else { Toggled::False });
                    if !n.value.is_empty() {
                        k.set_description(n.value.as_str());
                    }
                }
                Role::Slider => {
                    k.set_value(n.value.as_str());
                    k.set_numeric_value(n.fraction as f64);
                    k.set_min_numeric_value(0.0);
                    k.set_max_numeric_value(1.0);
                }
                Role::TextInput => k.set_value(n.value.as_str()),
                _ => {}
            }
            if n.selected && n.role == Role::Button {
                k.set_selected(true);
            }
            if n.focusable() {
                k.add_action(Action::Focus);
                k.add_action(Action::Click);
            }
            out.push((id(i), k));
            kids.push(id(i));
        }
        let mut root = Node::new(KRole::Window);
        root.set_label(title);
        root.set_children(kids);
        out.push((ROOT, root));
        let mut t = Tree::new(ROOT);
        t.toolkit_name = Some("nemo-px".into());
        t.toolkit_version = Some(env!("CARGO_PKG_VERSION").into());
        TreeUpdate { nodes: out, tree: Some(t), focus: focus.map_or(ROOT, id) }
    }

    struct Act(calloop::channel::Sender<crate::app::Msg>);
    impl ActionHandler for Act {
        fn do_action(&mut self, r: ActionRequest) {
            if r.target.0 == 0 {
                return;
            }
            let click = matches!(r.action, Action::Click);
            if click || matches!(r.action, Action::Focus) {
                let _ = self.0.send(crate::app::Msg::A11y((r.target.0 - 1) as usize, click));
            }
        }
    }

    struct Wake(Arc<Mutex<Option<TreeUpdate>>>);
    impl ActivationHandler for Wake {
        fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
            self.0.lock().ok().and_then(|l| l.clone())
        }
    }

    struct Off;
    impl DeactivationHandler for Off {
        fn deactivate_accessibility(&mut self) {}
    }

    pub struct Bridge {
        adapter: accesskit_unix::Adapter,
        latest: Arc<Mutex<Option<TreeUpdate>>>,
    }

    impl Bridge {
        pub fn new(tx: calloop::channel::Sender<crate::app::Msg>) -> Bridge {
            let latest = Arc::new(Mutex::new(None));
            Bridge { adapter: accesskit_unix::Adapter::new(Wake(latest.clone()), Act(tx), Off), latest }
        }

        /// новое дерево: запомнить для диктора, что подключится позже, и отдать подключённому
        pub fn publish(&mut self, update: TreeUpdate) {
            if let Ok(mut l) = self.latest.lock() {
                *l = Some(update.clone());
            }
            self.adapter.update_if_active(|| update);
        }

        pub fn window_focus(&mut self, on: bool) {
            self.adapter.update_window_focus_state(on);
        }
    }
}

impl App {
    /// просьба экранного диктора: сфокусировать узел или нажать его
    pub fn a11y_request(&mut self, idx: usize, click: bool) {
        if idx >= self.st.a11y_nodes.borrow().len() {
            return;
        }
        self.st.a11y_focus = Some(idx);
        self.a11y_reveal();
        if click {
            self.a11y_activate();
        }
        self.dirty(&SKind::Modal);
    }

    /// после отрисовки окна настроек: дерево — диктору (мост создаётся при первом открытии)
    pub fn a11y_sync(&mut self) {
        #[cfg(feature = "a11y")]
        {
            if self.st.modal != Some(crate::app::Modal::Settings) {
                if let Some(b) = self.a11y.as_mut() {
                    b.window_focus(false);
                }
                return;
            }
            let stamp = (self.st.a11y_nodes.borrow().len(), self.st.a11y_focus, self.st.stab);
            if self.a11y_stamp == Some(stamp) && self.a11y.is_some() {
                return;
            }
            if self.a11y.is_none() {
                self.a11y = Some(bridge::Bridge::new(self.tx.clone()));
            }
            let title = crate::i18n::tr("настройки").to_string();
            let update = bridge::tree(&self.st.a11y_nodes.borrow(), self.st.a11y_focus, &title);
            if let Some(b) = self.a11y.as_mut() {
                b.publish(update);
                b.window_focus(true);
            }
            self.a11y_stamp = Some(stamp);
        }
    }
}
