//! The player state machine: the core's transition function, for the browser
//! (MUS-116 to MUS-119).
//!
//! [`player_next`] is one call of [`player::next`] with its types converted,
//! so every surface (the bar, the full player, the lock screen) shows the
//! same state the server would. The browser calls it as `playerNext`.

use gunmetal_core::player;
use serde::{Deserialize, Serialize};
use tsify::Tsify;

/// What the player is doing right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Tsify)]
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

/// An input the machine accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Tsify)]
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
    /// Start library radio from what just played.
    StartRadio,
    /// Try the failed item again.
    Retry,
    /// Skip the failed item.
    Skip,
    /// The owner stopped this session.
    StoppedByOwner,
    /// The device was revoked or the session was ended.
    DeviceRevoked,
}

/// `playerNext` was called with a pair the diagram has no arrow for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Tsify)]
pub struct Illegal {
    /// The state that received the event.
    pub state: State,
    /// The event that has no arrow from that state.
    pub event: Event,
}

/// The next state, or the pair that is not an arrow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Tsify)]
pub enum PlayerStep {
    /// The machine moved to this state.
    State(State),
    /// The pair is not an arrow in the diagram.
    Illegal(Illegal),
}

/// The TypeScript declarations of this module's mirror types, in the order
/// they are declared above.
pub const DECLARATIONS: [&str; 4] = [State::DECL, Event::DECL, Illegal::DECL, PlayerStep::DECL];

impl From<player::State> for State {
    fn from(value: player::State) -> Self {
        match value {
            player::State::Idle => Self::Idle,
            player::State::Loading => Self::Loading,
            player::State::Playing => Self::Playing,
            player::State::Paused => Self::Paused,
            player::State::Buffering => Self::Buffering,
            player::State::Stalled => Self::Stalled,
            player::State::ItemEnded => Self::ItemEnded,
            player::State::EndOfQueue => Self::EndOfQueue,
            player::State::Error => Self::Error,
            player::State::Stopped => Self::Stopped,
        }
    }
}

impl From<State> for player::State {
    fn from(value: State) -> Self {
        match value {
            State::Idle => Self::Idle,
            State::Loading => Self::Loading,
            State::Playing => Self::Playing,
            State::Paused => Self::Paused,
            State::Buffering => Self::Buffering,
            State::Stalled => Self::Stalled,
            State::ItemEnded => Self::ItemEnded,
            State::EndOfQueue => Self::EndOfQueue,
            State::Error => Self::Error,
            State::Stopped => Self::Stopped,
        }
    }
}

impl From<player::Event> for Event {
    fn from(value: player::Event) -> Self {
        match value {
            player::Event::Play => Self::Play,
            player::Event::FirstAudioOrFirstFrame => Self::FirstAudioOrFirstFrame,
            player::Event::CannotStart => Self::CannotStart,
            player::Event::Pause => Self::Pause,
            player::Event::BufferRanDry => Self::BufferRanDry,
            player::Event::BufferRefilled => Self::BufferRefilled,
            player::Event::NoProgress => Self::NoProgress,
            player::Event::Recovered => Self::Recovered,
            player::Event::GaveUp => Self::GaveUp,
            player::Event::EndOfItem => Self::EndOfItem,
            player::Event::NextItem => Self::NextItem,
            player::Event::NothingNext => Self::NothingNext,
            player::Event::StartRadio => Self::StartRadio,
            player::Event::Retry => Self::Retry,
            player::Event::Skip => Self::Skip,
            player::Event::StoppedByOwner => Self::StoppedByOwner,
            player::Event::DeviceRevoked => Self::DeviceRevoked,
        }
    }
}

impl From<Event> for player::Event {
    fn from(value: Event) -> Self {
        match value {
            Event::Play => Self::Play,
            Event::FirstAudioOrFirstFrame => Self::FirstAudioOrFirstFrame,
            Event::CannotStart => Self::CannotStart,
            Event::Pause => Self::Pause,
            Event::BufferRanDry => Self::BufferRanDry,
            Event::BufferRefilled => Self::BufferRefilled,
            Event::NoProgress => Self::NoProgress,
            Event::Recovered => Self::Recovered,
            Event::GaveUp => Self::GaveUp,
            Event::EndOfItem => Self::EndOfItem,
            Event::NextItem => Self::NextItem,
            Event::NothingNext => Self::NothingNext,
            Event::StartRadio => Self::StartRadio,
            Event::Retry => Self::Retry,
            Event::Skip => Self::Skip,
            Event::StoppedByOwner => Self::StoppedByOwner,
            Event::DeviceRevoked => Self::DeviceRevoked,
        }
    }
}

