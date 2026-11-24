// Семантическая модель интерфейса: каждый интерактивный элемент (кнопка, флажок, ползунок, поле, заголовок), который рисуют помощники Ui,
// записывает «роль, имя, значение, рамка, состояние». Из этого строится дерево доступности (шелл отдаёт его экранному диктору через AT-SPI),
// порядок обхода клавишей Tab и кольцо фокуса. Запись включена только когда она нужна (обход клавиатурой, диктор, тест): иначе — ничего не стоит.
// Поток-локальное хранилище, как у подсказок (Ui::tip): поле в Ui сломало бы модули, что собирают его литералом.

use crate::gfx::R;
use std::cell::{Cell, RefCell};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Button,
    Toggle,
    Slider,
    TextInput,
    Heading,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub role: Role,
    /// видимое название, уже переведённое
    pub name: String,
    /// значение: текст ползунка или поля
    pub value: String,
    /// рамка в координатах поверхности
    pub rect: R,
    /// флажок включён
    pub checked: bool,
    /// кнопка выбрана (вкладка, режим)
    pub selected: bool,
    pub enabled: bool,
    /// доля ползунка 0..1
    pub fraction: f32,
    /// рамка хоть частично на виду (не прокручена за край)
    pub visible: bool,
}

impl Node {
    /// его можно нажать или выбрать клавиатурой (прокрученный за край — тоже: окно прокрутится к нему)
    pub fn focusable(&self) -> bool {
        self.role != Role::Heading && self.enabled
    }
}

thread_local! {
    static ON: Cell<bool> = const { Cell::new(false) };
    static NODES: RefCell<Vec<Node>> = const { RefCell::new(Vec::new()) };
}

/// включить или выключить запись (на кадр: App включает её перед отрисовкой окна, если есть кому читать)
pub fn enable(on: bool) {
    ON.with(|o| o.set(on));
}

pub fn enabled() -> bool {
    ON.with(|o| o.get())
}

pub fn push(n: Node) {
    NODES.with(|v| v.borrow_mut().push(n));
}

/// узлы последней отрисовки по порядку; список очищается
pub fn take() -> Vec<Node> {
    NODES.with(|v| std::mem::take(&mut *v.borrow_mut()))
}

/// порядок обхода клавишей Tab: индексы тех, до кого можно дойти, в порядке рисования
pub fn order(nodes: &[Node]) -> Vec<usize> {
    nodes.iter().enumerate().filter(|(_, n)| n.focusable()).map(|(i, _)| i).collect()
}

/// следующий (dir = 1) или предыдущий (dir = -1) узел по кругу от `cur`; None — перейти не к чему
pub fn step(nodes: &[Node], cur: Option<usize>, dir: i32) -> Option<usize> {
    let ord = order(nodes);
    if ord.is_empty() {
        return None;
    }
    let pos = cur.and_then(|c| ord.iter().position(|&i| i == c));
    let next = match (pos, dir >= 0) {
        (Some(p), true) => (p + 1) % ord.len(),
        (Some(p), false) => (p + ord.len() - 1) % ord.len(),
        (None, true) => 0,
        (None, false) => ord.len() - 1,
    };
    Some(ord[next])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(role: Role, name: &str, visible: bool) -> Node {
        Node { role, name: name.into(), value: String::new(), rect: R::new(0, 0, 10, 10), checked: false, selected: false, enabled: true, fraction: 0.0, visible }
    }

    #[test]
    fn tab_walks_the_interactive_nodes_in_drawing_order_and_wraps() {
        let v = vec![n(Role::Heading, "h", true), n(Role::Button, "a", true), n(Role::Toggle, "b", false), n(Role::Slider, "c", true)];
        assert_eq!(order(&v), vec![1, 2, 3], "headings are skipped, scrolled-away nodes are reachable");
        assert_eq!(step(&v, None, 1), Some(1));
        assert_eq!(step(&v, Some(1), 1), Some(2));
        assert_eq!(step(&v, Some(3), 1), Some(1), "wraps");
        assert_eq!(step(&v, Some(1), -1), Some(3), "shift+tab wraps back");
        assert_eq!(step(&v, None, -1), Some(3));
        assert_eq!(step(&[], None, 1), None);
    }

    #[test]
    fn disabled_nodes_cannot_take_focus() {
        let mut d = n(Role::Button, "x", true);
        d.enabled = false;
        assert!(!d.focusable());
    }
}
