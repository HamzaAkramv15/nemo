// Характер Nemo: рыбка иногда говорит, одной строкой, и только о событиях, которые шелл и так видит
// (зарядка, кофе, обновление). Без опроса по таймеру, без сети, без фоновой работы: функция
// вызывается из мест, где событие уже случилось. Настройка `personality`: off — ни слова, quiet (по
// умолчанию) — только эти системные события, playful — ещё пустые панели. Сообщения на английском,
// арабском и русском; на языке без сообщения не показывается ничего (не английское вместо него).
// Не говорит при полном экране, трансляции и «не беспокоить», не чаще раза в 45 секунд вообще и раза в
// 10 минут про одно и то же. Ошибки и всё, что требует чёткого сообщения, остаются сухими.

use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Event {
    ChargerOn,
    ChargerOff,
    BatteryFull,
    CaffeineEnded,
    UpdateReady,
    /// вернулся к заблокированному экрану
    Unlocked,
    /// очистил все уведомления (только в игривом режиме)
    NotifsCleared,
}

const EVENTS: usize = 7;

impl Event {
    /// эти события — только в игривом режиме: тихий характер говорит про систему, а не про каждое действие
    fn playful_only(self) -> bool {
        matches!(self, Event::Unlocked | Event::NotifsCleared)
    }

    fn idx(self) -> usize {
        self as usize
    }
}

/// фразы про событие на языке интерфейса; пустой список — на этом языке их нет. Их несколько: рыбка не повторяется слово в слово
fn lines(ev: Event, lang: &str) -> &'static [&'static str] {
    // i18n-ok: фразы характера лежат здесь на трёх языках, а не в таблицах перевода
    match (ev, lang) {
        (Event::ChargerOn, "en") => &["Plugged in. The tide is coming in.", "Charging. Back to the deep end soon.", "Power again. Take a breath."],
        (Event::ChargerOn, "ar") => &["تم التوصيل. المدّ قادم.", "يجري الشحن. نعود إلى الأعماق قريبًا."],
        (Event::ChargerOn, "ru") => &["Подключено. Прилив начался.", "Зарядка пошла. Скоро снова на глубину.", "Питание вернулось. Можно выдохнуть."], // i18n-ok
        (Event::ChargerOff, "en") => &["Unplugged. Swimming on battery.", "Off the wall. Let's keep it short.", "On our own fins now."],
        (Event::ChargerOff, "ar") => &["فُصل الشاحن. نسبح على البطارية.", "نسبح بزعانفنا وحدنا الآن."],
        (Event::ChargerOff, "ru") => &["Отключено. Плывём на батарее.", "Без розетки. Давай недолго.", "Теперь на своих плавниках."], // i18n-ok
        (Event::BatteryFull, "en") => &["Full tank. You can unplug.", "Topped up. The cable can rest."],
        (Event::BatteryFull, "ar") => &["اكتمل الشحن. يمكنك فصله.", "امتلأ الخزان. يمكن للكابل أن يرتاح."],
        (Event::BatteryFull, "ru") => &["Полный бак. Можно отключать.", "Налито до краёв. Кабель может отдохнуть."], // i18n-ok
        (Event::CaffeineEnded, "en") => &["Your eyes could use a rest, too.", "Coffee's over. Screen can sleep again."],
        (Event::CaffeineEnded, "ar") => &["عيناك أيضًا تحتاجان إلى راحة.", "انتهت القهوة. يمكن للشاشة أن تنام."],
        (Event::CaffeineEnded, "ru") => &["И глазам пора отдохнуть.", "Кофе кончился. Экран снова может спать."], // i18n-ok
        (Event::UpdateReady, "en") => &["Fresh water is ready.", "A cleaner tank is waiting for you."],
        (Event::UpdateReady, "ar") => &["الماء العذب جاهز.", "ينتظرك حوض أنظف."],
        (Event::UpdateReady, "ru") => &["Свежая вода готова.", "Чистый аквариум ждёт тебя."], // i18n-ok
        (Event::Unlocked, "en") => &["Welcome back.", "There you are.", "Still here. Still swimming."],
        (Event::Unlocked, "ar") => &["أهلًا بعودتك.", "ها أنت ذا."],
        (Event::Unlocked, "ru") => &["С возвращением.", "Вот и ты.", "Я здесь. Всё ещё плаваю."], // i18n-ok
        (Event::NotifsCleared, "en") => &["Clear water.", "All quiet. Nicely done.", "Nothing left to read."],
        (Event::NotifsCleared, "ar") => &["ماء صافٍ.", "هدوء تام. أحسنت."],
        (Event::NotifsCleared, "ru") => &["Вода чистая.", "Тишина. Славно.", "Читать больше нечего."], // i18n-ok
        _ => &[],
    }
}

/// фраза про событие; `n` выбирает вариант (по времени), на языке без фраз — None
pub fn quip(ev: Event, lang: &str, n: usize) -> Option<&'static str> {
    let l = lines(ev, lang);
    (!l.is_empty()).then(|| l[n % l.len()])
}

