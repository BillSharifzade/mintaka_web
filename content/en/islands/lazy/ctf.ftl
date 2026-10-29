# The console challenge (web/console/ctf.ts): four puzzles, one per pillar star, played from the
# browser console. A lazy bundle: shipped as /assets/strings/ctf.<lang>.<hash>.json and fetched on
# the first challenge command, not inlined into every page. The puzzle data comes from /api/ctf/*;
# these lines explain it.

ctf-intro = The console challenge: four puzzles, one for each pillar's star. Each answer unlocks the next, and the fourth opens an application form. Every run gets its own data from our server.
ctf-commands = mintaka.puzzle() returns this stage's data, mintaka.flag(answer) checks an answer, mintaka.hint() gives a hint, mintaka.restart() starts a new run.
ctf-stage = Stage { $n } of 4 · { $title }
ctf-rigel-title = Rigel · efficiency
ctf-rigel = grid is a square sky of nebula densities, one digit per cell. Entering a cell costs its digit; the top-left cell, where you start, is free. Moving up, down, left or right, find the cheapest route to the bottom-right cell. The flag is its total cost.
ctf-rigel-hint = Moving only right and down gives the wrong number here. The costs are small whole numbers, so Dijkstra with a simple queue is plenty.
ctf-saiph-title = Saiph · security
ctf-saiph = a and b are two messages encrypted with the same one-time pad (each byte XORed with the same key byte), which is exactly what a one-time pad must never do. a is a sentence we show everyone who reads this page's source. The flag is the last word of b.
ctf-saiph-hint = a XOR b is the two plaintexts XORed together. View the page source and read the greeting at the top.
ctf-betelgeuse-title = Betelgeuse · highload
ctf-betelgeuse = tcp_info is what getsockopt(TCP_INFO) returned for one connection: Linux's struct tcp_info on x86-64, little-endian, as hex. How many bytes per second can this connection send before its congestion window, not the network, becomes the limit? The flag is the window in bytes divided by the smoothed round-trip time, rounded down.
ctf-betelgeuse-hint = The layout is in linux/tcp.h, and our own server reads the same struct. The window is counted in segments, the round-trip time in microseconds.
ctf-meissa-title = Meissa · design
ctf-meissa = trials are pointing measurements from a simulated usability test: target distance and width in pixels, then movement time in milliseconds. Fit Fitts's law, MT = a + b × log2(D/W + 1), by ordinary least squares. The flag is the throughput, 1000 / b bits per second, rounded to one decimal.
ctf-meissa-hint = S7 on this page fits the same law to your own clicks. The formula is log2(D/W + 1), with the + 1.
ctf-right = Correct. { $star } is lit.
ctf-wrong = Not that one. mintaka.hint() if you want a nudge.
ctf-done = All four stars are lit. Your application form is open in the Team section: mintaka.apply() takes you there.
ctf-not-done = The application form opens after the fourth stage. mintaka.ctf() shows where you are.
ctf-apply-title = Apply
ctf-apply-lead = You solved the console challenge, and your application will say so when it reaches us. Tell us who you are and what you would like to build.
ctf-apply-name = Name
ctf-apply-email = Email
ctf-apply-telegram = Telegram
ctf-apply-rule = An email or a Telegram handle: one is enough.
ctf-apply-work = Your work
ctf-apply-work-hint = A link to code you are proud of, and a few lines about it.
ctf-apply-send = Send application
ctf-apply-sending = Sending…
ctf-apply-sent = Sent. We read every application and answer each one.
