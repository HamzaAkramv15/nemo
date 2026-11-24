// Настройки: группы вкладок слева, содержимое справа. Всё применяется сразу и пишется в
// ~/.config/nemo/shell.json (его можно править и руками — шелл подхватит).
//
// Вкладка — строка в GROUPS: ключ, имя, значок, слова для поиска и функция рисования (каждая в
// своём модуле). Содержимое листается колесом: сколько его, узнаём по зонам, которые вкладка
// нарисовала (Canvas::reach), так что новая строка настроек сама удлиняет прокрутку.

use crate::app::{App, Modal, SKind, State};
use crate::font::PX;
use crate::gfx::{BoxStyle, R};
use crate::ui::{Act, Cur, Tone, Ui};
pub use crate::bartab::{BDrag, BarEd};
use crate::settabs::{tab_about, tab_color, tab_comfort, tab_region, tab_dock, tab_look, tab_notif, tab_shots, tab_system, tab_widgets, tab_windows};
use smithay_client_toolkit::seat::keyboard::{KeyEvent, Keysym};

pub const SIZE: (i32, i32) = (800, 600);
pub(crate) const SIDE_W: i32 = 168;
const ROW_H: i32 = 26;
const HEAD_H: i32 = 22;
/// шаг колеса, пикселей
const WHEEL: i32 = 48;

pub struct Tab {
    pub id: &'static str,
    pub name: &'static str,
    pub icon: &'static str,
    /// слова для поиска из лаунчера («?vrr» → экраны), по-русски и по-английски
    pub find: &'static str,
    pub draw: fn(&mut Ui, &State, R),
    /// у вкладки свой список с прокруткой (клавиши): общую не включать
    pub own_scroll: bool,
}

const fn tab(id: &'static str, name: &'static str, icon: &'static str, find: &'static str, draw: fn(&mut Ui, &State, R)) -> Tab {
    Tab { id, name, icon, find, draw, own_scroll: false }
}

