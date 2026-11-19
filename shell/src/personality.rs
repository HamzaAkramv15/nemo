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
}

const EVENTS: usize = 5;

impl Event {
    fn idx(self) -> usize {
        self as usize
    }
}

/// фраза про событие на языке интерфейса; None — на этом языке её нет
pub fn quip(ev: Event, lang: &str) -> Option<&'static str> {
    let (en, ar, ru) = match ev {
        Event::ChargerOn => ("Plugged in. The tide is coming in.", "تم التوصيل. المدّ قادم.", "Подключено. Прилив начался."), // i18n-ok
        Event::ChargerOff => ("Unplugged. Swimming on battery.", "فُصل الشاحن. نسبح على البطارية.", "Отключено. Плывём на батарее."), // i18n-ok
        Event::BatteryFull => ("Full tank. You can unplug.", "اكتمل الشحن. يمكنك فصله.", "Полный бак. Можно отключать."), // i18n-ok
        Event::CaffeineEnded => ("Your eyes could use a rest, too.", "عيناك أيضًا تحتاجان إلى راحة.", "И глазам пора отдохнуть."), // i18n-ok
        Event::UpdateReady => ("Fresh water is ready.", "الماء العذب جاهز.", "Свежая вода готова."), // i18n-ok
    };
    match lang {
        "en" => Some(en),
        "ar" => Some(ar),
        "ru" => Some(ru),
        _ => None,
    }
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
        if level == "off" || busy {
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
        let text = quip(ev, crate::i18n::code())?;
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

    const ALL: [Event; EVENTS] = [Event::ChargerOn, Event::ChargerOff, Event::BatteryFull, Event::CaffeineEnded, Event::UpdateReady];

    #[test]
    fn every_event_speaks_english_arabic_and_russian_and_no_other_language() {
        for e in ALL {
            for l in ["en", "ar", "ru"] {
                assert!(quip(e, l).is_some_and(|s| s.len() > 8), "{e:?} {l}");
            }
            for l in ["de", "fr", "es", "pt-BR", "uk", "pl", ""] {
                assert_eq!(quip(e, l), None, "{e:?} must stay silent in {l}, not fall back to English");
            }
        }
        // арабский — арабскими буквами, не копия английского
        assert!(quip(Event::ChargerOn, "ar").unwrap().chars().any(|c| ('\u{0600}'..='\u{06ff}').contains(&c)));
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