/// не чаще раза в это время вообще
pub const GAP: Duration = Duration::from_secs(45);
/// не чаще раза в это время про одно событие
pub const EACH: Duration = Duration::from_secs(600);

#[derive(Default)]
pub struct Gate {
    last: Option<Instant>,
    each: [Option<Instant>; EVENTS],
}

impl Gate {
    /// можно ли сказать сейчас; если да — запоминает. busy: полный экран, трансляция или «не беспокоить»
    pub fn allow(&mut self, ev: Event, now: Instant, level: &str, busy: bool) -> bool {
        if level == "off" || busy || ev.playful_only() && level != "playful" {
            return false;
        }
        let spaced = |t: Option<Instant>, gap: Duration| t.is_none_or(|t| now.saturating_duration_since(t) >= gap);
        if !spaced(self.last, GAP) || !spaced(self.each[ev.idx()], EACH) {
            return false;
        }
        self.last = Some(now);
        self.each[ev.idx()] = Some(now);
        true
    }
}

impl crate::app::App {
    /// фраза для события, если сейчас можно и она есть на этом языке
    pub fn quip_for(&mut self, ev: Event) -> Option<String> {
        let busy = self.st.game.fullscreen || self.st.curtain.is_some() || self.st.set.dnd;
        // сначала язык: без фразы на нём лимит не тратится
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as usize);
        let text = quip(ev, crate::i18n::code(), n)?;
        self.st.quip.allow(ev, Instant::now(), &self.st.set.personality, busy).then(|| text.to_string())
    }

    /// зарядка включена/выключена/полная: следим за переходами, не за уровнем
    pub fn quip_power(&mut self) {
        let Some(b) = self.st.sys.bat else { return };
        let ac = b.charging || b.plugged;
        let prev = self.st.ac_prev.replace(ac);
        let ev = match prev {
            Some(false) if ac => Some(Event::ChargerOn),
            Some(true) if !ac => Some(Event::ChargerOff),
            _ => None,
        };
        if b.charging {
            self.st.full_said = false;
        } else if b.plugged && b.pct >= 95 && !self.st.full_said {
            self.st.full_said = true;
            if let Some(q) = self.quip_for(Event::BatteryFull) {
                self.toast_note("Nemo", &q);
            }
        }
        if let Some(q) = ev.and_then(|e| self.quip_for(e)) {
            self.toast_note("Nemo", &q);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Event; EVENTS] = [Event::ChargerOn, Event::ChargerOff, Event::BatteryFull, Event::CaffeineEnded, Event::UpdateReady, Event::Unlocked, Event::NotifsCleared];

    #[test]
    fn every_event_speaks_english_arabic_and_russian_and_no_other_language() {
        for e in ALL {
            for l in ["en", "ar", "ru"] {
                assert!(quip(e, l, 0).is_some_and(|s| s.len() > 8), "{e:?} {l}");
                assert!(lines(e, l).len() >= 2, "{e:?} {l}: more than one way to say it");
            }
            for l in ["de", "fr", "es", "pt-BR", "uk", "pl", ""] {
                assert_eq!(quip(e, l, 0), None, "{e:?} must stay silent in {l}, not fall back to English");
            }
        }
        // арабский — арабскими буквами, не копия английского
        assert!(quip(Event::ChargerOn, "ar", 0).unwrap().chars().any(|c| ('\u{0600}'..='\u{06ff}').contains(&c)));
    }

    #[test]
    fn variants_rotate_and_wrap() {
        assert_ne!(quip(Event::Unlocked, "en", 0), quip(Event::Unlocked, "en", 1));
        assert_eq!(quip(Event::Unlocked, "en", 0), quip(Event::Unlocked, "en", 3));
    }

    #[test]
    fn small_talk_only_in_playful_mode() {
        let (mut g, t) = (Gate::default(), Instant::now());
        assert!(!g.allow(Event::Unlocked, t, "quiet", false));
        assert!(!g.allow(Event::NotifsCleared, t, "quiet", false));
        assert!(g.allow(Event::Unlocked, t, "playful", false));
    }

    #[test]
    fn off_and_busy_are_silent() {
        let (mut g, t) = (Gate::default(), Instant::now());
        assert!(!g.allow(Event::ChargerOn, t, "off", false));
        assert!(!g.allow(Event::ChargerOn, t, "quiet", true), "never during a game, a cast or do-not-disturb");
        // молчание не тратит лимит
        assert!(g.allow(Event::ChargerOn, t, "quiet", false));
    }

    #[test]
    fn it_never_talks_twice_in_a_short_while() {
        let (mut g, t) = (Gate::default(), Instant::now());
        assert!(g.allow(Event::ChargerOn, t, "quiet", false));
        assert!(!g.allow(Event::ChargerOff, t + Duration::from_secs(10), "quiet", false), "45 s between any two");
        assert!(g.allow(Event::ChargerOff, t + Duration::from_secs(50), "quiet", false));
        assert!(!g.allow(Event::ChargerOn, t + Duration::from_secs(120), "quiet", false), "10 min for the same event");
        assert!(g.allow(Event::ChargerOn, t + Duration::from_secs(700), "playful", false));
    }
}
