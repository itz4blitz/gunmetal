//! The request pipeline over a stand-in route table, driven through the
//! in-process test client. Every expected response is written out whole:
//! status, every header and the body.

mod support;

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{HeaderValue, Request};
use gunmetal_http::call::{Call, Reply};
use gunmetal_http::client::{TestClient, TestResponse};
use gunmetal_http::pipeline::{AccessRecord, router};
use gunmetal_http::problem::RequestId;
use gunmetal_http::request::{Fields, NoQuery};
use gunmetal_http::route::{Access, BodyRule, Effect, JsonLimits, Method, RateClass};
use gunmetal_http::table::{RouteEntry, Table, TableError};
use proptest::prelude::*;
use support::{
    AUDIT, HOST, INTERNAL, JSON, NOT_ALLOWED, ORIGIN, PUBLIC, Peer, Seen, Sent, TOO_LARGE,
    UNKNOWN_HOST, UNSUPPORTED, bearer, client, client_with, cross_site, empty, entries, get,
    headers, hooks, invalid, json, misplaced, not_found, problem, record, request, spec,
};

fn send_all(client: &TestClient, requests: Vec<Request<Body>>) -> Vec<TestResponse> {
    requests.into_iter().map(|r| client.send(r)).collect()
}

fn credentials(seen: &Seen) -> Vec<(&'static str, Option<Vec<u8>>)> {
    seen.accessed()
        .into_iter()
        .map(|(path, sent, _)| {
            let bytes = match sent {
                Sent::None => None,
                Sent::Cookie(bytes) | Sent::Header(bytes) => Some(bytes),
            };
            (path, bytes)
        })
        .collect()
}

