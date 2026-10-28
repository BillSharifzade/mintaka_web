## S6 · Overload lab (web/instruments/loadsim/): a simulation, compiled from Rust to WebAssembly
## and run in the visitor's browser. Every number it shows comes from that run — nothing here is
## measured on our servers, and the copy must keep saying so.

loadsim-status = simulation
loadsim-loading = loading
loadsim-nowasm = This browser can't run WebAssembly, so the simulation can't start here.
loadsim-failed = The simulation didn't load, so there's nothing to run here.
loadsim-load = Load
loadsim-load-value = ×{ $x } — { $rate } requests/s
loadsim-pause = Pause
loadsim-play = Play
loadsim-reset = Start over
loadsim-about = Simulation, not a measurement of this site. Requests arrive at random (Poisson), { $base } a second at ×1, and each needs { $mean } of work on average, with a long tail (lognormal). Clients give up after { $timeout } and try again { $n ->
    [one] once
    [2] twice
   *[other] { $n } times
}. Both systems get exactly the same requests.
loadsim-naive = Naive
loadsim-naive-about = { $n ->
    [one] One worker
   *[other] { $n } workers
} and one queue with no limit. Every request waits its turn, even after its client has given up — and then the work is done for nobody.
loadsim-ours = Built our way
loadsim-ours-about = Starts with { $n ->
    [one] one worker
   *[other] { $n } workers
}; an autoscaler adds more, up to { $max }, each taking { $boot } to start. Once requests have queued for longer than { $target } for { $interval }, new ones get an immediate 503 instead, and so do the stale ones; the queue never holds more than { $per } per worker.
loadsim-p99 = p99 latency of successes
loadsim-none = none answered in time
loadsim-served = Answered in time, of offered
loadsim-served-value = { $good } of { $offered }/s
loadsim-refused = Turned away (503) · timed out
loadsim-refused-value = { $shed } · { $timeouts }/s
loadsim-state = Queue and workers
loadsim-waiting = { $n ->
    [one] { $n } request waiting
   *[other] { $n } requests waiting
}
loadsim-busy = { $busy } of { $n ->
    [one] { $n } worker
   *[other] { $n } workers
} busy
loadsim-starting = { $n } starting
loadsim-wasted = { $n }/s of work for clients who had given up
loadsim-limit = (the simulation's limit)
loadsim-little = { $l } in the system ≈ { $lambda }/s × { $w } = { $lw }
loadsim-little-about = Little's law: the average number of requests in a system equals the rate they leave times how long each stays. All three are measured separately over the last 4 s; they agree while a system is steady and part ways while its queue grows or drains.
loadsim-legend = Charts show the last minute on the same scales in both panels. Latency uses a log scale from 10 ms to 10 s; the dashed line is the clients' { $timeout } timeout. To bound memory, the naive queue stops at { $cap } waiting requests; after that, new ones are lost and their clients time out.
loadsim-try = Try ×40 for half a minute, then go back to ×1: the naive system stays down until it has worked through the requests nobody is waiting for any more.