/// группы по смыслу: как выглядит, рабочий стол, что делают руки, система
pub const GROUPS: &[(&str, &[Tab])] = &[
    (
        "внешний вид",
        &[
            tab("color", "цвет", "drop", "палитра акцент цвет тема темы шаблон светлая тёмная стекло прозрачность плотность palette accent colour color theme themes template light dark glass opacity paleta destaque cor cores tema temas modelo claro escuro vidro transparência", tab_color), // i18n-ok
            tab("wall", "обои", "image", "обои картинка фон папка миниатюры skwd skwd-wall видео wallpaper background image folder thumbnails picker video papel parede imagem fundo pasta miniaturas seletor vídeo", crate::wall::tab), // i18n-ok
            tab("fonts", "шрифты", "text", "шрифт размер терминал fastfetch браузер fonts size terminal browser fonte fontes tamanho terminal navegador", crate::fonttab::tab), // i18n-ok
            tab("look", "вид", "eye", "пиксельность пиксели лёгкий режим слабый тормозит виртуалка эффекты развёртка размытие значки иконки курсор pixel pixelation lite slow lag vm animation effects reveal scanlines blur icons cursor visual aparência pixels pixelização leve lento animação efeitos desfoque ícones cursor", tab_look), // i18n-ok
            tab("access", "доступность", "eye", "доступность контраст высокая контрастность движение анимации экранный диктор клавиатура фокус размер текста accessibility contrast high contrast motion reduce motion screen reader keyboard focus text size acessibilidade contraste movimento teclado foco", crate::settabs::tab_access), // i18n-ok
            tab("anim", "анимации", "slides", "анимации появление закрытие скорость длительность узор дизеринг развёртка шум диагональ блоки плавно сразу движение animations reveal open close speed duration pattern dither fade motion animações abrir fechar velocidade duração padrão movimento", crate::animtab::tab), // i18n-ok
        ],
    ),
    (
        "рабочий стол",
        &[
            tab("bar", "бар", "menu", "бар модули часы разделитель плавающий плашка ширина центр трей стрелка температура bar modules clock separator floating pill width center tray arrow temperature barra módulos relógio separador flutuante largura centro bandeja temperatura", crate::bartab::tab_bar), // i18n-ok
            tab("dock", "док", "tile", "док закреплённые dock pinned dock fixados", tab_dock), // i18n-ok
            tab("widgets", "виджеты", "grid", "виджеты график часы заметка widgets graph clock note widgets gráfico relógio nota", tab_widgets), // i18n-ok
            tab("windows", "окна", "float", "окна отступы плавающие стол заголовки углы скругление рамка gaps floating stacking windows titlebars corners rounded radius border janelas espaço flutuantes títulos cantos arredondados borda mosaico", tab_windows), // i18n-ok
            tab("screens", "экраны", "screen", "экраны монитор разрешение частота масштаб vrr hz герц поворот display monitor resolution refresh scale rotation telas tela monitor resolução frequência escala rotação", crate::displays::tab), // i18n-ok
        ],
    ),
    (
        "каждый день",
        &[
            tab("comfort", "комфорт", "coffee", "комфорт кофе не засыпать бодрствование сон блокировка экран гаснет простой ночной свет тёплый не беспокоить рыбка характер comfort caffeine coffee keep awake sleep idle lock screen off night light warm do not disturb fish character personality conforto café não dormir inatividade luz noturna não perturbe", tab_comfort), // i18n-ok
            tab("region", "язык и время", "clock", "язык и время часы 12 24 формат дата неделя выходные пятница суббота хиджра арабские цифры календарь language time clock 12 24 hour format date week start weekend friday saturday hijri arabic digits calendar idioma horário relógio formato data semana fim de semana", tab_region), // i18n-ok
            tab("notif", "уведомления", "bell", "уведомления не беспокоить место история osd громкость яркость notifications do not disturb position history osd volume brightness notificações notificação perturbe posição histórico volume brilho", tab_notif), // i18n-ok
            Tab { own_scroll: true, ..tab("keys", "клавиши", "keys", "клавиши сочетания хоткеи раскладка keys shortcuts hotkeys layout keyboard teclas atalhos teclado layout disposição", crate::keys::tab) }, // i18n-ok
            tab("shots", "скриншоты", "camera", "скриншоты снимок лупа screenshot loupe captura capturas tela imagem lupa", tab_shots), // i18n-ok
            tab("games", "игры", "play", "игры steam proton разрывы gamemode ntsync tearing games shader jogos jogo cortes shader", crate::games::tab), // i18n-ok
        ],
    ),
    (
        "система",
        &[
            tab("system", "сеанс", "gear", "система язык сон блокировка ожидание ночной свет system language idle sleep lock night light sistema idioma língua português brasileiro brasil suspender bloqueio inatividade luz noturna", tab_system), // i18n-ok
            tab("startup", "автозапуск", "play", "автозапуск запуск при входе программы сеанс startup autostart login apps session inicialização iniciar programas sessão", crate::startup::tab), // i18n-ok
            tab("login", "экран входа", "lock", "экран входа вход логин greetd пользователи пароль numlock вуаль менеджер входа login screen greeter display manager users password numlock veil tela de login entrada greetd usuários senha numlock véu gerenciador de login", crate::login::tab), // i18n-ok
            tab("bt", "bluetooth", "bt", "bluetooth блютуз наушники колонки мышь клавиатура сопряжение подключить забыть заряд bluetooth headphones speaker mouse keyboard pair connect forget battery fones caixa de som mouse teclado parear conectar esquecer bateria", crate::bttab::tab), // i18n-ok
            tab("agents", "службы", "bolt", "службы трей polkit порталы флешки телефоны автоподключение обложки тексты погода services tray portals automount drives phones covers lyrics weather serviços serviço bandeja portais discos telefones montagem capas letras clima", crate::agents::tab), // i18n-ok
            tab("updates", "обновления", "down", "обновления версия откат updates version rollback atualizações atualizar versão reverter система софт дистрибутив пакеты fedora dnf arch pacman debian ubuntu apt opensuse zypper безопасные удержано ядро system software distribution packages upgrade held back kernel safe sistema atualizações do sistema pacotes", crate::update::tab), // i18n-ok
        ],
    ),
    (
        "диагностика",
        &[
            tab("diag", "состояние", "gauge", "состояние диагностика здоровье память процессы версии журнал отчёт проверка зависимости diagnostics health status memory processes versions log report doctor dependencies", crate::diagtab::tab_diag), // i18n-ok
            tab("about", "о шелле", "about", "о шелле версия память about version memory sobre versão memória", tab_about), // i18n-ok
        ],
    ),
];

