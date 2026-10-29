# S5 · Attack our code (web/instruments/fuzzer). A coverage-guided fuzzer attacks a small Rust
# parser of ours in the visitor's browser. Coverage points and parse errors are numbered as in
# crates/fuzzer/src/points.rs and target.rs: keep the ids.
fuzzer-status-nowasm = WebAssembly unavailable
fuzzer-status-loading = loading the fuzzer
fuzzer-status-failed = couldn't load
fuzzer-status-ready = in your browser

## Chapters and controls

fuzzer-chapter-attack = Attack
fuzzer-chapter-fix = Fix
fuzzer-chapter-compare = Compare
fuzzer-lead = A fuzzer throws millions of malformed inputs at code until something breaks. This one runs in your browser, against a real Rust parser of ours with one real bug in it. It starts from four meaningless bytes and no hints.
fuzzer-start = Unleash the fuzzer
fuzzer-fix-next = Fuzz the fixed build
fuzzer-blind-next = Now try it blind
fuzzer-again = Run it again
fuzzer-stop = Stop
fuzzer-speed = Speed
fuzzer-speed-watch = Watch
fuzzer-speed-full = Full speed
fuzzer-watch-hint = Slowed down so you can follow it. Full speed finds the bug in milliseconds.

## Live counters

fuzzer-stat-inputs = Inputs tried
fuzzer-stat-rate = Per second
fuzzer-stat-paths = Code paths
fuzzer-stat-crashes = Crashes
fuzzer-stat-time = Time

## Log

fuzzer-log-label = Fuzzer log
fuzzer-log-start = fuzzing { $build } · { $mode } · seed { $seed }
fuzzer-log-new = new path
fuzzer-log-crash = CRASH
fuzzer-log-done = stopped · { $inputs } inputs · { $paths } paths
fuzzer-build-buggy = v1 (buggy)
fuzzer-build-fixed = v2 (fixed)
fuzzer-mode-guided = coverage-guided
fuzzer-mode-blind = blind random

## Result cards

fuzzer-crash-title = Crash found after { $inputs } inputs · { $time }
fuzzer-crash-panic = Rust’s own panic message, captured in your browser:
fuzzer-crash-min = Shrunk from { $from } bytes to the { $to } that matter:
fuzzer-crash-why = The name section’s length byte promised more bytes than the chart has. Line { $line } trusted it.
fuzzer-crash-safe = In Rust this is a clean, safe stop. In C the same line could read past the end of the buffer. Either way, it never ships.
fuzzer-crash-fix = The fix, one line:
fuzzer-fixed-title = Fixed build: { $inputs } inputs, 0 crashes
fuzzer-fixed-body = { $paths } of { $total } code paths reached. The one left dark is a chart named exactly MINTAKA: seven exact bytes with no branch in between are nearly invisible to coverage feedback. Production fuzzers add dictionaries and comparison tracing for that.
fuzzer-blind-title = Blind: { $paths } of { $total } paths in { $inputs } inputs
fuzzer-blind-body = Random bytes, no feedback, { $time } of your CPU. Guided by coverage, the same engine found the bug after { $guided } inputs. Feedback is the whole trick.
fuzzer-blind-body-no-guided = Random bytes, no feedback, { $time } of your CPU. Guided by coverage, the same engine finds the bug in a fraction of a second.

## Try it yourself

fuzzer-try-title = Try to crash it yourself
fuzzer-try-lead = Type a chart as text with \xNN escapes, or as hex pairs. Up to 64 bytes.
fuzzer-try-format = Format: “ORI” · version (1–2) · star count (1–7) · 4 bytes per star (x, y, magnitude, flags) · then, in version 2, sections: C (lines) and N (name: length byte, then the name).
fuzzer-try-label = Your input
fuzzer-try-run = Run it
fuzzer-try-build = Against
fuzzer-try-example-valid = A valid chart
fuzzer-try-example-betelgeuse = A Betelgeuse
fuzzer-try-ok = Parsed cleanly · reached { $paths } code paths.
fuzzer-try-rejected = Rejected safely: { $reason } · reached { $paths } code paths.
fuzzer-try-invalid = That isn’t a valid escape. Use \x followed by two hex digits.
fuzzer-try-crashed = You crashed it. You think like an attacker — we’d like to meet you.
fuzzer-try-careers = See open roles

## Screen-reader announcements

fuzzer-announce-start = Fuzzer started.
fuzzer-announce-paths = { $paths } of { $total } code paths found.
fuzzer-announce-crash = Crash found after { $inputs } inputs.
fuzzer-announce-done = Run finished: { $inputs } inputs, { $paths } paths, { $n ->
    [one] { $n } crash
   *[other] { $n } crashes
}.

## Why a chart was rejected (parse errors in crates/fuzzer/src/target.rs)

fuzzer-reason-bad-magic = it doesn’t start with “ORI”
fuzzer-reason-bad-version = unknown version
fuzzer-reason-no-stars = a chart needs at least one star
fuzzer-reason-too-many-stars = Orion has seven stars, not more
fuzzer-reason-truncated = the chart is cut short
fuzzer-reason-reserved-flags = reserved flag bits are set
fuzzer-reason-unknown-section = unknown section
fuzzer-reason-bad-line = a line points to a star that isn’t there

## Coverage points, by id (crates/fuzzer/src/points.rs). Keep the ids.

fuzzer-point-0 = magic “O”
fuzzer-point-1 = magic “OR”
fuzzer-point-2 = magic “ORI”
fuzzer-point-3 = version 1
fuzzer-point-4 = version 2
fuzzer-point-5 = unknown version
fuzzer-point-6 = chart with no stars
fuzzer-point-7 = more than 7 stars
fuzzer-point-8 = star count accepted
fuzzer-point-9 = star records cut short
fuzzer-point-10 = first star read
fuzzer-point-11 = second star
fuzzer-point-12 = third star — a belt
fuzzer-point-13 = seventh star — all of Orion
fuzzer-point-14 = brighter than magnitude 0
fuzzer-point-15 = first magnitude
fuzzer-point-16 = second magnitude
fuzzer-point-17 = faint star
fuzzer-point-18 = north-east quadrant
fuzzer-point-19 = north-west quadrant
fuzzer-point-20 = south-east quadrant
fuzzer-point-21 = south-west quadrant
fuzzer-point-22 = variable star
fuzzer-point-23 = multiple star
fuzzer-point-24 = giant
fuzzer-point-25 = supergiant
fuzzer-point-26 = reserved flag bits
fuzzer-point-27 = a Betelgeuse: bright variable supergiant
fuzzer-point-28 = a Mintaka: multiple giant
fuzzer-point-29 = clean end of chart
fuzzer-point-30 = unknown section
fuzzer-point-31 = lines section
fuzzer-point-32 = lines checked
fuzzer-point-33 = line to a missing star
fuzzer-point-34 = name section
fuzzer-point-35 = name length read
fuzzer-point-36 = name read
fuzzer-point-37 = name in capitals
fuzzer-point-38 = a chart named MINTAKA

## Star names on the sky. English names are used where these are absent.

fuzzer-star-meissa = Meissa
fuzzer-star-betelgeuse = Betelgeuse
fuzzer-star-bellatrix = Bellatrix
fuzzer-star-mintaka = Mintaka
fuzzer-star-alnilam = Alnilam
fuzzer-star-alnitak = Alnitak
fuzzer-star-rigel = Rigel
fuzzer-star-saiph = Saiph
