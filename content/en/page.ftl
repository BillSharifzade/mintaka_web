# Mintaka — page copy. English is the source language.
# Russian and Tajik are written natively by the team, never machine-translated.
# Inline syntax understood by the generator: `code` and **strong**.
# Rule: no invented numbers. Numbers on the page are measured live or computed from a cited source.

## Document

meta-title = Mintaka — software at the edge of physics
meta-description = Rust-first software engineering from Dushanbe. Fast by proof, secure by attack, scaled to the metal, intuitive by experiment — and this page measures itself to prove it.
skip-link = Skip to content
lang-switch = Language
lang-name-en = English
lang-name-ru = Русский
lang-name-tg = Тоҷикӣ

## Header and navigation

nav-label = Main
nav-efficiency = Efficiency
nav-security = Security
nav-highload = Highload
nav-design = Design
nav-process = How we work
nav-team = Team
nav-cta = Start a project
nav-menu = Sections
orion-label = Sections, drawn as the stars of Orion
orion-betelgeuse = Betelgeuse — Highload
orion-meissa = Meissa — Design
orion-bellatrix = Bellatrix — Team
orion-belt = The belt — How we work
orion-saiph = Saiph — Security
orion-rigel = Rigel — Efficiency

## S1 Hero

hero-eyebrow = Rust-first software engineering · Dushanbe
hero-title = Software at the edge of physics.
hero-sub = Fast by proof. Secure by attack. Scaled to the metal. Intuitive by experiment.
hero-cta = Start a project
hero-proof = See the proof ↓
hero-note = This page measures itself. Press **P** for the proof console.

## Instruments

instrument-static = static

## S2 Edge of physics

physics-eyebrow = The speed of light
physics-title = The only speed limit we accept is the speed of light.
physics-lead = Light in glass fibre covers 204 km every millisecond. Whatever your request spends beyond that goes to cable detours, routers and software. Software is the part we remove.
physics-instrument-id = S2 · Physics
physics-instrument-title = Your request against the speed of light
physics-static-lead = From Dushanbe, light in fibre needs at least this long for a round trip:
physics-static-city = City
physics-static-distance = Great-circle distance
physics-static-floor = Round trip, at least
physics-foot = Method: distance on the great circle × 2 ÷ 204.2 km/ms (the speed of light divided by the group index of standard fibre, 1.468). Real cables are longer, so real round trips are slower.
city-almaty = Almaty
city-dubai = Dubai
city-moscow = Moscow
city-frankfurt = Frankfurt
city-new-york = New York

## S3 Constellation

pillars-eyebrow = Four pillars
pillars-title = Four stars. One system.
pillars-lead = Everything we build passes through the same narrow channel — the belt in our mark — and comes out faster, safer, able to carry more, and obvious to use.
pillar-efficiency = Efficiency
pillar-efficiency-line = Faster by mathematics, not by hardware.
pillar-security = Security
pillar-security-line = We attack our own work first.
pillar-highload = Highload
pillar-highload-line = Built for the day traffic goes supernova.
pillar-design = Human-intuitive design
pillar-design-line = Designed for how minds actually work.
pillar-go = Go to { $name }

## Stars (facts: spec Appendix D — keep this careful wording)

star-rigel = Rigel
star-rigel-fact = Usually the brightest star in Orion. The name comes from the Arabic for “foot”.
star-saiph = Saiph
star-saiph-fact = From the Arabic saif, “sword”. A sword cuts both ways.
star-betelgeuse = Betelgeuse
star-betelgeuse-fact = A red supergiant. In our Sun's place it would reach past the orbit of Mars.
star-meissa = Meissa
star-meissa-fact = Marks Orion's head. The name means “the shining one”.
star-bellatrix = Bellatrix
star-bellatrix-fact = Latin for “female warrior”.
star-alnitak = Alnitak
star-alnitak-fact = “The girdle”.
star-alnilam = Alnilam
star-alnilam-fact = “The string of pearls”.
star-mintaka = Mintaka
star-mintaka-fact = “The belt”.
coords-source = Positions: J2000, ICRS.

