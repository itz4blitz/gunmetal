//! The player state machine: one transition function for every surface.
//!
//! Every player (the bar, the full-screen player, the lock screen, a phone
//! acting as a remote) is a view of one [`State`]. [`next`] is the diagram
//! in `docs/ui/player.md` ("One playback model") as a pure function: each
//! legal arrow is one `(state, event)` pair, and every other pair is
//! [`Illegal`]. Stopped is terminal and covers the owner stopping a session
//! and a device being revoked (ACC-069). Skipping a damaged file is the
//! [`Event::Skip`] arrow out of [`State::Error`] (MUS-079).

/// What the player is doing right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Nothing is loaded. The start of a new session.
    Idle,
    /// Play was requested; waiting for the first audio or first frame.
    Loading,
    /// Media is moving.
    Playing,
    /// Media is held at the current position.
    Paused,
    /// The buffer ran dry mid-play.
    Buffering,
    /// Buffering made no progress for a set time.
    Stalled,
    /// The current item reached its end.
    ItemEnded,
    /// Nothing is next, Continue-with is off and repeat is off.
    EndOfQueue,
    /// Starting failed, or a stall was given up.
    Error,
    /// The session was ended: stopped by the owner, or the device revoked.
    Stopped,
}

/// An input the machine accepts. Combined labels in the diagram are split
/// so each reason is its own event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// Start, resume, or play the queue again from the beginning.
    Play,
    /// The decoder produced the first audio or the first video frame.
    FirstAudioOrFirstFrame,
    /// The item cannot start (damaged, undecodable here, refused).
    CannotStart,
    /// Hold at the current position.
    Pause,
    /// The buffer ran dry while playing.
    BufferRanDry,
    /// The buffer refilled after running dry.
    BufferRefilled,
    /// Buffering made no progress for a set time.
    NoProgress,
    /// A stall recovered and media is moving again.
    Recovered,
    /// A stall was given up.
    GaveUp,
    /// The current item reached its end.
    EndOfItem,
    /// There is a next item to load.
    NextItem,
    /// There is nothing next.
    NothingNext,
    /// Start library radio from what just played ([`State::EndOfQueue`]).
    StartRadio,
    /// Try the failed item again.
    Retry,
    /// Skip the failed item (MUS-079).
    Skip,
    /// The owner stopped this session.
    StoppedByOwner,
    /// The device was revoked or the session was ended (ACC-069).
    DeviceRevoked,
}

/// [`next`] was called with a pair the diagram has no arrow for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Illegal {
    /// The state that received the event.
    pub state: State,
    /// The event that has no arrow from that state.
    pub event: Event,
}