/// одна строка про вкладку: что на ней и зачем; показывается под заголовком окна. Ключ — id вкладки
pub fn desc(id: &str) -> &'static str {
    match id {
        "color" => "палитра, акцент и прозрачность: цвета всего рабочего стола и программ",
        "wall" => "обои: своя картинка, папка с картинками или рисунок палитры",
        "fonts" => "шрифты шелла, терминала и браузера",
        "look" => "вид деталей: пиксельность, углы, размытие, значки, лёгкий режим",
        "anim" => "как окна и панели появляются и исчезают",
        "access" => "контраст, меньше движения, работа с клавиатуры и экранным диктором",
        "bar" => "что показывает полоса сверху: модули, форма, часы",
        "dock" => "док с программами: край, размер, прятать, числа на значках",
        "widgets" => "виджеты на рабочем столе: добавить, переставить, настроить",
        "windows" => "как окна расставляются и выглядят: отступы, углы, заголовки",
        "screens" => "мониторы: разрешение, частота, масштаб, поворот",
        "comfort" => "экран и питание: засыпание, блокировка, кофе, ночной свет",
        "region" => "язык, формат времени и даты, календарь",
        "notif" => "уведомления: где показываются, сколько, не беспокоить",
        "keys" => "клавиши: что делает каждое сочетание, свои сочетания",
        "shots" => "скриншоты: куда сохраняются и что делать после снимка",
        "games" => "игры: что включается на время игры",
        "system" => "сеанс: язык, сон, блокировка, строки nemo в конфиге niri",
        "startup" => "что запускается при входе: включить или выключить",
        "login" => "экран входа: вид, пользователи, numlock, менеджер входа",
        "bt" => "bluetooth: устройства, сопряжение, заряд",
        "agents" => "службы: окна паролей, трей, порталы, диски, погода",
        "updates" => "обновления Nemo и системы, проверка подписей, откат",
        "diag" => "здоровье рабочего стола: память, версии, нехватка программ, журнал решений",
        "about" => "о Nemo: версия, авторы, модули",
        _ => "",
    }
}

/// все вкладки подряд
pub fn tabs() -> impl Iterator<Item = &'static Tab> {
    GROUPS.iter().flat_map(|g| g.1.iter())
}

/// вкладка по ключу (из IPC, лаунчера, `shot settings:<id>`)
pub fn find(id: &str) -> Option<&'static Tab> {
    tabs().find(|t| t.id == id)
}

/// где рисовать содержимое (видимая часть) — общее для рисования и колеса
pub(crate) fn body_rect(w: i32, h: i32, sw: i32) -> R {
    let fr = R::new(0, 0, w - 6, h - 6);
    R::new(1 + sw + 20, 56, fr.right() - sw - 1 - 48, fr.h - 72)
}

fn side_rect(h: i32, sw: i32) -> R {
    R::new(1, 31, sw, h - 6 - 32)
}

/// ширина колонки вкладок: по самой длинной вкладке на языке интерфейса (немецкие длиннее
/// русских), не уже SIDE_W и не шире SIDE_W + 36; высоты от неё не зависят
fn side_w(ui: &mut Ui, st: &State) -> i32 {
    let longest = GROUPS.iter().flat_map(|g| g.1.iter()).filter(|tb| st.mods_here.shows(tb.id)).map(|tb| ui.w(PX, tb.name)).max().unwrap_or(0);
    // поля колонки 2×8, значок 14 + 7, по 8 у краёв кнопки
    (longest + 16 + 21 + 16).clamp(SIDE_W, SIDE_W + 36)
}

/// прокрутка открытой вкладки (сдвиг запомнен для неё; другая вкладка начинается сверху)
pub fn scroll_of(st: &State) -> i32 {
    if st.sscroll.1 == st.stab {
        st.sscroll.0
    } else {
        0
    }
}

/// полоса прокрутки: клетки дорожки и бегунок
pub(crate) fn scrollbar(ui: &mut Ui, r: R, view: i32, full: i32, at: i32) {
    let t = ui.t;
    if full <= view {
        return;
    }
    ui.cv.fill(R::new(r.x + 1, r.y, 2, r.h), t.border.alpha(0.5));
    let th = (r.h * view / full).max(18);
    let ty = r.y + (r.h - th) * at / (full - view).max(1);
    ui.cv.fill(R::new(r.x, ty, 4, th), t.red);
}

