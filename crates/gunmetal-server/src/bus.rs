//! The in-process event bus: how one module tells the others that something
//! happened without depending on them.
//!
//! A [`Topic`] carries one type of event to its subscribers, in the order
//! they subscribed. The [`Bus`] holds one topic per kind of event; a package
//! that publishes a new kind adds one line to it.
//!
//! The security topic is the path to the audit log. Every producer of a
//! security event is given the bus as its [`SecuritySink`]; the audit log's
//! sink (WP-069) subscribes to [`Bus::security`] and its answer comes back
//! to the producer. An event nobody stored is an event that was not
//! recorded, so with no subscriber, or with one that refuses, the producer
//! is told the audit log is unavailable and its action does not take effect
//! (SEC-OPS-020).

use std::sync::{PoisonError, RwLock};

use gunmetal_core::audit_event::{AuditUnavailable, SecurityEvent, SecuritySink};

/// What a subscriber is: a function that takes each event and may refuse it.
type Subscriber<E, R> = Box<dyn Fn(&E) -> Result<(), R> + Send + Sync>;

/// One kind of event and the subscribers that receive it. `R` is how a
/// subscriber refuses an event.
pub struct Topic<E, R> {
    subscribers: RwLock<Vec<Subscriber<E, R>>>,
}

impl<E, R> Default for Topic<E, R> {
    fn default() -> Self {
        Self {
            subscribers: RwLock::new(Vec::new()),
        }
    }
}

impl<E, R> Topic<E, R> {
    /// Adds a subscriber, which receives every event published from now on.
    /// A subscriber must not subscribe to the topic it is called from.
    pub fn subscribe(&self, subscriber: impl Fn(&E) -> Result<(), R> + Send + Sync + 'static) {
        self.subscribers
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .push(Box::new(subscriber));
    }

    /// Hands `event` to each subscriber in turn and returns how many took
    /// it.
    ///
    /// # Errors
    ///
    /// The first refusal. Subscribers after the one that refused do not
    /// receive the event.
    pub fn publish(&self, event: &E) -> Result<usize, R> {
        let subscribers = self
            .subscribers
            .read()
            .unwrap_or_else(PoisonError::into_inner);
        subscribers
            .iter()
            .try_for_each(|subscriber| subscriber(event))
            .map(|()| subscribers.len())
    }
}

/// The server's bus: one topic per kind of event.
#[derive(Default)]
pub struct Bus {
    /// Security events, on their way to the audit log.
    pub security: Topic<SecurityEvent, AuditUnavailable>,
    // One line per kind of event, appended by later packages.
}

impl SecuritySink for Bus {
    /// Publishes `event` on the security topic. The answer is `Ok` only
    /// when at least one subscriber took the event and none refused it.
    fn record(&self, event: SecurityEvent) -> Result<(), AuditUnavailable> {
        match self.security.publish(&event)? {
            0 => Err(AuditUnavailable),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// What each subscriber saw, tagged with the subscriber's name.
    type Seen<E> = Arc<Mutex<Vec<(&'static str, E)>>>;

    fn recorder<E: Clone + Send + 'static, R: 'static>(
        seen: &Seen<E>,
        name: &'static str,
        answer: fn() -> Result<(), R>,
    ) -> impl Fn(&E) -> Result<(), R> + Send + Sync + 'static {
        let seen = Arc::clone(seen);
        move |event| {
            seen.lock().expect("seen").push((name, event.clone()));
            answer()
        }
    }

    fn seen<E: Clone>(seen: &Seen<E>) -> Vec<(&'static str, E)> {
        seen.lock().expect("seen").clone()
    }

    #[test]
    fn a_topic_without_subscribers_delivers_to_nobody() {
        let topic: Topic<u8, ()> = Topic::default();
        assert_eq!(topic.publish(&7), Ok(0));
    }

    #[test]
    fn delivers_each_event_to_every_subscriber_in_order() {
        let topic: Topic<u8, ()> = Topic::default();
        let log = Seen::default();
        topic.subscribe(recorder(&log, "first", || Ok(())));
        assert_eq!(topic.publish(&1), Ok(1));
        topic.subscribe(recorder(&log, "second", || Ok(())));
        assert_eq!(topic.publish(&2), Ok(2));
        assert_eq!(seen(&log), [("first", 1), ("first", 2), ("second", 2)]);
    }

    #[test]
    fn stops_at_the_first_subscriber_that_refuses() {
        let topic: Topic<u8, &'static str> = Topic::default();
        let log = Seen::default();
        topic.subscribe(recorder(&log, "first", || Ok(())));
        topic.subscribe(recorder(&log, "second", || Err("full")));
        topic.subscribe(recorder(&log, "third", || Ok(())));
        assert_eq!(topic.publish(&9), Err("full"));
        assert_eq!(seen(&log), [("first", 9), ("second", 9)]);
    }

    fn event() -> SecurityEvent {
        SecurityEvent::GmDebugLoggingEnabled { account: None }
    }

    #[test]
    fn a_security_event_nobody_stores_is_not_recorded() {
        assert_eq!(Bus::default().record(event()), Err(AuditUnavailable));
    }

    #[test]
    fn carries_a_security_event_to_the_audit_sink_and_its_answer_back() {
        let bus = Bus::default();
        let log = Seen::default();
        bus.security.subscribe(recorder(&log, "audit", || Ok(())));
        assert_eq!(bus.record(event()), Ok(()));
        assert_eq!(seen(&log), [("audit", event())]);
        bus.security
            .subscribe(recorder(&log, "broken", || Err(AuditUnavailable)));
        assert_eq!(bus.record(event()), Err(AuditUnavailable));
        assert_eq!(
            seen(&log),
            [("audit", event()), ("audit", event()), ("broken", event())]
        );
    }
}
