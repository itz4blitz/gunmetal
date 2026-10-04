//! The harness for the `INFO` list mapper in
//! [`gunmetal_core::tags::riff`].

use gunmetal_core::catalog::{Credit, Role, TrackPosition, TrackTags};
use gunmetal_core::parse::{Budget, LimitKind, Limits, ParseFault};
use gunmetal_core::tags::mp4::{TagField, TagProblem};
use gunmetal_core::tags::riff::{self, InfoChunk, InfoTags};

/// Where in its file the harness says the list starts.
pub const BEFORE: u64 = 1_000;

/// What the mapper reported for one input.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    /// [`riff::from_info`] over the input as the sub-chunks of a list that
    /// starts [`BEFORE`] octets into its file.
    pub tags: InfoTags,
    /// The steps of budget the reading spent.
    pub steps: u64,
}

/// Feeds `data` to [`riff::from_info`] as the sub-chunks of an `INFO` list
/// that starts [`BEFORE`] octets into its file, under the default limits,
/// with the budget the module documents as always enough:
/// [`riff::STEPS_PER_OCTET`] steps per octet plus [`riff::STEPS_FIXED`].
///
/// # Panics
///
/// Panics when the mapper breaks an invariant that holds for every input:
/// the documented budget spent, or more steps than the n / 8 + 1 it
/// documents for n octets; a walk stopped at an offset outside the list;
/// a source or problem whose ID is not the four octets at its offset in
/// the list, whose header does not fit in the list, or that comes before
/// the one listed ahead of it; a problem that is not about one value; a
/// field filled without a source, or a source for a field left empty; a
/// field that no `INFO` sub-chunk maps to filled; a credit that is not a
/// composer's; a list longer than the tag-field limit; or text that is
/// blank, longer than the short-text limit or holds a control character.
#[must_use]
pub fn run(data: &[u8]) -> Outcome {
    let len = u64::try_from(data.len()).unwrap_or(u64::MAX);
    let mut budget = Budget::for_input(len, riff::STEPS_PER_OCTET, riff::STEPS_FIXED);
    let documented = budget.remaining();
    let tags = riff::from_info(data, BEFORE, &Limits::DEFAULT, &mut budget);
    let steps = documented.saturating_sub(budget.remaining());
    assert!(
        steps.saturating_sub(1).saturating_mul(8) <= len
            && tags.stopped.as_ref().is_none_or(|fault| {
                !matches!(fault, ParseFault::BudgetExceeded { .. })
                    && (BEFORE..=BEFORE.saturating_add(len)).contains(&fault.offset())
            }),
        "{steps} steps for {len} octets gave {tags:?}"
    );
    let mapped = &tags.mapped;
    // Whether the eight-octet header of `chunk` lies inside the list and
    // starts with its ID.
    let read_at = |chunk: &InfoChunk| {
        chunk
            .offset
            .checked_sub(BEFORE)
            .and_then(|start| usize::try_from(start).ok())
            .and_then(|start| data.get(start..))
            .and_then(|rest| rest.get(..8))
            .is_some_and(|header| header.starts_with(&chunk.id))
    };
    let chunks = mapped
        .sources
        .iter()
        .map(|(_, chunk)| *chunk)
        .collect::<Vec<_>>();
    let mut previous_problem_offset = BEFORE;
    assert!(
        chunks.iter().all(read_at)
            && chunks.iter().map(|chunk| chunk.offset).is_sorted()
            && mapped.problems.iter().all(
                |problem| matches!(problem, TagProblem::Value { source, .. } if {
                    let in_order = source.offset >= previous_problem_offset;
                    previous_problem_offset = source.offset;
                    read_at(source) && in_order
                })
            ),
        "{mapped:?} from {data:?}"
    );
    let t = &mapped.tags;
    let sourced = |field: TagField| mapped.sources.iter().any(|(by, _)| *by == field);
    let only_info = TrackTags {
        title: t.title.clone(),
        artist: t.artist.clone(),
        album: t.album.clone(),
        position: TrackPosition::new(t.position.track(), t.position.track_total(), None, None)
            .unwrap_or_default(),
        date: t.date,
        genres: t.genres.clone(),
        credits: t.credits.clone(),
        ..TrackTags::default()
    };
    let fields = usize::try_from(Limits::DEFAULT.get(LimitKind::TagFields)).unwrap_or(usize::MAX);
    assert!(
        *t == only_info
            && t.title.is_some() == sourced(TagField::Title)
            && t.artist.is_empty() != sourced(TagField::Artist)
            && t.album.is_some() == sourced(TagField::Album)
            && t.date.is_some() == sourced(TagField::Date)
            && t.genres.is_empty() != sourced(TagField::Genres)
            && t.position.track().is_some() == sourced(TagField::Track)
            && t.position.track_total().is_some() == sourced(TagField::TrackTotal)
            && t.credits.is_empty() != sourced(TagField::Credit(Role::Composer))
            && t.credits
                .iter()
                .all(|credit| credit.role() == Role::Composer)
            && [t.artist.len(), t.genres.len(), t.credits.len()]
                .iter()
                .all(|count| *count <= fields),
        "{mapped:?}"
    );
    let short = usize::try_from(Limits::DEFAULT.get(LimitKind::ShortText)).unwrap_or(usize::MAX);
    assert!(
        t.title
            .iter()
            .chain(&t.artist)
            .chain(&t.album)
            .chain(&t.genres)
            .map(String::as_str)
            .chain(t.credits.iter().map(Credit::name))
            .all(|text| {
                text.len() <= short
                    && !text.trim().is_empty()
                    && !text.chars().any(char::is_control)
            }),
        "{t:?}"
    );
    Outcome { tags, steps }
}