/// вкладка подходит запросу: все слова запроса есть в её названии (как на экране и по-русски) или в
/// словах-подсказках (на всех языках сразу), без учёта регистра
/// высота поля поиска над списком вкладок
const SEARCH_H: i32 = 36;
/// вкладка в списке слева: модуль есть и она подходит запросу
fn listed(st: &State, tb: &Tab, query: &str) -> bool {
    st.mods_here.shows(tb.id) && tab_matches(tb, query)
}

pub fn tab_matches(tb: &Tab, query: &str) -> bool {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return true;
    }
    let hay = format!("{} {} {}", tb.name, crate::i18n::tr(tb.name), tb.find).to_lowercase();
    q.split_whitespace().all(|w| hay.contains(w)) || crate::setindex::tab_has_hit(tb.id, query)
}

/// вкладки, подходящие запросу, по порядку
pub fn matching(st: &State, query: &str) -> Vec<&'static Tab> {
    tabs().filter(|tb| st.mods_here.shows(tb.id) && tab_matches(tb, query)).collect()
}

pub fn draw(ui: &mut Ui, st: &State, w: i32, h: i32) {
    if st.welcome.is_some() {
        return crate::welcome::draw(ui, st, w, h);
    }
    let t = ui.t;
    let fr = R::new(0, 0, w - 6, h - 6);
    ui.frame(fr, false);
    let sub = crate::tf!("Nemo {} · {:.0} МБ", "Nemo {} · {:.0} MB", "Nemo {} · {:.0} MB", nemo_px::brand::VERSION, st.rss);
    ui.header(R::new(1, 1, fr.w - 2, 30), "gear", "настройки", &sub);
    // за заголовок окно двигается (крестик зарегистрирован позже — он сверху)
    ui.zone(R::new(1, 1, fr.w - 2, 30), Act::MDrag, Cur::Default);
    ui.ghost(R::new(fr.right() - 28, 5, 24, 22), "close", Act::CloseModal);
    // «отменить»: последнее изменение настроек (снимки в памяти, Ctrl+Z); слева от «сбросить страницу»
    let undo_w = if st.set_history.is_empty() { 0 } else { ui.w(PX, "отменить") + 40 };
    if undo_w > 0 {
        let page = if crate::config::page_keys(st.stab).is_empty() { 0 } else { 190 };
        ui.button(R::new(fr.right() - 34 - undo_w - page - 8, 5, undo_w, 22), "отменить", "reboot", Tone::Ghost, false, true, Act::SettingsUndo);
    }
    // «сбросить страницу»: у страниц с ключами настроек; первый щелчок спрашивает, второй (пять секунд) сбрасывает
    if !crate::config::page_keys(st.stab).is_empty() {
        let armed = st.reset_armed.is_some_and(|(tab, at)| tab == st.stab && at.elapsed().as_secs() < 5);
        let label = if armed { "нажми ещё раз: сбросить" } else { "сбросить страницу" };
        let bw = (ui.w(PX, label) + 40).min(fr.w - 330);
        ui.button(R::new(fr.right() - 34 - bw, 5, bw, 22), label, "reboot", if armed { Tone::Primary } else { Tone::Ghost }, armed, true, Act::SCycle("reset_page"));
    }
    // вкладки группами; список длиннее окна — листается колесом
    let sw = side_w(ui, st);
    let side = side_rect(h, sw);
    ui.cv.fill(side, t.panel.alpha(0.5));
    ui.cv.fill(R::new(side.right(), side.y, 1, side.h), t.border);
    ui.zone(side, Act::SSideScroll, Cur::Default);
    // поиск по вкладкам над списком: пишешь «ночь», «часы», «blur» — остаются подходящие вкладки
    let query = st.search_field.text.clone();
    let focused = st.field_focus == Some("set_search");
    ui.field(R::new(side.x + 8, side.y + 6, side.w - 16, 24), &st.search_field, "поиск", focused, false, Act::SField("set_search"));
    ui.cv.push_clip(R::new(side.x, side.y + SEARCH_H, side.w, side.h - SEARCH_H));
    let mut y = side.y + SEARCH_H + 8 - st.stab_scroll;
    // раскрыта группа открытой вкладки (щелчок по другой группе открывает её первую вкладку); при поиске — все, где есть совпадения.
    // Так список всегда умещается в окно: пять заголовков и вкладки одной группы, а не тридцать строк с прокруткой
    let searching = !query.trim().is_empty();
    for (gname, tabs) in GROUPS {
        let Some(first) = tabs.iter().find(|tb| listed(st, tb, &query)) else { continue };
        let open = searching || tabs.iter().any(|tb| tb.id == st.stab);
        let hr = R::new(side.x + 6, y, side.w - 12, HEAD_H + 2);
        let hov = ui.hover(hr);
        if hov {
            ui.cv.fill(hr, t.surface_hi.alpha(0.4));
        }
        ui.cv.icon(if open { "down" } else { "right" }, side.x + 10, y + 5, 1, if open { t.red } else { t.text_faint });
        ui.text_in(PX, R::new(side.x + 26, y, side.w - 36, HEAD_H), gname, if open { t.text } else { t.text_dim });
        if !open {
            ui.zone(hr, Act::STab(first.id), Cur::Pointer);
        }
        y += HEAD_H + 2;
        if !open {
            y += 2;
            continue;
        }
        // вкладки модулей, которых нет (скриншоты без nemo-shot, экран входа без nemo-greet), прячутся
        for tb in tabs.iter().filter(|tb| listed(st, tb, &query)) {
            let r = R::new(side.x + 8, y, side.w - 16, ROW_H - 2);
            ui.button(r, tb.name, tb.icon, Tone::Ghost, st.stab == tb.id, true, Act::STab(tb.id));
            if st.stab == tb.id {
                ui.cv.fill(R::new(r.x - 8, r.y, 3, r.h), t.red);
            }
            y += ROW_H;
        }
        y += 8;
    }
    let side_full = y + st.stab_scroll - side.y;
    st.stab_h.set(side_full);
    ui.cv.pop_clip();
    scrollbar(ui, R::new(side.right() - 5, side.y + 2, 4, side.h - 4), side.h, side_full, st.stab_scroll);
    // содержимое: фон-зона для колеса, вкладка со сдвигом, полоса прокрутки справа
    let body = body_rect(w, h, sw);
    let tb = find(st.stab).unwrap_or(&GROUPS[0].1[0]);
    let off = if tb.own_scroll { 0 } else { scroll_of(st) };
    ui.zone(R::new(side.right() + 1, 31, fr.right() - side.right() - 2, fr.h - 32), Act::SScroll, Cur::Default);
    // что на этой вкладке и зачем: одна строка под заголовком окна
    if st.search_field.text.trim().is_empty() {
        let d = (ui.tr)(desc(tb.id)).to_string();
        let d = ui.f.elide(PX, &d, body.w);
        ui.text_in(PX, R::new(body.x, 32, body.w, 16), &d, t.text_faint);
    }
    ui.cv.push_clip(R::new(body.x - 4, body.y - 4, body.w + 8, body.h + 8));
    ui.cv.reach = i32::MIN;
    let z0 = ui.zones.len();
    if st.search_field.text.trim().is_empty() {
        (tb.draw)(ui, st, R::new(body.x, body.y - off, body.w, body.h));
    } else {
        crate::setindex::draw_results(ui, st, R::new(body.x, body.y - off, body.w, body.h));
    }
    // отмеченные строки: настройка не заводская — точка слева; правый щелчок по строке возвращает её (setclick.rs)
    let marks: Vec<R> = ui.zones[z0..].iter().filter(|z| z.act.setting_key().is_some_and(|k| st.set.modified(k))).map(|z| z.r).collect();
    for r in marks {
        ui.cv.fill(R::new(r.x - 4, r.y + 3, 3, r.h.min(16) - 6), t.red);
    }
    // кольцо фокуса обхода клавишей Tab (a11y.rs): рамка выбранного элемента, на обоих окнах — видно при любой палитре
    st.a11y_ring.set(None);
    if let Some(n) = st.a11y_focus.and_then(|i| st.a11y_nodes.borrow().get(i).cloned()) {
        let r = R::new(n.rect.x - ui.cv.ox - 3, n.rect.y - ui.cv.oy - 3, n.rect.w + 6, n.rect.h + 6);
        ui.cv.outline(r, t.text_hi, 2);
        ui.cv.outline(r.inset(2), t.bg, 1);
        st.a11y_ring.set(Some(n.rect));
    }
    let full = if tb.own_scroll { body.h } else { ui.cv.reach.saturating_add(off + 12 - body.y).max(body.h) };
    st.sscroll_h.set(full);
    ui.cv.pop_clip();
    scrollbar(ui, R::new(body.right() + 18, body.y, 4, body.h), body.h, full, off);
}

