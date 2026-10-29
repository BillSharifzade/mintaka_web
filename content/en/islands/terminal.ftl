# The ~ terminal (web/terminal/index.ts). Command names (help, ping, …) and the prompt are code,
# not copy. Every { $variable } is measured on this page view.
# Messages with { $n } are counts, read with tp(): any of them may select on $n for plurals,
# e.g. { $n ->
#     [one] …
#    *[other] …
# }

terminal-title = Terminal
terminal-keys = ↑ ↓ history · Tab completes · Esc closes
terminal-close = Close
terminal-log = Terminal output
terminal-input = Command
terminal-welcome = Mintaka terminal. Every answer comes from this page view. Type help to see the commands.
terminal-unknown = command not found: { $cmd } — type help
terminal-error = error: { $reason }
terminal-offline = offline: this command needs our server. Try again once you're connected.

terminal-help-help = list the commands
terminal-help-whoami = what your browser tells this page, worked out locally
terminal-help-pillars = the four stars, one line each
terminal-help-ping = { $n ->
    [one] { $n } request
   *[other] { $n } requests
} to /_ping: min, median, p90, max
terminal-help-trace = this page's own request, hop by hop
terminal-help-proof = our server's view of this connection (/_proof)
terminal-help-lang = read this page in another language
terminal-help-sky = pause or resume the star field
terminal-help-ctf = the console challenge
terminal-help-contact = go to the contact form
terminal-help-clear = clear the screen (also Ctrl+L)
terminal-help-exit = close the terminal (also Esc)

terminal-star-rigel = Rigel
terminal-star-saiph = Saiph
terminal-star-betelgeuse = Betelgeuse
terminal-star-meissa = Meissa
terminal-pillar-rigel = Efficiency. Faster by mathematics, not by hardware.
terminal-pillar-saiph = Security. We attack our own work first.
terminal-pillar-betelgeuse = Highload. Built for the day traffic goes supernova.
terminal-pillar-meissa = Human-intuitive design. Designed for how minds actually work.

terminal-yes = yes
terminal-no = no
terminal-unknown-browser = unknown browser
terminal-gpu-no-adapter = the API is there, but no adapter
terminal-who-browser = browser
terminal-who-langs = languages
terminal-who-tier = render tier
terminal-who-tier-none = not reported yet
terminal-who-motion = motion
terminal-motion-reduced = reduced, as your settings ask
terminal-motion-full = full
terminal-who-cores = logical cores
terminal-who-isolated = crossOriginIsolated
terminal-who-gpu = WebGPU
terminal-who-local = Worked out in your browser. Nothing was sent.

terminal-ping-start = PING /_ping: { $n ->
    [one] { $n } request
   *[other] { $n } requests
}, one after another…
terminal-ping-got = { $n } of { $total } answered
terminal-ping-stats = min { $min } · median { $median } · p90 { $p90 } · max { $max }
terminal-ping-server = our server's own time (Server-Timing: app), median { $ms }
terminal-ping-src = Timed by your browser: Resource Timing, requestStart to responseStart.
terminal-ping-src-clock = Timed with the page's clock around fetch(): no Resource Timing entry was recorded for these requests.

terminal-trace-head = Your page request, hop by hop (Navigation Timing and Server-Timing):
terminal-trace-none = No Navigation Timing entry was recorded for this page.
terminal-trace-dns = DNS lookup
terminal-trace-connect = connection (TCP and TLS)
terminal-trace-tls = TLS handshake
terminal-trace-wait = request to first byte
terminal-trace-st-app = our server (app)
terminal-trace-st-krtt = kernel RTT (krtt)
terminal-trace-st-tls = TLS handshake, our side (tls)
terminal-trace-download = download
terminal-trace-total = total
terminal-note-reused = connection reused
terminal-note-plain = plain HTTP (development)

terminal-proof-src = From /_proof, fetched just now.
terminal-lang-same = You're already reading { $code }.
terminal-lang-go = Opening /{ $code }/…
terminal-lang-unknown = There's no { $code } version of this page. Available: { $list }
terminal-lang-list = Usage: lang <code>. Available: { $list }
terminal-sky-on = Star field resumed.
terminal-sky-off = Star field paused.
terminal-sky-none = The star field isn't running on this device, so there's nothing to pause.
terminal-ctf = The challenge opens at launch.
terminal-contact-none = There's no contact form on this page.