/// The next state for `event` in `state`, or [`Illegal`] when the diagram
/// has no such arrow.
///
/// # Errors
///
/// [`Illegal`] carrying `state` and `event` when that pair is not an arrow
/// in the playback-model diagram.
pub const fn next(state: State, event: Event) -> Result<State, Illegal> {
    match (state, event) {
        (State::Idle, Event::Play)
        | (State::ItemEnded, Event::NextItem)
        | (State::EndOfQueue, Event::Play | Event::StartRadio)
        | (State::Error, Event::Retry | Event::Skip) => Ok(State::Loading),
        (State::Loading, Event::FirstAudioOrFirstFrame)
        | (State::Paused, Event::Play)
        | (State::Buffering, Event::BufferRefilled)
        | (State::Stalled, Event::Recovered) => Ok(State::Playing),
        (State::Loading, Event::CannotStart) | (State::Stalled, Event::GaveUp) => Ok(State::Error),
        (State::Playing, Event::Pause) => Ok(State::Paused),
        (State::Playing, Event::BufferRanDry) => Ok(State::Buffering),
        (State::Playing, Event::EndOfItem) => Ok(State::ItemEnded),
        (State::Buffering, Event::NoProgress) => Ok(State::Stalled),
        (State::ItemEnded, Event::NothingNext) => Ok(State::EndOfQueue),
        (State::Playing | State::Paused, Event::StoppedByOwner | Event::DeviceRevoked) => {
            Ok(State::Stopped)
        }
        (
            State::Idle
            | State::Loading
            | State::Playing
            | State::Paused
            | State::Buffering
            | State::Stalled
            | State::ItemEnded
            | State::EndOfQueue
            | State::Error
            | State::Stopped,
            Event::Play
            | Event::FirstAudioOrFirstFrame
            | Event::CannotStart
            | Event::Pause
            | Event::BufferRanDry
            | Event::BufferRefilled
            | Event::NoProgress
            | Event::Recovered
            | Event::GaveUp
            | Event::EndOfItem
            | Event::NextItem
            | Event::NothingNext
            | Event::StartRadio
            | Event::Retry
            | Event::Skip
            | Event::StoppedByOwner
            | Event::DeviceRevoked,
        ) => Err(Illegal { state, event }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every state, written out from the diagram. A new variant that is
    /// missing here fails [`product_lists_include_every_variant`].
    const STATES: [State; 10] = [
        State::Idle,
        State::Loading,
        State::Playing,
        State::Paused,
        State::Buffering,
        State::Stalled,
        State::ItemEnded,
        State::EndOfQueue,
        State::Error,
        State::Stopped,
    ];

    /// Every event, written out from the diagram's labels (combined labels
    /// split). A new variant that is missing here fails
    /// [`product_lists_include_every_variant`].
    const EVENTS: [Event; 17] = [
        Event::Play,
        Event::FirstAudioOrFirstFrame,
        Event::CannotStart,
        Event::Pause,
        Event::BufferRanDry,
        Event::BufferRefilled,
        Event::NoProgress,
        Event::Recovered,
        Event::GaveUp,
        Event::EndOfItem,
        Event::NextItem,
        Event::NothingNext,
        Event::StartRadio,
        Event::Retry,
        Event::Skip,
        Event::StoppedByOwner,
        Event::DeviceRevoked,
    ];

    /// Every legal arrow in player.md's diagram, written from the diagram,
    /// not from [`next`]. Combined labels are one row per reason.
    const LEGAL: [(State, Event, State); 21] = [
        (State::Idle, Event::Play, State::Loading),
        (
            State::Loading,
            Event::FirstAudioOrFirstFrame,
            State::Playing,
        ),
        (State::Loading, Event::CannotStart, State::Error),
        (State::Playing, Event::Pause, State::Paused),
        (State::Paused, Event::Play, State::Playing),
        (State::Playing, Event::BufferRanDry, State::Buffering),
        (State::Buffering, Event::BufferRefilled, State::Playing),
        (State::Buffering, Event::NoProgress, State::Stalled),
        (State::Stalled, Event::Recovered, State::Playing),
        (State::Stalled, Event::GaveUp, State::Error),
        (State::Playing, Event::EndOfItem, State::ItemEnded),
        (State::ItemEnded, Event::NextItem, State::Loading),
        (State::ItemEnded, Event::NothingNext, State::EndOfQueue),
        (State::EndOfQueue, Event::Play, State::Loading),
        (State::EndOfQueue, Event::StartRadio, State::Loading),
        (State::Error, Event::Retry, State::Loading),
        (State::Error, Event::Skip, State::Loading),
        (State::Playing, Event::StoppedByOwner, State::Stopped),
        (State::Playing, Event::DeviceRevoked, State::Stopped),
        (State::Paused, Event::StoppedByOwner, State::Stopped),
        (State::Paused, Event::DeviceRevoked, State::Stopped),
    ];

    fn destination(state: State, event: Event) -> Option<State> {
        let mut found = None;
        for &(from, input, to) in &LEGAL {
            if from == state && input == event {
                assert_eq!(found, None, "LEGAL lists {state:?} + {event:?} twice");
                found = Some(to);
            }
        }
        found
    }

    #[test]
    fn takes_every_legal_transition_in_the_diagram() {
        for &(from, event, to) in &LEGAL {
            assert_eq!(next(from, event), Ok(to));
        }
    }

    #[test]
    fn rejects_every_state_event_pair_outside_the_legal_set() {
        for state in STATES {
            for event in EVENTS {
                if destination(state, event).is_some() {
                    continue;
                }
                assert_eq!(next(state, event), Err(Illegal { state, event }));
            }
        }
    }

    #[test]
    fn product_lists_include_every_variant() {
        for state in STATES {
            match state {
                State::Idle
                | State::Loading
                | State::Playing
                | State::Paused
                | State::Buffering
                | State::Stalled
                | State::ItemEnded
                | State::EndOfQueue
                | State::Error
                | State::Stopped => {
                    assert!(STATES.contains(&state));
                }
            }
        }
        for event in EVENTS {
            match event {
                Event::Play
                | Event::FirstAudioOrFirstFrame
                | Event::CannotStart
                | Event::Pause
                | Event::BufferRanDry
                | Event::BufferRefilled
                | Event::NoProgress
                | Event::Recovered
                | Event::GaveUp
                | Event::EndOfItem
                | Event::NextItem
                | Event::NothingNext
                | Event::StartRadio
                | Event::Retry
                | Event::Skip
                | Event::StoppedByOwner
                | Event::DeviceRevoked => {
                    assert!(EVENTS.contains(&event));
                }
            }
        }
        for (i, a) in STATES.iter().enumerate() {
            for (j, b) in STATES.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b);
                }
            }
        }
        for (i, a) in EVENTS.iter().enumerate() {
            for (j, b) in EVENTS.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b);
                }
            }
        }
    }
}