impl App {
    /// колесо над вкладкой: листать, если есть куда (true — пролистали)
    pub fn settings_scroll(&mut self, d: i32) -> bool {
        let (w, h) = SIZE;
        let (view, own) = match &self.st.welcome {
            Some(wz) => (crate::welcome::body_rect(wz, w, h).h, wz.own_scroll()),
            None => (body_rect(w, h, SIDE_W).h, find(self.st.stab).is_some_and(|t| t.own_scroll)),
        };
        let max = (self.st.sscroll_h.get() - view).max(0);
        if max == 0 || own {
            return false;
        }
        let at = (scroll_of(&self.st) + d * WHEEL).clamp(0, max);
        self.st.sscroll = (at, self.st.stab);
        self.dirty(&SKind::Modal);
        true
    }

    /// колесо над списком вкладок
    pub fn settings_side_scroll(&mut self, d: i32) {
        let h = SIZE.1;
        let max = (self.st.stab_h.get() - side_rect(h, SIDE_W).h).max(0);
        self.st.stab_scroll = (self.st.stab_scroll + d * WHEEL).clamp(0, max);
        self.dirty(&SKind::Modal);
    }
}

pub fn title(ui: &mut Ui, x: i32, y: i32, w: i32, s: &str) -> i32 {
    let t = ui.t;
    ui.cv.fill(R::new(x, y + 7, 6, 6), t.red);
    ui.text_in(PX, R::new(x + 14, y, w - 14, 20), s, t.text);
    ui.cv.fill(R::new(x, y + 24, w, 1), t.border);
    y + 34
}