#[test]
fn answers_routes_and_passes_the_grant_to_the_rate_hook_and_the_handler() {
    let (client, seen) = client();
    assert_eq!(
        send_all(
            &client,
            vec![
                get("/api/v1/server"),
                bearer("GET", "/api/v1/playlists/pls_7", &[], ""),
                bearer("DELETE", "/api/v1/playlists/pls_7", &[], ""),
                bearer("GET", "/api/v1/me", &[], ""),
                bearer("POST", "/api/v1/notes", &[JSON], "{}"),
            ]
        ),
        [
            json(200, r#"{"name":"gm"}"#),
            json(200, r#"{"id":"pls_7","name":"Mix"}"#),
            empty(),
            json(200, r#"{"principal":"Bearer alice"}"#),
            empty(),
        ]
    );
    let alice = Some(b"Bearer alice".to_vec());
    assert_eq!(
        credentials(&seen),
        [
            ("/api/v1/server", None),
            ("/api/v1/playlists/{id}", alice.clone()),
            ("/api/v1/playlists/{id}", alice.clone()),
            ("/api/v1/me", alice.clone()),
            ("/api/v1/notes", alice),
        ]
    );
    let alice = Some("Bearer alice".to_owned());
    assert_eq!(
        seen.rated(),
        [
            (RateClass::Read, None),
            (RateClass::Read, alice.clone()),
            (RateClass::Read, alice.clone()),
            (RateClass::Read, alice.clone()),
            (RateClass::Write, alice),
        ]
    );
    assert_eq!(seen.events(), ["begin", "commit"]);
    assert_eq!(seen.handled(), ["me"]);
}

/// Verifies: SEC-API-095
#[test]
fn logs_the_route_template_and_never_the_path_or_a_credential() {
    let (client, seen) = client();
    let mut with_peer = bearer(
        "GET",
        "/api/v1/playlists/pls_SECRET?cursor=tok_SECRET",
        &[],
        "",
    );
    with_peer.extensions_mut().insert(Peer(9));
    let mut refused = request(
        "GET",
        "/api/v1/playlists/pls_SECRET",
        &[("host", HOST), ("authorization", "Bearer refused")],
        "",
    );
    refused.extensions_mut().insert(Peer(4));
    let mut unknown = get("/api/v1/nowhere/pls_SECRET");
    unknown.extensions_mut().insert(Peer(5));
    assert_eq!(
        send_all(
            &client,
            vec![with_peer, refused, unknown, get("/api/v1/server")]
        ),
        [
            // The playlist route takes no query, so this one is refused;
            // its record still names only the template.
            invalid(0),
            not_found(1),
            not_found(2),
            json(200, r#"{"name":"gm"}"#),
        ]
    );
    let logged = seen.logged();
    assert_eq!(
        logged,
        [
            (
                record(0, Some(Method::Get), Some("/api/v1/playlists/{id}"), 400),
                Some("Bearer alice".to_owned()),
                Some(Peer(9)),
            ),
            (
                record(1, Some(Method::Get), Some("/api/v1/playlists/{id}"), 404),
                None,
                Some(Peer(4)),
            ),
            (record(2, Some(Method::Get), None, 404), None, Some(Peer(5))),
            (
                record(3, Some(Method::Get), Some("/api/v1/server"), 200),
                None,
                None,
            ),
        ]
    );
    let records = format!("{:?}", seen.records());
    assert!(!records.contains("SECRET"));
    assert!(!records.contains("Bearer"));
    assert_eq!(
        seen.accessed().first().map(|(_, _, peer)| *peer),
        Some(Some(Peer(9)))
    );
}

/// Verifies: SEC-API-001, SEC-API-055
#[test]
fn answers_only_the_routes_in_the_table_and_matches_them_exactly() {
    let (client, seen) = client();
    let targets = [
        "/api/v1/server/",
        "/api/v1/server.json",
        "/api/v1/server;x=1",
        "/api/v1/Server",
        "/api/v1/%73erver",
        "/api/v1/server/extra",
        "/API/v1/server",
        "/api/v1//server",
        "/api/v2/server",
        "/api/v1/playlists/a%2Fb",
        "/api/v1/playlists/a.b",
        "/api/v1/playlists",
        "/server",
        "/",
        "/healthz",
    ];
    let responses: Vec<TestResponse> = targets
        .iter()
        .map(|target| client.send(get(target)))
        .collect();
    let expected: Vec<TestResponse> = (0..15).map(not_found).collect();
    assert_eq!(responses, expected);
    assert_eq!(seen.accessed(), []);
    assert_eq!(
        seen.records().first(),
        Some(&record(0, Some(Method::Get), None, 404))
    );
}

/// Verifies: SEC-API-001, SEC-API-092
#[test]
fn builds_a_router_only_from_a_well_formed_table() {
    let seen = Arc::new(Seen::default());
    let build = |method, path, effect| {
        let entry = RouteEntry::new(
            spec(method, path, Access::Public { effect }, BodyRule::None),
            |_: Call| async { Ok(Reply::empty()) },
        );
        router(&[entry.clone(), entry], hooks(&seen, false)).map(|_| ())
    };
    assert_eq!(
        [
            build(Method::Get, "/healthz", Effect::Reads),
            build(Method::Get, "/api/v2/server", Effect::Reads),
            build(Method::Get, "/api/v1/server/", Effect::Reads),
            build(Method::Get, "/api/v1/server", Effect::Mutates),
            build(Method::Get, "/api/v1/server", Effect::Reads),
        ],
        [
            Err(TableError::Unversioned("/healthz")),
            Err(TableError::Unversioned("/api/v2/server")),
            Err(TableError::Template("/api/v1/server/")),
            Err(TableError::ReadMutates("/api/v1/server")),
            Err(TableError::Duplicate(Method::Get, "/api/v1/server")),
        ]
    );
    assert_eq!(router(&[], hooks(&seen, false)).map(|_| ()), Ok(()));
}

/// A body type that names the same `id` its route's path binds.
#[derive(serde::Deserialize)]
struct Thing {}

impl Fields for Thing {
    const FIELDS: &'static [&'static str] = &["id", "name"];
}

/// Verifies: SEC-API-067
#[test]
fn refuses_at_start_up_a_route_that_takes_one_name_from_the_path_and_the_body() {
    let seen = Arc::new(Seen::default());
    let path = "/api/v1/things/{id}";
    let entry = RouteEntry::new(
        spec(
            Method::Put,
            path,
            Access::Public {
                effect: Effect::Mutates,
            },
            BodyRule::Json(JsonLimits::DEFAULT),
        ),
        |_: Call<NoQuery, Thing>| async { Ok(Reply::empty()) },
    );
    assert_eq!(
        router(&[entry], hooks(&seen, false)).map(|_| ()),
        Err(TableError::Ambiguous(path, "id"))
    );
}

/// Verifies: SEC-HIS-005, SEC-IAM-067
#[test]
fn lists_the_routes_that_need_no_session() {
    let seen = Arc::new(Seen::default());
    let allow_list = [
        (Method::Get, "/api/v1/server"),
        (Method::Get, "/api/v1/test/panic"),
        (Method::Get, "/api/v1/test/panic-text"),
        (Method::Get, "/api/v1/test/panic-other"),
        (Method::Get, "/api/v1/test/missing"),
        (Method::Get, "/api/v1/test/slow"),
    ];
    assert_eq!(Table::new(entries(&seen)).unwrap().anonymous(), allow_list);
    // A public route nobody put on the list is caught.
    let mut more = entries(&seen);
    more.push(RouteEntry::new(
        spec(Method::Get, "/api/v1/leak", PUBLIC, BodyRule::None),
        |_: Call| async { Ok(Reply::empty()) },
    ));
    let mut longer = allow_list.to_vec();
    longer.push((Method::Get, "/api/v1/leak"));
    assert_eq!(Table::new(more).unwrap().anonymous(), longer);
}

/// Verifies: SEC-API-007, SEC-IAM-010, SEC-NET-014, SEC-TM-009
#[test]
fn refuses_hosts_off_the_allow_list_with_421() {
    let (client, seen) = client();
    let mut unreadable = request("GET", "/api/v1/server", &[], "");
    unreadable.headers_mut().insert(
        "host",
        HeaderValue::from_bytes(b"music.example.com\xff").unwrap(),
    );
    let requests = vec![
        request("GET", "/api/v1/server", &[("host", "attacker.example")], ""),
        request(
            "GET",
            "/api/v1/server",
            &[("host", "music.example.com.attacker.example")],
            "",
        ),
        request("GET", "/api/v1/server", &[], ""),
        request(
            "GET",
            "/api/v1/server",
            &[("host", HOST), ("host", HOST)],
            "",
        ),
        request(
            "GET",
            "http://attacker.example/api/v1/server",
            &[("host", HOST)],
            "",
        ),
        request(
            "GET",
            "http://music.example.com/api/v1/server",
            &[("host", "attacker.example")],
            "",
        ),
        request("GET", "http://attacker.example/api/v1/server", &[], ""),
        request(
            "GET",
            "http://attacker.example/api/v1/server",
            &[("host", "attacker.example")],
            "",
        ),
        request(
            "GET",
            "http://localhost/api/v1/server",
            &[("host", HOST)],
            "",
        ),
        request("GET", "/api/v1/server", &[("host", "bad host")], ""),
        request("GET", "/api/v1/server", &[("host", "127.0.0.1")], ""),
        unreadable,
    ];
    let expected: Vec<TestResponse> = (0..12)
        .map(|n| problem("unknown_host", 421, UNKNOWN_HOST, n, &[]))
        .collect();
    assert_eq!(send_all(&client, requests), expected);
    assert_eq!(seen.accessed(), []);
    assert_eq!(
        seen.records().first(),
        Some(&record(0, Some(Method::Get), None, 421))
    );
    let allowed = vec![
        request(
            "GET",
            "/api/v1/server",
            &[("host", "MUSIC.example.com.:443")],
            "",
        ),
        request("GET", "/api/v1/server", &[("host", "localhost:4533")], ""),
        request(
            "GET",
            "/api/v1/server",
            &[("host", "192.168.1.20:4533")],
            "",
        ),
        request("GET", "http://music.example.com/api/v1/server", &[], ""),
        request(
            "GET",
            "http://music.example.com:8080/api/v1/server",
            &[("host", "Music.Example.Com")],
            "",
        ),
    ];
    assert_eq!(
        send_all(&client, allowed),
        vec![json(200, r#"{"name":"gm"}"#); 5]
    );
}

/// Verifies: SEC-API-008
#[test]
fn answers_other_methods_with_405_and_ignores_overrides() {
    let (client, seen) = client();
    let allow = [("allow", "GET, DELETE, PATCH")];
    let responses: Vec<TestResponse> = [
        "POST", "PUT", "TRACE", "CONNECT", "HEAD", "OPTIONS", "get", "PROPFIND",
    ]
    .into_iter()
    .map(|method| {
        client.send(request(
            method,
            "/api/v1/playlists/pls_7",
            &[("host", HOST)],
            "",
        ))
    })
    .collect();
    // axum answers HEAD without the body it was given, as HTTP requires.
    let mut expected: Vec<TestResponse> = (0..8)
        .map(|n| problem("method_not_allowed", 405, NOT_ALLOWED, n, &allow))
        .collect();
    expected[4].body = Vec::new();
    assert_eq!(responses, expected);
    for (name, n) in [
        ("x-http-method-override", 8),
        ("x-http-method", 9),
        ("x-method-override", 10),
    ] {
        assert_eq!(
            client.send(request(
                "GET",
                "/api/v1/playlists/pls_7",
                &[("host", HOST), (name, "DELETE")],
                ""
            )),
            json(200, r#"{"id":"pls_7","name":"Mix"}"#)
        );
        assert_eq!(
            client.send(request(
                "POST",
                "/api/v1/server",
                &[("host", HOST), (name, "GET")],
                ""
            )),
            problem(
                "method_not_allowed",
                405,
                NOT_ALLOWED,
                2 * n - 7,
                &[("allow", "GET")]
            )
        );
    }
    assert_eq!(seen.events(), [] as [&str; 0]);
    assert_eq!(
        seen.records().get(..3),
        Some(
            [
                record(0, Some(Method::Post), Some("/api/v1/playlists/{id}"), 405),
                record(1, Some(Method::Put), Some("/api/v1/playlists/{id}"), 405),
                record(2, None, Some("/api/v1/playlists/{id}"), 405),
            ]
            .as_slice()
        )
    );
}

/// Verifies: SEC-API-004, SEC-EXT-006, SEC-HIS-041
#[test]
fn refuses_credentials_outside_the_cookie_and_the_authorization_header() {
    let (client, seen) = client();
    let requests = vec![
        get("/api/v1/me?token=abc"),
        get("/api/v1/me?access_token=abc"),
        get("/api/v1/me?api_key=abc"),
        get("/api/v1/me?apiKey=abc"),
        get("/api/v1/me?APIKEY=abc"),
        get("/api/v1/me?password=abc"),
        get("/api/v1/me?refresh_token=abc"),
        get("/api/v1/me?limit=5&%74oken=abc"),
        get("/api/v1/me;token=abc"),
        get("/api/v1/token=abc/me"),
        get("/api/v1/nowhere?token=abc"),
        request(
            "GET",
            "/api/v1/me",
            &[("host", HOST), ("x-plex-token", "abc")],
            "",
        ),
        request(
            "GET",
            "/api/v1/me",
            &[("host", HOST), ("x-api-key", "abc")],
            "",
        ),
        request(
            "GET",
            "/api/v1/me",
            &[
                ("host", HOST),
                ("authorization", "Bearer a"),
                ("authorization", "Bearer b"),
            ],
            "",
        ),
        request(
            "GET",
            "/api/v1/me",
            &[
                ("host", HOST),
                ("authorization", "Bearer a"),
                ("cookie", "__Host-gm_session=b"),
                ("gunmetal-request", "1"),
            ],
            "",
        ),
        request(
            "GET",
            "/api/v1/me",
            &[
                ("host", HOST),
                ("cookie", "__Host-gm_session=a; __Host-gm_session=b"),
                ("gunmetal-request", "1"),
            ],
            "",
        ),
        bearer("GET", "/api/v1/me?token=abc", &[], ""),
    ];
    let expected: Vec<TestResponse> = (0..17).map(misplaced).collect();
    assert_eq!(send_all(&client, requests), expected);
    assert_eq!(seen.accessed(), []);
    assert_eq!(seen.handled(), [] as [&str; 0]);
    assert_eq!(
        client.send(bearer("GET", "/api/v1/me", &[], "")),
        json(200, r#"{"principal":"Bearer alice"}"#)
    );
}

/// Verifies: SEC-API-033
#[test]
fn needs_the_request_header_on_every_cookie_request_before_authentication() {
    let (client, seen) = client();
    let cookie = ("cookie", "__Host-gm_session=s3cr3t");
    let site = ("sec-fetch-site", "same-origin");
    let requests = vec![
        request("GET", "/api/v1/me", &[("host", HOST), cookie, site], ""),
        request(
            "DELETE",
            "/api/v1/playlists/p",
            &[("host", HOST), cookie, site],
            "",
        ),
        request(
            "PATCH",
            "/api/v1/playlists/p",
            &[("host", HOST), cookie, site, ("origin", ORIGIN), JSON],
            r#"{"name":"x"}"#,
        ),
        request(
            "POST",
            "/api/v1/notes",
            &[("host", HOST), cookie, site, JSON],
            "{}",
        ),
        request(
            "GET",
            "/api/v1/me",
            &[("host", HOST), cookie, ("gunmetal-request", "true")],
            "",
        ),
    ];
    let expected: Vec<TestResponse> = (0..5).map(cross_site).collect();
    assert_eq!(send_all(&client, requests), expected);
    assert_eq!(seen.accessed(), []);
    let marker = ("gunmetal-request", "1");
    assert_eq!(
        client.send(request(
            "GET",
            "/api/v1/me",
            &[("host", HOST), cookie, marker],
            ""
        )),
        json(200, r#"{"principal":"s3cr3t"}"#)
    );
    assert_eq!(
        client.send(request(
            "DELETE",
            "/api/v1/playlists/p",
            &[("host", HOST), cookie, marker, site],
            ""
        )),
        empty()
    );
    assert_eq!(
        credentials(&seen),
        [
            ("/api/v1/me", Some(b"s3cr3t".to_vec())),
            ("/api/v1/playlists/{id}", Some(b"s3cr3t".to_vec())),
        ]
    );
}

/// Verifies: SEC-API-034, SEC-CLI-007, SEC-IAM-040
#[test]
fn refuses_cookie_requests_from_other_sites() {
    let (client, seen) = client();
    let cookie = ("cookie", "__Host-gm_session=s3cr3t");
    let marker = ("gunmetal-request", "1");
    let delete = |extra: &[(&str, &str)]| {
        let mut all = vec![("host", HOST), cookie, marker];
        all.extend_from_slice(extra);
        request("DELETE", "/api/v1/playlists/p", &all, "")
    };
    let requests = vec![
        request(
            "GET",
            "/api/v1/me",
            &[
                ("host", HOST),
                cookie,
                marker,
                ("sec-fetch-site", "cross-site"),
            ],
            "",
        ),
        request(
            "GET",
            "/api/v1/me",
            &[
                ("host", HOST),
                cookie,
                marker,
                ("sec-fetch-site", "same-site"),
            ],
            "",
        ),
        delete(&[("origin", "https://evil.example")]),
        delete(&[("origin", "https://sub.music.example.com")]),
        delete(&[("origin", "http://music.example.com")]),
        delete(&[("origin", "https://music.example.com:8443")]),
        delete(&[("origin", "null")]),
        delete(&[("origin", ORIGIN), ("sec-fetch-site", "cross-site")]),
        delete(&[("origin", ORIGIN), ("sec-fetch-site", "same-site")]),
        delete(&[("origin", ORIGIN), ("sec-fetch-site", "none")]),
        delete(&[
            ("sec-fetch-site", "same-origin"),
            ("origin", "https://evil.example"),
        ]),
        delete(&[]),
    ];
    let expected: Vec<TestResponse> = (0..12).map(cross_site).collect();
    assert_eq!(send_all(&client, requests), expected);
    assert_eq!(seen.accessed(), []);
    assert_eq!(seen.events(), [] as [&str; 0]);
    assert_eq!(
        send_all(
            &client,
            vec![
                delete(&[("origin", ORIGIN)]),
                delete(&[("sec-fetch-site", "same-origin")]),
                delete(&[("origin", ORIGIN), ("sec-fetch-site", "same-origin")]),
                // A bearer credential is not sent by a browser on its own,
                // so these checks do not apply to it.
                bearer(
                    "DELETE",
                    "/api/v1/playlists/p",
                    &[
                        ("sec-fetch-site", "cross-site"),
                        ("origin", "https://evil.example")
                    ],
                    ""
                ),
            ]
        ),
        vec![empty(); 4]
    );
}

/// Verifies: SEC-API-035, SEC-API-065
#[test]
fn takes_only_uncompressed_json_bodies() {
    let (client, _) = client();
    let target = "/api/v1/playlists/p";
    let body = r#"{"name":"x"}"#;
    let form = "application/x-www-form-urlencoded";
    let requests = vec![
        bearer("PATCH", target, &[("content-type", "text/plain")], body),
        bearer("PATCH", target, &[("content-type", form)], "name=x"),
        bearer(
            "PATCH",
            target,
            &[("content-type", "multipart/form-data; boundary=x")],
            body,
        ),
        bearer(
            "PATCH",
            target,
            &[("content-type", "application/jsonx")],
            body,
        ),
        bearer(
            "PATCH",
            target,
            &[("content-type", "text/plain; application/json")],
            body,
        ),
        bearer("PATCH", target, &[], body),
        bearer("PATCH", target, &[JSON, JSON], body),
        bearer("PATCH", target, &[JSON, ("content-encoding", "gzip")], body),
        bearer(
            "PATCH",
            target,
            &[JSON, ("content-encoding", "deflate")],
            body,
        ),
        bearer("PATCH", target, &[JSON, ("content-encoding", "br")], body),
        bearer(
            "PATCH",
            target,
            &[JSON, ("content-encoding", "identity")],
            body,
        ),
        bearer("DELETE", target, &[JSON], ""),
        bearer("DELETE", target, &[("content-encoding", "gzip")], ""),
    ];
    let expected: Vec<TestResponse> = (0..13)
        .map(|n| problem("unsupported_body", 415, UNSUPPORTED, n, &[]))
        .collect();
    assert_eq!(send_all(&client, requests), expected);
    assert_eq!(
        send_all(
            &client,
            vec![
                bearer(
                    "PATCH",
                    target,
                    &[("content-type", "Application/JSON")],
                    body
                ),
                bearer(
                    "PATCH",
                    target,
                    &[("content-type", "application/json; charset=utf-8")],
                    body
                ),
                bearer(
                    "PATCH",
                    target,
                    &[("content-type", " application/json ;charset=utf-8")],
                    body
                ),
            ]
        ),
        vec![json(200, r#"{"id":"p","name":"x"}"#); 3]
    );
}

/// Verifies: SEC-API-060
#[test]
fn refuses_bodies_over_the_cap_before_decoding() {
    let (client, _) = client();
    let at_cap = format!(r#"{{"text":"{}"}}"#, "a".repeat(21));
    let over_cap = format!(r#"{{"text":"{}"}}"#, "a".repeat(22));
    assert_eq!((at_cap.len(), over_cap.len()), (32, 33));
    let too_large = |n| problem("body_too_large", 413, TOO_LARGE, n, &[]);
    assert_eq!(
        send_all(
            &client,
            vec![
                bearer("POST", "/api/v1/notes", &[JSON], &at_cap),
                bearer("POST", "/api/v1/notes", &[JSON], &over_cap),
                // Not JSON at all, but over the cap: refused for its size,
                // unread.
                bearer("POST", "/api/v1/notes", &[JSON], &"{".repeat(33)),
                bearer("DELETE", "/api/v1/playlists/p", &[], "x"),
                bearer("POST", "/api/v1/notes", &[JSON], r#"{"tags":[1,2,3,4,5]}"#),
                bearer("POST", "/api/v1/notes", &[JSON], r#"{"tags":[1,2,3,4]}"#),
                bearer("POST", "/api/v1/notes", &[JSON], &"[".repeat(32)),
                bearer("POST", "/api/v1/notes", &[JSON], ""),
            ]
        ),
        [
            empty(),
            too_large(1),
            too_large(2),
            too_large(3),
            invalid(4),
            empty(),
            invalid(6),
            invalid(7),
        ]
    );
}

/// Verifies: SEC-API-067, SEC-IAM-072
#[test]
fn refuses_extra_duplicate_and_repeated_fields() {
    let (client, seen) = client();
    let patch_at = |target: &str, body: &str| bearer("PATCH", target, &[JSON], body);
    let patch = |body: &str| patch_at("/api/v1/playlists/p", body);
    let requests = vec![
        patch(r#"{"name":"x","public":true}"#),
        patch(r#"{"name":"x","role":"admin"}"#),
        patch(r#"{"name":"x","name":"y"}"#),
        patch(r#"{"name":"x","tags":{"a":1,"a":2}}"#),
        patch(r#"{"name":5}"#),
        patch("{"),
        bearer("GET", "/api/v1/tracks?limit=5&limit=6", &[], ""),
        bearer("GET", "/api/v1/tracks?limit=5&limit=5", &[], ""),
        bearer("GET", "/api/v1/tracks?limit=5&callback=f", &[], ""),
        bearer("GET", "/api/v1/tracks?limit=0", &[], ""),
        bearer("GET", "/api/v1/tracks?limit=501", &[], ""),
        bearer(
            "PATCH",
            "/api/v1/playlists/p?name=y",
            &[JSON],
            r#"{"name":"x"}"#,
        ),
        bearer("GET", "/api/v1/playlists/p?id=q", &[], ""),
        bearer("GET", "/api/v1/tracks?limit=%zz", &[], ""),
        bearer("GET", "/api/v1/nowhere?limit=%zz", &[], ""),
        // Routes that declare no query type take no query parameter, though
        // their handlers never look at the query.
        bearer("GET", "/api/v1/me?callback=f", &[], ""),
        bearer("GET", "/api/v1/playlists/p?limit=5", &[], ""),
        bearer("DELETE", "/api/v1/playlists/p?force=1", &[], ""),
        get("/api/v1/server?callback=f"),
        patch_at("/api/v1/playlists/p?limit=5", r#"{"name":"x"}"#),
        // A key the body type does not name, on a handler that never reads
        // the body's fields.
        bearer("POST", "/api/v1/notes", &[JSON], r#"{"pinned":true}"#),
        bearer("POST", "/api/v1/notes?limit=5", &[JSON], "{}"),
        // A body is one object: not the fields in order, and not a scalar.
        patch(r#"["x"]"#),
        patch(r#""x""#),
        bearer("POST", "/api/v1/notes", &[JSON], "[]"),
        // The path's `id` given again in the body.
        patch(r#"{"id":"q","name":"x"}"#),
        bearer(
            "POST",
            "/api/v1/admin/accounts/acc_1/disable",
            &[JSON],
            r#"{"id":"acc_2","user":"usr_b"}"#,
        ),
    ];
    let expected: Vec<TestResponse> = (0..27).map(invalid).collect();
    assert_eq!(send_all(&client, requests), expected);
    assert_eq!(seen.handled(), [] as [&str; 0]);
    assert_eq!(
        send_all(
            &client,
            vec![
                bearer("GET", "/api/v1/tracks?limit=500", &[], ""),
                bearer("GET", "/api/v1/tracks", &[], ""),
                bearer("GET", "/api/v1/tracks?", &[], ""),
                bearer("GET", "/api/v1/tracks?cursor=n1&limit=2", &[], ""),
                patch(" {\"name\":\"x\"}"),
                bearer(
                    "POST",
                    "/api/v1/notes",
                    &[JSON],
                    r#"{"text":"a","tags":[1]}"#
                ),
            ]
        ),
        [
            json(200, r#"{"limit":500}"#),
            json(200, r#"{"limit":null}"#),
            json(200, r#"{"limit":null}"#),
            json(200, r#"{"limit":2}"#),
            json(200, r#"{"id":"p","name":"x"}"#),
            empty(),
        ]
    );
}

/// Verifies: SEC-API-013, SEC-HIS-009
#[test]
fn takes_the_acting_principal_only_from_the_credential() {
    let (client, _) = client();
    let patch = |body: &str| bearer("PATCH", "/api/v1/playlists/p", &[JSON], body);
    let requests = vec![
        patch(r#"{"name":"x","owner":"usr_b"}"#),
        patch(r#"{"name":"x","userId":"usr_b"}"#),
        patch(r#"{"name":"x","uid":"usr_b"}"#),
        patch(r#"{"name":"x","meta":{"ownerId":"b"}}"#),
        bearer("GET", "/api/v1/tracks?user_id=usr_b", &[], ""),
        bearer("GET", "/api/v1/tracks?Profile=prf_b", &[], ""),
        bearer("GET", "/api/v1/me?household=h", &[], ""),
        // No route may declare `owner` or `user_id` any more (the table
        // refuses one that does), so these two name a field the notes
        // route does not take as well as a principal.
        bearer("POST", "/api/v1/notes", &[JSON], r#"{"owner":"usr_b"}"#),
        bearer("POST", "/api/v1/notes?user_id=usr_b", &[JSON], "{}"),
        // The notes body declares `extra` and takes any object of strings
        // in it, so only the principal check refuses this one.
        bearer(
            "POST",
            "/api/v1/notes",
            &[JSON],
            r#"{"extra":{"owner":"usr_b"}}"#,
        ),
    ];
    let expected: Vec<TestResponse> = (0..10).map(invalid).collect();
    assert_eq!(send_all(&client, requests), expected);
    // The same body with a key that names no principal is accepted.
    assert_eq!(
        client.send(bearer(
            "POST",
            "/api/v1/notes",
            &[JSON],
            r#"{"extra":{"colour":"red"}}"#
        )),
        empty()
    );
    // An admin route that acts on other principals may name one.
    assert_eq!(
        client.send(bearer(
            "POST",
            "/api/v1/admin/accounts/acc_1/disable",
            &[JSON],
            r#"{"user":"usr_b"}"#
        )),
        json(200, r#"{"id":"acc_1","user":"usr_b"}"#)
    );
}

/// Verifies: SEC-API-073
#[test]
fn answers_when_the_request_id_or_the_log_hook_panics() {
    let seen = Arc::new(Seen::default());
    let mut drawing = hooks(&seen, false);
    drawing.request_id = Arc::new(|| std::panic::resume_unwind(Box::new("no randomness")));
    let client = TestClient::new(router(&entries(&seen), drawing).unwrap());
    let internal = problem("internal_error", 500, INTERNAL, 0, &[]);
    assert_eq!(
        send_all(&client, vec![get("/api/v1/server"), get("/api/v1/nowhere")]),
        [internal.clone(), internal]
    );
    let unnumbered = AccessRecord {
        request: RequestId(0),
        method: Some(Method::Get),
        route: None,
        status: 500,
        panic: Some("no randomness".to_owned()),
    };
    assert_eq!(seen.records(), [unnumbered.clone(), unnumbered]);
    assert_eq!(seen.handled(), [] as [&str; 0]);

    let seen = Arc::new(Seen::default());
    let mut logging = hooks(&seen, false);
    logging.log =
        Arc::new(|_: &AccessRecord, _: &_| std::panic::resume_unwind(Box::new("disk full")));
    let client = TestClient::new(router(&entries(&seen), logging).unwrap());
    assert_eq!(
        send_all(
            &client,
            vec![
                get("/api/v1/server"),
                get("/api/v1/nowhere"),
                bearer("GET", "/api/v1/me", &[], "")
            ]
        ),
        [
            json(200, r#"{"name":"gm"}"#),
            not_found(1),
            json(200, r#"{"principal":"Bearer alice"}"#),
        ]
    );
    assert_eq!(seen.handled(), ["me"]);
}

/// Verifies: SEC-API-073, SEC-TM-040, SEC-API-072
#[test]
fn turns_a_panic_into_the_generic_500_and_rolls_back() {
    let (client, seen) = client();
    let internal = |request: u8| {
        TestResponse {
        status: 500,
        headers: headers(167, &[("content-type", "application/problem+json")]),
        body: format!(
            r#"{{"type":"urn:gunmetal:problem:internal_error","title":"Something went wrong on the server. Try again later.","status":500,"request":"0000000000000000000000000000000{request}"}}"#
        )
        .into_bytes(),
    }
    };
    assert_eq!(
        client.send(get("/api/v1/test/panic")),
        TestResponse {
            status: 500,
            headers: vec![
                ("cache-control".to_owned(), "no-store".to_owned()),
                ("content-length".to_owned(), "167".to_owned()),
                (
                    "content-security-policy".to_owned(),
                    "default-src 'none'; frame-ancestors 'none'; sandbox".to_owned()
                ),
                ("content-type".to_owned(), "application/problem+json".to_owned()),
                ("cross-origin-resource-policy".to_owned(), "same-origin".to_owned()),
                ("referrer-policy".to_owned(), "no-referrer".to_owned()),
                ("x-content-type-options".to_owned(), "nosniff".to_owned()),
            ],
            body: br#"{"type":"urn:gunmetal:problem:internal_error","title":"Something went wrong on the server. Try again later.","status":500,"request":"00000000000000000000000000000000"}"#.to_vec(),
        }
    );
    assert_eq!(seen.events(), ["begin", "rollback"]);
    assert_eq!(
        send_all(
            &client,
            vec![
                get("/api/v1/test/panic-text"),
                get("/api/v1/test/panic-other")
            ]
        ),
        [internal(1), internal(2)]
    );
    assert_eq!(
        seen.events(),
        [
            "begin", "rollback", "begin", "rollback", "begin", "rollback"
        ]
    );
    let panicked = |request, route, text: &str| AccessRecord {
        request: RequestId(request),
        method: Some(Method::Get),
        route: Some(route),
        status: 500,
        panic: Some(text.to_owned()),
    };
    assert_eq!(
        seen.records(),
        [
            panicked(
                0,
                "/api/v1/test/panic",
                "index out of bounds: the len is 2 but the index is 3"
            ),
            panicked(
                1,
                "/api/v1/test/panic-text",
                "SELECT secret FROM /srv/gunmetal/data.db failed in rusqlite 0.40.2"
            ),
            panicked(2, "/api/v1/test/panic-other", ""),
        ]
    );
    // The server keeps answering.
    assert_eq!(
        client.send(get("/api/v1/server")),
        json(200, r#"{"name":"gm"}"#)
    );
    assert_eq!(
        INTERNAL,
        "Something went wrong on the server. Try again later."
    );
}

/// Verifies: SEC-API-072
#[test]
fn renders_handler_errors_from_the_catalogue() {
    let (client, _) = client();
    assert_eq!(client.send(get("/api/v1/test/missing")), not_found(0));
}

#[test]
fn runs_handlers_that_wait() {
    let (client, _) = client();
    assert_eq!(
        client.send(get("/api/v1/test/slow")),
        json(200, r#"{"name":"slow"}"#)
    );
}

#[test]
fn runs_the_posture_access_and_rate_hooks_in_order() {
    let (client, seen) = client();
    let as_bearer = |method, target, token, body: &str| {
        request(
            method,
            target,
            &[("host", HOST), ("authorization", token)],
            body,
        )
    };
    assert_eq!(
        send_all(
            &client,
            vec![
                // The posture hook runs before the host check.
                request(
                    "GET",
                    "/api/v1/server",
                    &[("host", "attacker.example"), ("x-test-posture", "1")],
                    ""
                ),
                as_bearer("GET", "/api/v1/me", "Bearer refused", ""),
                as_bearer("GET", "/api/v1/me", "Bearer throttled", ""),
                // The access and rate hooks run before the body is read or
                // its type checked.
                as_bearer("POST", "/api/v1/notes", "Bearer refused", &"x".repeat(100)),
                as_bearer(
                    "POST",
                    "/api/v1/notes",
                    "Bearer throttled",
                    &"x".repeat(100)
                ),
            ]
        ),
        [
            problem("audit_unavailable", 503, AUDIT, 0, &[]),
            not_found(1),
            problem("audit_unavailable", 503, AUDIT, 2, &[]),
            not_found(3),
            problem("audit_unavailable", 503, AUDIT, 4, &[]),
        ]
    );
    assert_eq!(
        credentials(&seen),
        [
            ("/api/v1/me", Some(b"Bearer refused".to_vec())),
            ("/api/v1/me", Some(b"Bearer throttled".to_vec())),
            ("/api/v1/notes", Some(b"Bearer refused".to_vec())),
            ("/api/v1/notes", Some(b"Bearer throttled".to_vec())),
        ]
    );
    assert_eq!(
        seen.rated(),
        [(RateClass::Read, None), (RateClass::Write, None)]
    );
    assert_eq!(seen.handled(), [] as [&str; 0]);
}

/// Verifies: SEC-API-038
#[test]
fn sends_hsts_only_over_https_under_a_host_name() {
    let (client, _) = client_with(true);
    let hsts = ("strict-transport-security", "max-age=31536000");
    assert_eq!(
        client.send(get("/api/v1/server")),
        TestResponse {
            status: 200,
            headers: headers(13, &[("content-type", "application/json"), hsts]),
            body: br#"{"name":"gm"}"#.to_vec(),
        }
    );
    assert_eq!(
        client.send(get("/api/v1/nowhere")),
        problem("not_found", 404, support::NOT_FOUND, 1, &[hsts])
    );
    for host in ["localhost", "192.168.1.20"] {
        assert_eq!(
            client.send(request("GET", "/api/v1/server", &[("host", host)], "")),
            json(200, r#"{"name":"gm"}"#)
        );
    }
    assert_eq!(
        client.send(request(
            "GET",
            "/api/v1/server",
            &[("host", "attacker.example")],
            ""
        )),
        problem("unknown_host", 421, UNKNOWN_HOST, 4, &[])
    );
    let (plain, _) = support::client();
    assert_eq!(
        plain.send(get("/api/v1/server")),
        json(200, r#"{"name":"gm"}"#)
    );
}

/// Verifies: SEC-API-040, SEC-IAM-014, SEC-API-053, SEC-API-054, SEC-CLI-004, SEC-PRV-020, SEC-STD-013
#[test]
fn sends_the_golden_headers_and_no_cors_or_server_header() {
    let (client, _) = client();
    let golden = |length: &str, kind: &[(&str, &str)]| {
        let mut all = vec![
            ("cache-control", "no-store"),
            ("content-length", length),
            (
                "content-security-policy",
                "default-src 'none'; frame-ancestors 'none'; sandbox",
            ),
        ];
        all.extend_from_slice(kind);
        all.extend_from_slice(&[
            ("cross-origin-resource-policy", "same-origin"),
            ("referrer-policy", "no-referrer"),
            ("x-content-type-options", "nosniff"),
        ]);
        all.sort_unstable();
        all.into_iter()
            .map(|(name, value)| (name.to_owned(), value.to_owned()))
            .collect::<Vec<(String, String)>>()
    };
    let problem_type = ("content-type", "application/problem+json");
    let preflight = request(
        "OPTIONS",
        "/api/v1/me",
        &[
            ("host", HOST),
            ("origin", "https://evil.example"),
            ("access-control-request-method", "DELETE"),
            ("access-control-request-headers", "gunmetal-request"),
        ],
        "",
    );
    // A callback parameter is refused: nothing answers as a script.
    let foreign = bearer("GET", "/api/v1/me?callback=f", &[("origin", "null")], "");
    let sets: Vec<(u16, Vec<(String, String)>)> = send_all(
        &client,
        vec![
            bearer(
                "GET",
                "/api/v1/me",
                &[("origin", "https://evil.example")],
                "",
            ),
            bearer("DELETE", "/api/v1/playlists/p", &[("origin", "null")], ""),
            preflight,
            foreign,
            get("/api/v1/nowhere"),
            get("/api/v1/me?token=a"),
            request("GET", "/api/v1/me", &[("host", "evil.example")], ""),
            bearer("DELETE", "/api/v1/playlists/p", &[JSON], ""),
            bearer("DELETE", "/api/v1/playlists/p", &[], "x"),
            get("/api/v1/test/panic-other"),
            request(
                "GET",
                "/api/v1/me",
                &[("host", HOST), ("x-test-posture", "1")],
                "",
            ),
        ],
    )
    .into_iter()
    .map(|response| (response.status, response.headers))
    .collect();
    assert_eq!(
        sets,
        [
            (200, golden("28", &[("content-type", "application/json")])),
            (204, golden("0", &[])),
            (405, golden("152", &[problem_type, ("allow", "GET")])),
            (400, golden("156", &[problem_type])),
            (404, golden("192", &[problem_type])),
            (400, golden("208", &[problem_type])),
            (421, golden("153", &[problem_type])),
            (415, golden("173", &[problem_type])),
            (413, golden("141", &[problem_type])),
            (500, golden("167", &[problem_type])),
            (503, golden("187", &[problem_type])),
        ]
    );
}

/// The whole catalogue a response may draw on, written out: type, status
/// and title.
const CATALOGUE: [(&str, u16, &str); 10] = [
    ("urn:gunmetal:problem:audit_unavailable", 503, AUDIT),
    ("urn:gunmetal:problem:body_too_large", 413, TOO_LARGE),
    (
        "urn:gunmetal:problem:credential_misplaced",
        400,
        support::MISPLACED,
    ),
    (
        "urn:gunmetal:problem:cross_site_request",
        403,
        support::CROSS_SITE,
    ),
    ("urn:gunmetal:problem:internal_error", 500, INTERNAL),
    (
        "urn:gunmetal:problem:invalid_request",
        400,
        support::INVALID,
    ),
    ("urn:gunmetal:problem:method_not_allowed", 405, NOT_ALLOWED),
    ("urn:gunmetal:problem:not_found", 404, support::NOT_FOUND),
    ("urn:gunmetal:problem:unknown_host", 421, UNKNOWN_HOST),
    ("urn:gunmetal:problem:unsupported_body", 415, UNSUPPORTED),
];

/// A request path that is a variant of a real one: `base` with a suffix, or
/// with the character at `at` percent-encoded or in the other case.
fn variant(base: &str, at: usize, kind: u8, tail: &str) -> String {
    let at = 1 + at % (base.len() - 1);
    let (before, rest) = base.split_at(at);
    let (this, after) = rest.split_at(1);
    let separators = ["/", ".", ";", "%2F", "%00", "//", "/.", "/..", "%20", ":"];
    match kind {
        0 => format!("{before}%{:02x}{after}", this.as_bytes()[0]),
        1 if this != this.to_ascii_uppercase() => {
            format!("{before}{}{after}", this.to_ascii_uppercase())
        }
        _ => format!("{base}{}{tail}", separators[at % separators.len()]),
    }
}

proptest! {
    /// Verifies: SEC-API-055
    #[test]
    fn no_suffix_delimiter_case_or_encoding_variant_reaches_a_route(
        base in prop_oneof![
            Just("/api/v1/server"),
            Just("/api/v1/tracks"),
            Just("/api/v1/me"),
            Just("/api/v1/test/slow"),
        ],
        at in 0_usize..64,
        kind in 0_u8..3,
        tail in "[a-z0-9]{0,6}",
    ) {
        let (client, seen) = client();
        let target = variant(base, at, kind, &tail);
        prop_assert_eq!(client.send(bearer("GET", &target, &[], "")), not_found(0));
        prop_assert_eq!(seen.accessed(), []);
        prop_assert_eq!(client.send(bearer("GET", base, &[], "")).status, 200);
    }

    /// Verifies: SEC-API-072, SEC-TM-040
    #[test]
    fn every_error_is_a_catalogue_problem_that_echoes_nothing(
        method in prop_oneof![
            Just("GET"), Just("POST"), Just("PATCH"), Just("DELETE"), Just("HEAD"), Just("BREW"),
        ],
        target in prop_oneof![
            Just("/api/v1/server".to_owned()),
            Just("/api/v1/playlists/p".to_owned()),
            Just("/api/v1/notes".to_owned()),
            Just("/api/v1/tracks".to_owned()),
            Just("/api/v1/admin/accounts/a/disable".to_owned()),
            Just("/api/v1/test/panic-text".to_owned()),
            "/[a-z/.;%0-9]{0,24}",
        ],
        query in prop_oneof![Just(String::new()), "\\?[a-zA-Z_=&%0-9]{0,24}"],
        host in prop_oneof![Just(HOST), Just("evil.example")],
        kind in prop_oneof![Just(None), Just(Some("application/json")), Just(Some("text/xml"))],
        credential in prop_oneof![
            Just(None),
            Just(Some(("authorization", "Bearer alice"))),
            Just(Some(("authorization", "Bearer refused"))),
            Just(Some(("cookie", "__Host-gm_session=s"))),
        ],
        body in prop_oneof![
            Just(String::new()),
            Just(r#"{"name":"x"}"#.to_owned()),
            "[ -~]{0,80}",
            "[\\[\\]{}\",:a1 ]{0,80}",
        ],
    ) {
        let (client, _) = client();
        let mut all = vec![("host", host)];
        all.extend(kind.map(|kind| ("content-type", kind)));
        all.extend(credential);
        let response = client.send(request(method, &format!("{target}{query}"), &all, &body));
        if method == "HEAD" {
            // No route may declare HEAD, so it is always refused, and axum
            // answers it without the body it was given, as HTTP requires.
            prop_assert!([400, 404, 405, 421].contains(&response.status));
            prop_assert_eq!(response.body, Vec::<u8>::new());
        } else if response.status >= 400 {
            let fields: BTreeMap<String, serde_json::Value> =
                serde_json::from_slice(&response.body).unwrap();
            let text = |name: &str| fields.get(name).and_then(|v| v.as_str()).unwrap_or("");
            let status = fields.get("status").and_then(serde_json::Value::as_u64);
            prop_assert_eq!(
                fields.keys().map(String::as_str).collect::<Vec<_>>(),
                ["request", "status", "title", "type"]
            );
            prop_assert_eq!(status, Some(u64::from(response.status)));
            prop_assert!(CATALOGUE.contains(&(text("type"), response.status, text("title"))));
            prop_assert_eq!(text("request"), "00000000000000000000000000000000");
        } else {
            prop_assert!([200, 204].contains(&response.status));
        }
    }
}