impl From<player::Illegal> for Illegal {
    fn from(value: player::Illegal) -> Self {
        let player::Illegal { state, event } = value;
        Self {
            state: state.into(),
            event: event.into(),
        }
    }
}

/// The next player state for `event` in `state`, from the core's machine.
#[must_use]
pub fn player_next(state: State, event: Event) -> PlayerStep {
    match player::next(state.into(), event.into()) {
        Ok(next) => PlayerStep::State(next.into()),
        Err(illegal) => PlayerStep::Illegal(illegal.into()),
    }
}

crate::export::export! {
    /// The browser's `playerNext`: [`player_next`], with its types converted.
    "playerNext": fn player_next_export = player_next(; state: State, event: Event) -> PlayerStep
}

#[cfg(test)]
mod tests {
    use gunmetal_core::player;

    use super::{DECLARATIONS, Event, Illegal, PlayerStep, State, player_next};

    /// Every core state beside its mirror.
    const STATES: [(player::State, State); 10] = [
        (player::State::Idle, State::Idle),
        (player::State::Loading, State::Loading),
        (player::State::Playing, State::Playing),
        (player::State::Paused, State::Paused),
        (player::State::Buffering, State::Buffering),
        (player::State::Stalled, State::Stalled),
        (player::State::ItemEnded, State::ItemEnded),
        (player::State::EndOfQueue, State::EndOfQueue),
        (player::State::Error, State::Error),
        (player::State::Stopped, State::Stopped),
    ];

    /// Every core event beside its mirror.
    const EVENTS: [(player::Event, Event); 17] = [
        (player::Event::Play, Event::Play),
        (
            player::Event::FirstAudioOrFirstFrame,
            Event::FirstAudioOrFirstFrame,
        ),
        (player::Event::CannotStart, Event::CannotStart),
        (player::Event::Pause, Event::Pause),
        (player::Event::BufferRanDry, Event::BufferRanDry),
        (player::Event::BufferRefilled, Event::BufferRefilled),
        (player::Event::NoProgress, Event::NoProgress),
        (player::Event::Recovered, Event::Recovered),
        (player::Event::GaveUp, Event::GaveUp),
        (player::Event::EndOfItem, Event::EndOfItem),
        (player::Event::NextItem, Event::NextItem),
        (player::Event::NothingNext, Event::NothingNext),
        (player::Event::StartRadio, Event::StartRadio),
        (player::Event::Retry, Event::Retry),
        (player::Event::Skip, Event::Skip),
        (player::Event::StoppedByOwner, Event::StoppedByOwner),
        (player::Event::DeviceRevoked, Event::DeviceRevoked),
    ];

    /// Every legal arrow in player.md's diagram, written from the diagram.
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

    #[test]
    fn each_state_converts_to_its_mirror_and_back() {
        for (core, mirrored) in STATES {
            assert_eq!(State::from(core), mirrored);
            assert_eq!(player::State::from(mirrored), core);
        }
    }

    #[test]
    fn each_event_converts_to_its_mirror_and_back() {
        for (core, mirrored) in EVENTS {
            assert_eq!(Event::from(core), mirrored);
            assert_eq!(player::Event::from(mirrored), core);
        }
    }

    #[test]
    fn every_legal_arrow_matches_the_core() {
        for (state, event, next) in LEGAL {
            assert_eq!(player_next(state, event), PlayerStep::State(next));
            assert_eq!(
                player::next(state.into(), event.into()),
                Ok(player::State::from(next))
            );
        }
    }

    /// A pair the diagram has no arrow for comes back as the pair itself.
    #[test]
    fn an_illegal_pair_is_the_state_and_event_that_were_given() {
        assert_eq!(
            player_next(State::Idle, Event::Pause),
            PlayerStep::Illegal(Illegal {
                state: State::Idle,
                event: Event::Pause,
            })
        );
        assert_eq!(
            player::next(player::State::Idle, player::Event::Pause),
            Err(player::Illegal {
                state: player::State::Idle,
                event: player::Event::Pause,
            })
        );
        assert_eq!(
            player_next(State::Stopped, Event::Play),
            PlayerStep::Illegal(Illegal {
                state: State::Stopped,
                event: Event::Play,
            })
        );
    }

    #[test]
    fn the_declarations_are_the_literal_typescript() {
        assert_eq!(DECLARATIONS.len(), 4);
        assert!(DECLARATIONS[0].contains("export type State"));
        assert!(DECLARATIONS[1].contains("export type Event"));
        assert!(DECLARATIONS[2].contains("export interface Illegal"));
        assert!(DECLARATIONS[3].contains("export type PlayerStep"));
    }
}
