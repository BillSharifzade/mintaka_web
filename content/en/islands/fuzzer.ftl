# S5 · Attack our code (web/instruments/fuzzer). A coverage-guided fuzzer attacks a small Rust
# parser of ours in the visitor's browser. What it shows before the visitor starts it; the rest
# (log, results, coverage points, "Try to crash it yourself") is in lazy/fuzzer.ftl.
fuzzer-status-nowasm = WebAssembly unavailable
fuzzer-status-loading = loading the fuzzer
fuzzer-status-failed = couldn't load
fuzzer-status-ready = in your browser

## Chapters and controls

fuzzer-chapter-attack = Attack
fuzzer-chapter-fix = Fix
fuzzer-chapter-compare = Compare
fuzzer-lead = A fuzzer throws millions of malformed inputs at code until something breaks. This one runs in your browser, against a real Rust parser of ours with one real bug in it. It starts from four meaningless bytes and no hints.
fuzzer-start = Run the fuzzer
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

## Try it yourself

fuzzer-try-title = Try to crash it yourself

## Star names on the sky. English names are used where these are absent.

fuzzer-star-meissa = Meissa
fuzzer-star-betelgeuse = Betelgeuse
fuzzer-star-bellatrix = Bellatrix
fuzzer-star-mintaka = Mintaka
fuzzer-star-alnilam = Alnilam
fuzzer-star-alnitak = Alnitak
fuzzer-star-rigel = Rigel
fuzzer-star-saiph = Saiph