## S4 Efficiency — Rigel

eff-eyebrow = Efficiency
eff-title = Faster by mathematics, not by hardware.
eff-lead = Light-speed software starts long before the server: in the algorithm, the data structure and the type system. We engineer each from first principles — and give every project its own stack instead of reusing the last one.
eff-algo = Algorithm engineering
eff-algo-text = Complexity analysis first, then the best known algorithm — adapted to your real data.
eff-data = Data structures
eff-data-text = Cache-aware layouts, compact and lock-free structures, SIMD where it pays.
eff-arch = Architecture
eff-arch-text = The simplest shape that meets the numbers. Nothing distributed that doesn't need to be.
eff-types = Types
eff-types-text = Rust's type system makes invalid states impossible to represent, so whole classes of bugs never compile.
eff-research = Research to production
eff-research-text = Papers turned into tested, benchmarked code.
eff-stack = An individual stack
eff-stack-text = Chosen per project after measuring — never by habit.
eff-race-id = S4 · Algorithms
eff-race-title = Route across Dushanbe
eff-race-fallback = With JavaScript and WebAssembly on, three route-finding algorithms race across Dushanbe's real road network, right here in your browser.
eff-race-foot = Map data © OpenStreetMap contributors (ODbL), extract of { $date }. Timings are measured in your browser, on your device.
eff-code-title = Parse, don't validate
eff-code-caption = An `Email` cannot exist unless it was checked. The compiler enforces it everywhere after.
eff-code-file = src/email.rs

## S5 Security — Saiph

sec-eyebrow = Security
sec-title = We attack our own work first.
sec-lead = Every product goes through offensive and defensive experiments before it meets the world — on the same scientific footing as our code. Our own house follows the same rules.
sec-offence = Offence
sec-offence-note = We try to break it
sec-defence = Defence
sec-defence-note = So it holds
sec-off-1 = Threat modelling for every system
sec-off-2 = Penetration tests before each release
sec-off-3 = Fuzzing (`cargo-fuzz`) and property-based tests (`proptest`)
sec-off-4 = Supply-chain audits with `cargo-audit`, `cargo-deny` and `cargo-vet`
sec-off-5 = Static analysis and review checklists
sec-off-6 = Red-team exercises against our own staging
sec-def-1 = Memory safety by default, with Rust
sec-def-2 = Least privilege everywhere
sec-def-3 = Secrets in a vault, never in code
sec-def-4 = Mutual TLS between services
sec-def-5 = Detection, alerting and incident runbooks
sec-def-6 = Backups proven by restore drills
sec-armour-title = Our own armour
sec-armour-lead = The rules we hold ourselves to before we ask you to trust us.
armour-keys = Hardware security keys on every account
armour-signed = Signed commits and signed releases
armour-sbom = Reproducible builds and a software bill of materials for everything we ship
armour-secrets = Secret scanning on every push
armour-access = Least-privilege access, reviewed every quarter
armour-laptops = Encrypted, managed laptops
armour-drills = Incident drills every quarter
armour-disclosure = Responsible disclosure through `/.well-known/security.txt`
claim-unverified = Unverified — confirm before launch
sec-shield-id = S5 · Attacks
sec-shield-title = Try to break this
sec-shield-fallback = With JavaScript and WebAssembly on, paste an attack string here and watch it taken apart: what it tries to do, and which layer of our stack stops it. Nothing you type leaves your browser.
sec-instrument-id = S5 · Armour
sec-instrument-title = This page's armour, inspected live
sec-instrument-fallback = With JavaScript on, this panel reads the security headers and the TLS handshake this page was delivered with, and checks them in front of you.
sec-ctf = Found the console yet? There's a challenge waiting. Solve it and we'd like to meet you.

## S6 Highload — Betelgeuse

