//! How long a session lasts, on a clock the test moves.

use gunmetal_core::authz::DeviceClass;
use gunmetal_fs::sqlite::{Row, Value};

use super::http::{ME, ask, calls, granted, serve, unauthenticated};
use super::world::{SAM, World, pair};
use crate::session::lifetime::Lifetime;
use crate::testing::NOON;

/// A minute and a day, in milliseconds.
const MINUTE: i64 = 60_000;
const DAY: i64 = 86_400_000;

/// Verifies: SEC-IAM-041
#[test]
fn a_personal_session_ends_after_seven_days_without_use() {
    let world = World::fresh();
    let (cookie, _) = world.sign_in(SAM, DeviceClass::Personal, Lifetime::Personal);
    let cookie = pair(&cookie);
    let (client, _) = serve(&world);
    // Used a millisecond before the limit, twice over: each use starts the
    // seven days again.
    world.manual.advance(7 * DAY - 1);
    assert_eq!(ask(&client, &ME, &cookie), granted());
    world.manual.advance(7 * DAY - 1);
    assert_eq!(ask(&client, &ME, &cookie), granted());
    world.manual.advance(7 * DAY);
    assert_eq!(ask(&client, &ME, &cookie), unauthenticated());
    // It has ended for good: its row is gone, and a clock that steps back
    // does not bring it to life.
    assert_eq!(
        world.rows("SELECT count(*) FROM sessions"),
        [Row(vec![Value::Integer(0)])]
    );
    world.manual.advance(-7 * DAY);
    assert_eq!(ask(&client, &ME, &cookie), unauthenticated());
}

/// Verifies: SEC-IAM-041
#[test]
fn a_personal_session_ends_thirty_days_after_it_was_issued() {
    let world = World::fresh();
    let (cookie, _) = world.sign_in(SAM, DeviceClass::Personal, Lifetime::Personal);
    let cookie = pair(&cookie);
    let (client, _) = serve(&world);
    // Used every six days, so it is never idle for long.
    for _ in 0..4 {
        world.manual.advance(6 * DAY);
        assert_eq!(ask(&client, &ME, &cookie), granted());
    }
    world.manual.advance(6 * DAY - 1);
    assert_eq!(ask(&client, &ME, &cookie), granted());
    world.manual.advance(1);
    assert_eq!(ask(&client, &ME, &cookie), unauthenticated());
    assert_eq!(
        world.rows("SELECT count(*) FROM sessions"),
        [Row(vec![Value::Integer(0)])]
    );
}

/// Verifies: SEC-CLI-010, SEC-CLI-024
#[test]
fn a_shared_browser_session_ends_after_thirty_minutes_without_use() {
    let world = World::fresh();
    let (cookie, issued) = world.sign_in(SAM, DeviceClass::Limited, Lifetime::Shared);
    let cookie = pair(&cookie);
    let (client, seen) = serve(&world);
    world.manual.advance(30 * MINUTE - 1);
    assert_eq!(ask(&client, &ME, &cookie), granted());
    world.manual.advance(30 * MINUTE - 1);
    assert_eq!(ask(&client, &ME, &cookie), granted());
    world.manual.advance(30 * MINUTE);
    assert_eq!(ask(&client, &ME, &cookie), unauthenticated());
    // Both requests that got through were the limited-class device's.
    assert_eq!(issued.facts.device, DeviceClass::Limited);
    assert_eq!(
        calls(&seen),
        [(ME.path, Some(issued.clone())), (ME.path, Some(issued))]
    );
}

#[test]
fn a_session_records_its_use_at_most_once_a_minute() {
    let world = World::fresh();
    let (cookie, _) = world.sign_in(SAM, DeviceClass::Personal, Lifetime::Personal);
    let cookie = pair(&cookie);
    let (client, _) = serve(&world);
    let times = "SELECT created, last_seen FROM sessions";
    world.manual.advance(MINUTE - 1);
    assert_eq!(ask(&client, &ME, &cookie), granted());
    assert_eq!(
        world.rows(times),
        [Row(vec![Value::Integer(NOON), Value::Integer(NOON)])]
    );
    world.manual.advance(1);
    assert_eq!(ask(&client, &ME, &cookie), granted());
    assert_eq!(
        world.rows(times),
        [Row(vec![
            Value::Integer(NOON),
            Value::Integer(NOON + MINUTE)
        ])]
    );
}
