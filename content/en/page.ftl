# Mintaka — page copy. English is the source language.
# Russian and Tajik were drafted by Claude at the founder's request (2026-10-30), for native review
# (docs/TRANSLATION.md).
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
# A/B test alternatives to hero-title (spec §S1; content/site.toml [experiment]). Each enabled
# language needs them while the test runs.
hero-title-hardware = As fast as the hardware allows.
hero-title-cycle = Engineered to the last cycle.
hero-sub = Fast by proof. Secure by attack. Scaled to the metal. Intuitive by experiment.
hero-cta = Start a project
hero-proof = See the proof ↓
hero-note = This page measures itself. Press **P** for the proof console.
# The button that pauses the hero's particle animation, and starts it again.
hero-motion-pause = Pause the animation
hero-motion-play = Play the animation

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
sec-fuzz-id = S5 · Attack
sec-fuzz-title = Attack our code
sec-fuzz-fallback = With JavaScript and WebAssembly on, a fuzzer attacks a real Rust parser of ours right here, finds its one bug, and then fails to break the fixed version.
sec-fuzz-foot = The fuzzer and the parser are Rust, compiled to WebAssembly. It runs only in your browser, only against this parser, and sends nothing anywhere.
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
# Post titles and summaries live in each post's front matter: content/{lang}/lab/*.md
lab-description = Long-form notes from Mintaka: the problem, the method, the mathematics, the code, and a benchmark you can reproduce.
lab-all = All posts in the Lab
# Shown in a language that has no posts of its own yet, above the default language's posts.
lab-untranslated = The Lab's posts are in English only, for now:
lab-feed = Atom feed
lab-feed-title = The Lab — Mintaka
lab-feed-follow = Follow the Lab with an Atom feed
lab-summary = Summary
lab-byline = Mintaka
lab-updated = updated
lab-code-for = Code:
lab-reproduce = Reproduce it
lab-reproduce-text = Everything this post describes is in our public repository, and each number in it names the machine, the date and the command that produced it. The code is here:
lab-formula = Formula
lab-code = { $lang } code
lab-code-titled = { $lang } code: { $title }
lab-code-lines = lines { $from }–{ $to }
lab-lang-rust = Rust
lab-lang-sh = Shell
lab-lang-toml = TOML
lab-lang-ftl = Fluent
lab-lang-text = Text

## S12 Team — Bellatrix