hl-eyebrow = Highload
hl-title = Built for the day traffic goes supernova.
hl-lead = We optimise every layer — software, hardware and network — until a system serves as much as its machines physically can. We get there by studying the internals of Linux, containers and Kubernetes, not just their manuals.
hl-layers = What we tune
hl-software = Software
hl-software-items = Async Rust (Tokio) · zero-copy parsing · allocation-free hot paths · backpressure · caching
hl-hardware = Hardware
hl-hardware-items = CPU pinning and NUMA · cache-line layout · SIMD · huge pages · NVMe with io_uring
hl-network = Network
hl-network-items = TCP tuning (BBR, buffers) · SO_REUSEPORT · HTTP/2 and HTTP/3 · kernel TLS · eBPF/XDP filtering
hl-platform = Platform
hl-platform-items = Namespaces and cgroups — what a container really is · Kubernetes scheduling and limits · autoscaling on latency · chaos tests
hl-trace-id = S6 · Request
hl-trace-title = Follow one request: yours
hl-trace-fallback = With JavaScript on, this panel replays your own page request hop by hop — browser, kernel, Rust and back — with the timings your browser and our server measured.
hl-live-id = S6 · Load
hl-live-title = Live load on this website
hl-live-fallback = With JavaScript on, this panel streams this server's real, aggregated traffic once a second.
hl-lab-id = S6 · Simulation
hl-lab-title = Overload lab
hl-lab-fallback = With JavaScript and WebAssembly on, this simulation lets you multiply the load on two systems — one naive, one built our way — and watch which one keeps its promises.
hl-story-title = Even supergiants have incidents
hl-story = In late 2019 Betelgeuse dimmed so much that people noticed it with the naked eye. It had thrown off a huge cloud of gas that cooled into dust. We build systems that dim gracefully instead of going dark.
hl-story-source = Montargès et al., “A dusty veil shading Betelgeuse during its Great Dimming”, Nature, 2021.

## S7 Human-intuitive design — Meissa

ds-eyebrow = Human-intuitive design
ds-title = Designed for how minds actually work.
ds-lead = Intuitive isn't a feeling we guess at. We study perception and attention, run experiments with real people, and measure until an interface feels obvious — and comfortable for every sense it touches.
ds-practice = How we work on it
practice-research = User research and interviews
practice-usability = Usability testing in short, frequent rounds
practice-ab = A/B and preference tests
practice-walkthrough = Cognitive walkthroughs
practice-wcag = Accessibility to WCAG 2.2 AA as a minimum
practice-comfort = Perceptual comfort: contrast, rhythm, motion with purpose
practice-speed = Speed as UX: feedback within 100 ms feels instant
ds-laws-id = S7 · Experiments
ds-laws-title = Laws you can feel
ds-laws-fallback = With JavaScript on, three 20-second experiments let you feel Fitts's law, the Hick–Hyman law and visual pop-out with your own hand and eyes. Your measurements never leave your browser.
ds-proof = This page is built to the same standard: keyboard first, WCAG 2.2 AA, motion only when you allow it.

## S8 The belt — how we work

pr-eyebrow = How we work
pr-title = Three stars. Three steps. One proof.
pr-measure = Measure
pr-measure-text = We profile what exists, model the threats, interview users and agree on success in numbers: latency, throughput, error budgets, task success.
pr-build = Build
pr-build-text = Rust-first and type-driven. Reviewed and tested — unit, property-based, fuzz — with performance and security gates in CI, and a demo every week.
pr-prove = Prove
pr-prove-text = Load tests, a penetration test and a usability test. Then we hand over the proof.
pr-pack-title = The Proof Pack ships with every project
pr-pack-lead = Most studios hand over code. We hand over evidence.
pr-pack-perf = Performance report
pr-pack-perf-text = Benchmarks, method, hardware, flame graphs, before and after.
pr-pack-sec = Security report
pr-pack-sec-text = Threat model, test scope, findings and fixes, software bill of materials.
pr-pack-ux = Usability report
pr-pack-ux-text = Tasks, success rate, time on task, System Usability Scale score, recordings with consent.
pr-pack-ops = Operations handbook
pr-pack-ops-text = Architecture, runbooks, dashboards, service-level objectives, on-call guide.

