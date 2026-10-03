# 11. GPUI is not adopted for the apps

Date: 2026-10-02
Status: accepted

Record numbers 3 to 10 are reserved by the build plan for records its wave
0 packages write.

## Context

GPUI is the GPU-accelerated Rust UI framework behind the Zed editor. The
owner asked whether Gunmetal should build its interface on it, or on a
component library built for it (Ely-GPUI-Components).

As of October 2026:

- GPUI is excellent on desktop (macOS, Windows, Linux) and has a new web
  backend that draws into a single canvas.
- It has no TV platform support and no remote-control focus navigation.
  An iOS backend was opened as a pull request on 22 August 2026 and is not
  merged; Android support exists only in a community project.
- It has no video surface, and no built-in integration with lock screens,
  AirPlay, Chromecast, background audio or CarPlay.
- We found no evidence of screen-reader support, which the design language
  requires.
- Zed's maintainers said early in 2026 that GPUI development would slow to
  focus on Zed itself, and a community fork exists. There is no stable
  crates.io release.
- Ely-GPUI-Components describes itself as early-stage, is tested on macOS
  only, and depends on a pinned Zed revision.

## Decision

Gunmetal does not use GPUI or Ely-GPUI-Components for its apps. The shared
React Native interface (record 1, decision 8) stays.

## When to revisit

GPUI is worth reconsidering for a dedicated native desktop app, where it
would be small and fast and could link the Rust core directly, once all
three of these hold:

1. GPUI's maintenance has settled, upstream or in a community fork.
2. It supports screen readers.
3. Embedding libmpv video in it has been demonstrated.

## Consequences

- No change to the client plan.
- A native desktop app remains a possible later differentiator, tracked by
  the conditions above rather than by a release.