/// число в пикселях: ‹ значение › — щелчок по стрелке или колесо меняют на шаг
pub fn num_btn(ui: &mut Ui, r: R, label: &str, v: i32, key: &'static str) {
    let t = ui.t;
    let hov = ui.hover(r);
    ui.cv.pxbox(r, &BoxStyle::new(if hov { t.surface_hi } else { t.surface }, if hov { t.red } else { t.border }).bevel(true));
    ui.text_in(PX, R::new(r.x + 8, r.y, r.w - 84, r.h), label, t.text_dim);
    ui.zone(r, Act::SNum(key, 1), Cur::Pointer);
    let (l, rr) = (R::new(r.right() - 72, r.y + 2, 16, r.h - 4), R::new(r.right() - 18, r.y + 2, 16, r.h - 4));
    for (b, icon) in [(l, "left"), (rr, "right")] {
        ui.cv.icon(icon, b.x + 1, b.cy() - 7, 2, if ui.hover(b) { t.ember } else { t.red });
    }
    ui.zone(l, Act::SNum(key, -1), Cur::Pointer);
    ui.zone(rr, Act::SNum(key, 1), Cur::Pointer);
    ui.cv.text_center(ui.f, PX, R::new(l.right(), r.y, rr.x - l.right(), r.h), &format!("{v}"), t.text);
}

/// кнопка выбора: подпись слева, значение справа; подписи — всё место, что значение не заняло
pub fn opt_btn(ui: &mut Ui, r: R, label: &str, val: &str, act: Act) {
    let t = ui.t;
    let hov = ui.hover(r);
    ui.cv.pxbox(r, &BoxStyle::new(if hov { t.surface_hi } else { t.surface }, if hov { t.red } else { t.border }).bevel(true));
    // значение не наезжает на подпись: длинное — с многоточием, подписи — не меньше трети
    let lw = ui.f.width(PX, (ui.tr)(label)).min(r.w / 3);
    let room = r.w - 32 - lw - 12;
    let tv = (ui.tr)(val).to_string();
    let v = if ui.f.width(PX, &tv) > room { ui.f.elide(PX, &tv, room) } else { tv };
    let vw = ui.f.width(PX, &v);
    ui.text_in(PX, R::new(r.x + 8, r.y, r.w - 32 - vw - 12, r.h), label, t.text_dim);
    ui.cv.text_right(ui.f, PX, R::new(r.x, r.y, r.w - 24, r.h), &v, t.text);
    ui.cv.icon("right", r.right() - 18, r.cy() - 7, 2, t.red);
    ui.zone(r, act, Cur::Pointer);
}