team-eyebrow = Team
team-title = From the Pamir to Orion.
# Draft — the team should rewrite this in its own words.
team-body = We're a young team of engineers from Dushanbe, Tajikistan — friends with one shared obsession: how things really work. We take software apart down to the kernel and build it back faster, safer and simpler.
team-photo-alt = Portrait of { $name }
# One card per person in content/site.toml [[team]]. The lines are drafts — their owners should
# rewrite them. An optional team-<id>-photo replaces the portrait's generic alt text.
team-bilol-name = Sharifzoda Bilol
team-bilol-role = Founder
team-bilol-line = I architect high-load Rust backends for fintech, telecom and government, and tune them for low-end phones on unstable networks.
team-amirhamza-name = Amirhamza Sayfiddinzoda
team-amirhamza-role = Co-founder
team-amirhamza-line = I take web products from architecture to release, in React, Node.js, Python and Go, and teach others to do the same.
team-amirhamza-photo = Amirhamza Sayfiddinzoda, seen from behind in a cap
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
pv-lead = The short version: we don't track you. We keep totals only: how fast our pages load for visitors and how often a few features are used. This page is the whole policy.
pv-cookies-title = No cookies, no trackers
pv-cookies = This site sets no cookies, loads nothing from other companies and runs no third-party analytics. Our own measurement, described below, keeps totals only. Your browser can confirm the rest: open the proof console (press P).
pv-logs-title = Server logs
pv-logs = Our server logs the time, the page, the response status and how long it took. It does not store or log IP addresses, browser details or what you type.
pv-measure-title = Page speed and feature counts
pv-measure = When you leave a page or switch away from it, your browser sends our server one small message. It says which of our pages it was, from a fixed list; five speed measurements of that visit (time to first byte, first and largest contentful paint, layout shift and the slowest interaction); how many times you opened the proof console or the terminal, followed a link to the contact form, sent the form, switched language, started or finished an instrument and solved a stage of the console challenge; and which of the four ways your device drew the opening animation. It carries no cookie, identifier, IP address, browser details or anything you typed. The server adds it to totals in memory (counts, and 75th percentiles since it started and for the last 24 hours) and keeps nothing else: nothing is written to disk, and the totals start again from zero when the server restarts. To stop one network from flooding the totals, it counts messages per network (the first three parts of the address), in memory only. Anyone can read the totals in the proof console or at /_rum. If your browser sends Global Privacy Control or Do Not Track, it measures and sends nothing.
pv-contact-title = When you write to us
pv-contact = Your message is encrypted the moment it reaches our server and is read only by the team, to reply to you. Spam protection uses a small calculation your browser solves and a short-lived counter per network, kept in memory only.
pv-telegram-title = If you continue in Telegram
pv-telegram = After you send the form, its last screen may offer a link to our Telegram bot. If you follow it, the bot connects that chat to your brief, and our server keeps, in a file only it can read: the chat's numeric id, your brief's number and language, when the chat was connected and when it was last used, and the numbers of that conversation's messages in our team's Telegram chat. It keeps that for 30 days after the last message in either direction; sending /stop to the bot deletes it at once. Our server doesn't store or log your messages: the bot copies them into the team's chat on Telegram, where they stay like any chat history, and copies the team's answers back to you. Anything sent through Telegram is also covered by Telegram's own privacy policy.
pv-physics-title = The speed-of-light measurement
pv-physics = It estimates your location from your device's time zone, inside your browser. Nothing about it is sent to us.
pv-experiments-title = The experiments
pv-experiments = The Fitts, Hick–Hyman and pop-out experiments measure your clicks and reaction times inside your browser. The measurements never leave it; only the fact that you started or finished one is counted, as described above.
# Shown while content/site.toml has an [experiment]; describes that test, so update it with each new one.
pv-ab-title = Testing the home page
pv-ab = We are testing { $versions } versions of the headline at the top of the home page. Each time our server sends that page, it picks one of them at random — with no cookie and no record of who got which, so you may see another one next time. While the test runs, the message described above also names the version you saw, and says, once per visit, whether you followed a link to the contact form and whether you sent the form. The server adds that to totals per version, in memory like the rest: page views, and how many of them did each of those two things. Once every version has { $views } page views counted, a number fixed before the test began, the counting stops and the proof console shows whether the difference between the versions is larger than chance.
pv-storage-title = What your browser keeps
pv-storage = If you switch the keyboard shortcuts off, your browser remembers that choice in its own storage. After your first visit it also keeps a copy of the pages you opened and their files, so the site opens without a connection (a service worker; clearing this site's data in your browser removes it). If you press Send on the contact form while offline, your draft stays in that tab's storage until it is sent or the tab is closed. If you start the console challenge, your browser keeps your progress (a token our server issued) until you start over or clear this site's data; it goes to our server only when you ask for a puzzle or check an answer. Nothing else here is sent to us.
pv-data-title = Map data
pv-data = Time zone locations come from the IANA time zone database (public domain).
pv-hosting-title = Where this site runs
pv-hosting = Our server runs in { $city }, { $country }, on a machine operated by { $provider }. Messages sent through the contact form are stored there, encrypted.
pv-hosting-edge = Your browser's encrypted connection ends at the hosting provider's edge proxy, which passes your request to our server together with your network address — used only for the short-lived counters described above. That proxy belongs to { $provider } and may keep its own connection logs. Other applications share this domain; any cookies they set are theirs, and our server ignores them.
pv-contact-us = Questions about privacy: { $email }

## Telegram bot (crates/server/src/telegram): what it writes to a client who continues in
## Telegram after sending the form, in the brief's language (docs/CONTRACTS.md §19). Rendered at
## build time: { $within } is the reply time from site.toml, { $site } this language's home page;
## { $brief } is the brief's number, which the server fills in.
bot-linked = Thank you — your brief has reached us. Its number is { $brief }. This chat is now connected to it: write here to add anything, files included, and the team will answer you here within { $within }. Send /stop to disconnect this chat.
bot-again = This chat is already connected to your brief { $brief }. Write here, and the team will answer you here.
bot-taken = This brief is already connected to another Telegram chat. If that wasn't you, send us a new brief at { $site }.
bot-expired = This link has expired. Send us a new brief at { $site } — it takes a minute.
bot-stopped = Done: this chat is no longer connected to your brief, and nothing more goes to the team from here. To connect it again, open the link from the site.
bot-hello = This bot answers briefs sent from our site. Send yours at { $site }, then follow the Telegram link on the screen that confirms it.

## curl mode (plain-text pages for terminals)

curl-tagline = software at the edge of physics
curl-sub = Rust-first software engineering · Dushanbe, Tajikistan
curl-served = served in
curl-fast = fast by proof
curl-secure = secure by attack
curl-scaled = scaled to the metal
curl-intuitive = intuitive by experiment
curl-cookies = 0 cookies
curl-build = build

## Footer

ft-tagline = Software at the edge of physics.
ft-made = Made in Dushanbe, Tajikistan.
ft-sheet = Sheet
ft-build = Build
ft-served = This page
ft-served-fallback = Measured live with JavaScript on.
ft-served-static = Runs no JavaScript, so nothing here is measured live. Each number in a post names its machine, date and command.
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

## Landing page (the studio's front door at /{lang}/studio/; the lab stays at /{lang}/).
## Reuses hero-eyebrow, hero-title, nav-cta, nav-efficiency/-security/-highload/-design,
## skip-link, lang-*, ft-made, ft-copyright, ft-tagline, ft-privacy, ft-security.

landing-meta-title = Mintaka — a Rust-first software studio
landing-meta-description = A software studio from Dushanbe. Fast by proof, secure by attack, scaled to the metal, intuitive by experiment — with a lab that measures every claim, so you need not trust ours.
landing-to-lab = The lab

landing-hero-lead = We build high-load Rust backends and the fast interfaces on top of them — tuned for cheap phones and unstable networks. Every promise on this page is measured next door, in a lab you can attack yourself.
landing-hero-lab = See the proof

landing-pillars-eyebrow = What we do
landing-pillars-title = Four promises, each with a number behind it.
landing-pillars-lead = We take on new products, performance rescues, security audits, design and AI integration. Each promise below comes with a number from our own production system.
landing-des-title = Interfaces that feel instant, even on a cheap phone
landing-high-title = Systems that hold at peak traffic
landing-sec-title = Software hardened by attacking it first
landing-eff-title = Backends that answer in microseconds

landing-eff-desc = Rust to the metal. We profile until the hot path is the hardware's limit, not ours.
landing-eff-proof = app response, p99 under 50 µs
landing-sec-desc = A strict policy the browser enforces on every page — and a penetration test we ran on ourselves.
landing-sec-proof = 0 trackers · 0 cookies
landing-high-desc = Built for the busy hour. We load-test to the point of pain, then engineer past it.
landing-high-proof = 4,000 requests a second on two cores
landing-des-desc = Accessible, measured, fast on a cheap phone and a slow network. Speed is how good it feels.
landing-des-proof = WCAG 2.2 AA · first screen under 14 kB

landing-method-eyebrow = How we work
landing-method-title = Measure. Build. Prove.
landing-method-lead = The three stars of Orion's Belt — and the three steps every project passes through.
landing-measure-title = Measure
landing-measure-desc = We start with numbers: budgets, baselines, and the physics of the problem.
landing-build-title = Build
landing-build-desc = Rust, no shortcuts — reviewed, tested and benchmarked as it grows.
landing-prove-title = Prove
landing-prove-desc = We attack it, measure it, and show the receipts. Only then does it ship.

landing-lab-title = Don't take our word for it.
landing-lab-body = Our lab is a live engineering notebook: a page that measures itself, invites you to attack it, and shows the physics underneath. It is built exactly the way we build for you.
landing-lab-cta = Open the lab

landing-contact-title = Tell us what you're building.
landing-contact-direct = Or reach us directly

landing-footer-note = This page runs { $kb } kB of JavaScript — all of it ours, and none of it needed to read a word.
landing-sky-label = The constellation Orion, from real sky coordinates, drawn into the Mintaka mark: an hourglass with Betelgeuse in rust at one corner and Mintaka, the studio's star, in blue on the Belt.
landing-here = Dushanbe · 38°33′ N 68°47′ E

landing-eff-unit = µs
landing-eff-caption = The time our server spends on a request, at the 99th percentile — under a steady 4,000 requests a second.
landing-sec-caption = Third-party requests, trackers or cookies on this site.
landing-high-unit = req/s
landing-high-caption = On a two-core machine in Dushanbe, with the processor under a third busy.
landing-des-caption = The first screen of this page, compressed — and every page audited to WCAG 2.2 AA.

landing-readout-title = This page, measured in your browser
landing-readout-note = Read just now from your browser's own timing records. Nothing leaves your device.
landing-readout-html = The page, compressed
landing-readout-total = Everything it loaded
landing-readout-js = JavaScript, all ours
landing-readout-third = Third-party requests
landing-readout-ttfb = First byte
landing-readout-fcp = First paint
landing-unit-kb = kB
landing-unit-ms = ms
