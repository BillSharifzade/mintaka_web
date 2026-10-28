## S7 · Laws you can feel (web/instruments/laws/). Three optional experiments; every measurement
## stays in the visitor's browser. Results are filled in live — never put an example number here.

laws-status = on this device
laws-tabs = Experiments
laws-fitts = Fitts's law
laws-hick = Hick–Hyman law
laws-pop = Pop-out search
laws-start = Start
laws-again = Run again
laws-count = { $n } of { $total }
laws-misses = { $n ->
    [one] { $n } miss
   *[other] { $n } misses
}
laws-local = Measured on this device from your own clicks, taps and key presses. Nothing is sent anywhere: this instrument makes no network requests.
laws-hidden = Stopped: the tab was hidden, which would spoil the timing. Start again when you're ready.
laws-results = Your numbers
laws-not-run = Not run yet.
laws-data = Your trials
laws-col-n = Options
laws-col-ms = Time, ms
laws-a = Intercept a
laws-b = Slope b
laws-r2 = Fit, R²
laws-tp = Throughput
laws-shapes-cue = Find this shape:

laws-fitts-intro = Hit 15 targets of different sizes and distances, as fast as you can. About 20 seconds, with a mouse, a finger or a pen.
laws-fitts-keys = This one measures pointing, so it needs a pointer. With a keyboard, the other two experiments work fully.
laws-fitts-skip = Go to the Hick–Hyman law
laws-fitts-go = Click or tap the circle to start
laws-fitts-x = Difficulty (ID), bits
laws-fitts-plain = Each extra bit of difficulty — about double the distance, or half the size — cost your hand { $b }.
laws-fitts-line = Your hand just obeyed a 1954 equation. Our buttons are sized with it.
laws-fitts-loose = The fit is loose this time. A second run usually tightens it.

laws-hick-intro = Pick the shape that matches, from 2, 4 or 8 options — click or tap it, press its number, or move with the arrow keys and press Enter. 24 quick rounds, about 30 seconds.
laws-hick-block = { $n ->
    [one] { $n } option
   *[other] { $n } options
}
laws-hick-x = Choice, log2(n + 1) bits
laws-hick-plain = Each extra bit of choice — roughly a doubling of the options — cost you { $b }.
laws-hick-line = Every extra choice cost you milliseconds. That's why our menus are short.
laws-hick-flat = This run didn't show the slowdown. Try again, a little faster.

laws-pop-intro = Find the odd star among 60 and click or tap it — or press its quarter of the field, 1 to 4, or move with the arrow keys and press Enter. Three rounds: by colour, by shape, then both. About 25 seconds.
laws-pop-area = Search field. Answer with the quarter of the field the odd star is in: 1 top left, 2 top right, 3 bottom left, 4 bottom right, or the arrow keys and Enter.
laws-pop-colour = Colour
laws-pop-shape = Shape
laws-pop-both = Colour and shape
laws-pop-both-short = Both
laws-pop-plain = Finding colour and shape together took you { $x } as long as finding colour alone.
laws-pop-line = Colour finds things for you. That's why we use rust so rarely: it marks Betelgeuse, and the button that starts a project.
laws-wrong = { $n ->
    [one] { $n } wrong answer
   *[other] { $n } wrong answers
}

laws-shape-circle = circle
laws-shape-ring = ring
laws-shape-square = square
laws-shape-triangle = triangle
laws-shape-diamond = diamond
laws-shape-plus = plus
laws-shape-cross = cross
laws-shape-hexagon = hexagon