## S9 Also in every system

q-eyebrow = Also in every system
q-title = The qualities nobody demos.
q-ai = AI-integrated
q-ai-text = AI where it earns its place: search and assistants over your own data, private models on your own infrastructure — evaluated and monitored like any other component.
q-ext = Extensible
q-ext-text = Modular architecture, documented APIs (OpenAPI), clear extension points.
q-durable = Durable and robust
q-durable-text = Graceful degradation, chaos tests, backups proven by restores, service-level objectives.
q-support = Supportable
q-support-text = Documentation, runbooks, observability, handover training and support plans.

## S10 Proof wall

pw-eyebrow = Results
pw-title = Proof wall
pw-lead = Client results with the numbers before and after. We publish them only with our clients' approval — the first ones are on their way.
pw-self-tag = Case zero
pw-self-title = This page
pw-self-text = A landing page that has to prove every claim it makes, with zero third-party code and no cookies. Open the proof console and check our work.
pw-self-cta = Open the proof console

## S11 The Lab

lab-eyebrow = Research
lab-title = The Lab
lab-lead = Where we write down what we learn: the problem, the method, the mathematics, the code, and a benchmark you can reproduce.
lab-soon = Being written
lab-post-1 = How this website serves a page in under a millisecond
lab-post-2 = Contraction Hierarchies on Dushanbe's roads
lab-post-3 = Post-quantum TLS in production with rustls
lab-post-4 = Designing one interface for Tajik, Russian and English

## S12 Team — Bellatrix

team-eyebrow = Team
team-title = From the Pamir to Orion.
# Draft — the team should rewrite this in its own words.
team-body = We're a young team of engineers from Dushanbe, Tajikistan — friends with one shared obsession: how things really work. We take software apart down to the kernel and build it back faster, safer and simpler.
team-name-title = Why “Mintaka”
team-name-star = Mintaka is the star at the western end of Orion's belt. The name comes from the Arabic al-mintaqa, “the belt”.
team-name-word = In Tajik, минтақа means “region” or “zone”.
team-name-pass = And the Mintaka Pass is an old Silk Road crossing high in the Karakoram, near the Pamir.
team-name-end = A star, a region and a road between worlds.
team-peak = The Pamir, illustrated. Its highest summit in Tajikistan, Ismoil Somoni Peak, rises to 7,495 m.
team-careers = We hire people who'd find the console challenge fun.
team-join = Write to us about joining

## S13 Start a project

ct-eyebrow = Start a project
ct-title = Tell us what you're building.
ct-lead = A few lines are enough. We reply within { $within }.
ct-kind = What are you building?
ct-kind-new-product = New product
ct-kind-performance = Performance rescue
ct-kind-security-audit = Security audit
ct-kind-design = Design and UX
ct-kind-ai = AI integration
ct-kind-not-sure = Not sure yet
ct-message = Tell us more
ct-message-hint = What's slow? What worries you? Who are your users?
ct-timeline = When do you want to start?
ct-timeline-asap = As soon as possible
ct-timeline-1-3-months = In 1–3 months
ct-timeline-later = Later
ct-timeline-exploring = Just exploring
ct-name = Your name
ct-email = Email
ct-telegram = Telegram
ct-telegram-hint = Your handle, if you'd rather talk on Telegram.
ct-company = Company
ct-optional = optional
ct-contact-rule = Leave an email or a Telegram handle — at least one — so we can reply.
ct-send = Send
ct-direct = Or write to us directly
ct-privacy = We use your message only to reply to you. It is stored encrypted, and this site sets no cookies and loads no trackers.
ct-trap = Leave this field empty