/// следующее значение из ряда (после последнего — первое)
pub fn step_in(list: &[i32], cur: i32) -> i32 {
    list.iter().copied().find(|v| *v > cur).unwrap_or(list[0])
}

pub(crate) fn cycle<'a>(list: &[(&'a str, &'a str)], cur: &str) -> &'a str {
    let i = list.iter().position(|x| x.0 == cur).map(|i| (i + 1) % list.len()).unwrap_or(0);
    list[i].0
}

impl App {
    /// что есть в системе для открытой вкладки: игры (gamemode, ntsync…), skwd-wall для обоев.
    /// При открытии вкладки и настроек — не в каждом кадре
    pub fn tab_probe(&mut self) {
        match self.st.stab {
            "games" => self.st.game_probe = crate::gameboost::probe(),
            "system" => self.st.niri_link = crate::niricfg::status(),
            "wall" => self.st.skwd_here = crate::skwd::installed(),
            "updates" if self.st.upd_page == 1 => {
                let tx = self.tx.clone();
                crate::sysupd::start(&mut self.st, tx, false);
            }
            "login" => self.st.login_probe = crate::login::probe(),
            "diag" => self.st.diag = Some(crate::diagtab::collect()),
            "look" if self.st.sets.is_none() => self.sets_load(),
            _ => {}
        }
    }

