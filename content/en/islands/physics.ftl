# S2 · Edge of physics (web/instruments/physics.ts).
# Every { $variable } is a value measured on this page view; the island fills it in.

physics-intro = Our server built this page in { $server }.
physics-measure = Measure my distance
physics-measuring = Measuring…
physics-again = Measure again
physics-hint = Times your round trip to our server and reads your device's time zone, all in your browser.
physics-status-ready = ready
physics-status-measured = measured

physics-rtt-label = Your round trip
physics-bound-label = You are within
physics-floor-label = Light needs at least
physics-src-kernel = Our Linux kernel's round-trip time for this connection (Server-Timing: krtt)
physics-src-navigation = Your browser, from requestStart to responseStart, minus our server's time
physics-src-ping = Fastest of { $n ->
    [one] { $n } request
   *[other] { $n } requests
} to /_ping, minus our server's time
physics-bound-src = Half the round trip × 204.2 km per ms, the speed of light in glass (c ÷ 1.468)
physics-floor-src = Time zone { $zone }: { $city }, { $km } away. 2 × distance ÷ 204.2 km per ms.
physics-floor-none = Needs your time zone's city and our server's location.

physics-bound = Your packets came back in { $rtt }, so you are at most { $km } from our server — light in glass can't go further in that time.
physics-floor = Light in optical fibre needs at least { $floor } to travel from { $city } to our server in { $server_city } and back. Your round trip took { $rtt }.
physics-gap = The other { $detours } is the network: cables that don't follow the great circle, routers and your own connection.
physics-server = Our server added { $server } on top.
physics-faster = Physics says you can't be in { $city }: your round trip was shorter than light needs. You're within { $km } of our server.
physics-no-zone = Your device's time zone ({ $zone }) doesn't name a place, so there is no floor to compare with.
physics-no-server = Our server's location isn't configured (development server), so only the upper bound is shown.
physics-no-proof = Couldn't reach /_proof for our server's location, so only the upper bound is shown.
physics-error = Couldn't time a round trip: { $reason }. Try again.
physics-offline = You're offline. Measuring needs one round trip to our server, so press the button again once you're connected.

physics-bar-label = Where the time went · { $total }
physics-part-floor = Physics floor
physics-part-net = Network detours
physics-part-srv = Our server
physics-part-rtt = Network round trip
physics-foot = Location estimated from your device's time zone, in your browser. Nothing is sent to us.