## No-JavaScript result page (/sent/)

sent-title = Your message is on its way.
sent-body = We reply within { $within }.
sent-error-title = Your message wasn't sent.
sent-back = Back to the form
e-name-required = Add your name, so we know who to reply to.
e-name-too-long = Your name is longer than 120 characters. Shorten it and send again.
e-email-invalid = That email address looks incomplete. Check it and send again.
e-telegram-invalid = A Telegram handle is 5–32 letters, digits or underscores and starts with a letter.
e-contact-required = Add an email address or a Telegram handle, so we can reply.
e-message-too-long = Your message is longer than 5,000 characters. Shorten it, or send the rest by email.
e-company-too-long = The company name is longer than 120 characters.
e-choice-invalid = One of the options wasn't recognised. Go back and choose again.
e-pow-invalid = The anti-spam check didn't pass. Go back and send again.
e-pow-expired = The form was open for too long. Go back and send again.
e-rate-limited = Too many messages from your network in the last hour. Try again later, or email us.
e-too-large = That message is too large for the form. Email it to us instead.
e-server-error = Something went wrong on our side and your message wasn't saved. Email us and we'll look into it.

## 404

nf-title = This page drifted out of orbit.
nf-body = Nothing on this site lives at that address.
nf-suggest = The closest page we have:
nf-home = Navigate by the stars

## Privacy

pv-title = Privacy
pv-lead = The short version: we don't track you. This page is the whole policy.
pv-cookies-title = No cookies, no trackers
pv-cookies = This site sets no cookies, loads nothing from other companies, and runs no analytics scripts. Your browser can confirm it: open the proof console (press P).
pv-logs-title = Server logs
pv-logs = Our server logs the time, the page, the response status and how long it took. It does not store or log IP addresses, browser details or what you type.
pv-contact-title = When you write to us
pv-contact = Your message is encrypted the moment it reaches our server and is read only by the team, to reply to you. Spam protection uses a small calculation your browser solves and a short-lived counter per network, kept in memory only.
pv-physics-title = The speed-of-light measurement
pv-physics = It estimates your location from your device's time zone, inside your browser. Nothing about it is sent to us.
pv-experiments-title = The experiments
pv-experiments = The Fitts, Hick–Hyman and pop-out experiments measure your clicks and reaction times inside your browser. The measurements never leave it.
pv-storage-title = Your settings
pv-storage = If you switch the keyboard shortcuts off, your browser remembers that choice in its own storage. It is never sent to us.
pv-data-title = Map data
pv-data = Time zone locations come from the IANA time zone database (public domain).
pv-contact-us = Questions about privacy: { $email }

## curl mode (plain-text pages for terminals)

curl-tagline = software at the edge of physics
curl-sub = Rust-first software engineering · Dushanbe, Tajikistan
curl-served = served in
curl-fast = fast by proof
curl-secure = secure by attack
curl-scaled = scaled to the metal
curl-intuitive = intuitive by experiment

## Footer

ft-tagline = Software at the edge of physics.
ft-made = Made in Dushanbe, Tajikistan.
ft-sheet = Sheet
ft-build = Build
ft-served = This page
ft-served-fallback = Measured live with JavaScript on.
ft-checked = Checked by
ft-checked-value = your browser
ft-promises = What this page promises
ft-promise-1 = No loading screens. Ever.
ft-promise-2 = Every number on this page is live or sourced.
ft-promise-3 = Zero third-party requests. Zero cookies.
ft-promise-4 = Works on a cheap phone, a slow network, a screen reader and with motion off.
ft-promise-5 = Don't take our word for it — try it.
ft-privacy = Privacy
ft-security = security.txt
ft-humans = humans.txt
ft-terminal = Open the terminal
ft-shortcuts = Keyboard shortcuts
ft-shortcuts-keys = ~ terminal · P proof console
ft-copyright = © { $year } Mintaka