    pub fn settings_key(&mut self, e: &KeyEvent) {
        if self.keys_key(e) {
            return;
        }
        // Tab, Enter, стрелки: обход окна с клавиатуры (a11y.rs); в поле ввода Tab не нужен — там своё
        if self.st.field_focus.is_none() && self.a11y_key(e) {
            return;
        }
        // Ctrl+Z вне полей ввода: отменить последнее изменение
        if self.st.field_focus.is_none() && self.mods.ctrl && matches!(e.keysym, Keysym::z | Keysym::Z) {
            self.settings_click(&Act::SettingsUndo);
            return;
        }
        if let Some(f) = self.st.field_focus {
            match e.keysym {
                Keysym::Escape => {
                    if f == "set_search" {
                        self.st.search_field.set("");
                    }
                    self.st.field_focus = None;
                }
                Keysym::Return | Keysym::KP_Enter => {
                    if f == "set_search" {
                        // Enter открывает первую подходящую вкладку
                        let q = self.st.search_field.text.clone();
                        let first = crate::setindex::search(&q).first().map(|e| e.tab).or_else(|| matching(&self.st, &q).first().map(|t| t.id));
                        if let Some(id) = first {
                            self.st.stab = id;
                            self.st.sscroll = (0, id);
                            self.st.search_field.set("");
                            self.tab_probe();
                        }
                        self.st.field_focus = None;
                    } else if f == "hex" {
                        self.apply_hex();
                    } else if f == "float_apps" {
                        self.settings_click(&Act::SCycle("float_apply"));
                    } else if f == "notif_muted" {
                        self.settings_click(&Act::SCycle("muted_apply"));
                    } else if f == "cast_hide" {
                        self.settings_click(&Act::SCycle("cast_apply"));
                    } else if f == "shot_dir" {
                        self.settings_click(&Act::SCycle("shot_dir_apply"));
                    } else if f == "game_apps" {
                        self.settings_click(&Act::SCycle("game_apply"));
                    } else if f == "night_geo" {
                        self.settings_click(&Act::SCycle("geo_apply"));
                    } else if f == "weather_city" {
                        self.settings_click(&Act::SCycle("weather_apply"));
                    } else if f == "kb_layout" {
                        self.settings_click(&Act::SCycle("kb_apply"));
                    } else if f == "key_cmd" {
                        self.settings_click(&Act::KeyAdd);
                    } else if f == "w_pass" {
                        self.settings_click(&Act::WInstall);
                    } else {
                        self.settings_click(&Act::SCycle("wall_apply"));
                    }
                }
                _ => {
                    let mods = self.mods;
                    let field = match f {
                        "hex" => &mut self.st.hex,
                        "shot_dir" => &mut self.st.shot_field,
                        "float_apps" => &mut self.st.float_field,
                        "notif_muted" => &mut self.st.muted_field,
                        "cast_hide" => &mut self.st.cast_field,
                        "key_cmd" => &mut self.st.keys.cmd,
                        "game_apps" => &mut self.st.game_field,
                        "night_geo" => &mut self.st.geo_field,
                        "weather_city" => &mut self.st.weather_field,
                        "set_search" => &mut self.st.search_field,
                        "kb_layout" => &mut self.st.keys.lay,
                        "w_pass" => match self.st.welcome.as_mut() {
                            Some(w) => &mut w.pass,
                            None => &mut self.st.wall_field,
                        },
                        _ => &mut self.st.wall_field,
                    };
                    App::edit_field(field, e, &mods);
                }
            }
            self.dirty(&SKind::Modal);
            return;
        }
        // первая настройка: Enter и Tab — по шагам, Esc закрывает
        if let Some(at) = self.st.welcome.as_ref().map(|w| w.step) {
            match e.keysym {
                // Esc спрашивает, а вопрос Esc и Enter просто убирают: случайно не закрыть
                Keysym::Escape => {
                    let asking = self.st.welcome.as_ref().is_some_and(|w| w.ask);
                    self.settings_click(&if asking { Act::WLeave(0) } else { Act::CloseModal });
                    return;
                }
                Keysym::Return | Keysym::KP_Enter => {
                    let asking = self.st.welcome.as_ref().is_some_and(|w| w.ask);
                    self.settings_click(&if asking { Act::WLeave(0) } else { Act::WNext });
                    return;
                }
                Keysym::Tab | Keysym::ISO_Left_Tab => {
                    let back = e.keysym == Keysym::ISO_Left_Tab || self.mods.shift;
                    self.settings_click(&if back { Act::WStep(at.saturating_sub(1)) } else { Act::WStep(at + 1) });
                    return;
                }
                _ => {}
            }
        }
        match e.keysym {
            Keysym::Escape => self.close_modal(),
            Keysym::Tab | Keysym::ISO_Left_Tab | Keysym::Down | Keysym::Up => {
                let back = e.keysym == Keysym::ISO_Left_Tab || e.keysym == Keysym::Up || self.mods.shift;
                let ids: Vec<&'static str> = tabs().filter(|t| self.st.mods_here.shows(t.id)).map(|t| t.id).collect();
                let n = ids.len();
                let i = ids.iter().position(|x| *x == self.st.stab).unwrap_or(0);
                self.st.stab = ids[if back { (i + n - 1) % n } else { (i + 1) % n }];
                self.tab_probe();
            }
            // листать вкладку: страница, начало, конец
            Keysym::Page_Down | Keysym::Page_Up => {
                let page = (body_rect(SIZE.0, SIZE.1, SIDE_W).h - 40) / WHEEL;
                self.settings_scroll(if e.keysym == Keysym::Page_Down { page } else { -page });
            }
            Keysym::Home => {
                self.settings_scroll(-1000);
            }
            Keysym::End => {
                self.settings_scroll(1000);
            }
            _ => {}
        }
        if self.st.modal == Some(Modal::Settings) {
            self.dirty(&SKind::Modal);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tab_of(id: &str) -> &'static Tab {
        find(id).expect(id)
    }

    /// поиск по вкладкам: пустой запрос — все; слова — на любом языке из подсказок; все слова сразу; регистр не важен
    #[test]
    fn the_search_finds_tabs_by_their_words() {
        let system = tab_of("system");
        assert!(tab_matches(system, ""));
        assert!(tab_matches(system, "   "));
        assert!(tab_matches(system, "night"));
        assert!(tab_matches(system, "NIGHT Light"));
        assert!(tab_matches(system, "ночной")); // i18n-ok
        assert!(!tab_matches(system, "night zzz"));
        assert!(!tab_matches(system, "wallpaper"));
        // другая вкладка по своему слову
        assert!(tab_matches(tab_of("games"), "proton"));
        assert!(!tab_matches(tab_of("games"), "night"));
    }

    /// у каждого слова из подсказок есть хотя бы одна вкладка, а запрос «настрой» не пуст: поиск не тупик
    #[test]
    fn every_tab_can_be_found_by_its_own_name() {
        for tb in tabs() {
            assert!(tab_matches(tb, tb.name), "{} is not found by its own name", tb.id);
            assert!(tab_matches(tb, tb.id) || tab_matches(tb, tb.name), "{}", tb.id);
        }
    }
}
