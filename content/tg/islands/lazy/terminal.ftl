# The ~ terminal (web/terminal/index.ts): everything it prints, in a lazy bundle fetched when the
# terminal first opens (/assets/strings/terminal.<lang>.<hash>.json). Its frame and the line that
# reports a failed fetch stay inline in ../terminal.ftl. Command names (help, ping, …) and the
# prompt are code, not copy. Every { $variable } is measured on this page view.
# Messages with { $n } are counts, read with tp(): any of them may select on $n for plurals.

## Lines of its own

terminal-welcome = Терминали Mintaka. Ҳар ҷавоб аз ҳамин тамошои саҳифа гирифта мешавад. Барои дидани фармонҳо help-ро нависед.
terminal-unknown = фармон ёфт нашуд: { $cmd } — help-ро нависед
terminal-offline = офлайн: ин фармон ба сервери мо ниёз дорад. Вақте ки пайваст шудед, боз кӯшиш кунед.

## help

terminal-help-help = рӯйхати фармонҳо
terminal-help-whoami = он чи браузери шумо ба ин саҳифа мегӯяд, дар ҳамин ҷо ҳисобшуда
terminal-help-pillars = чор ситора, ҳар кадом дар як сатр
terminal-help-ping = { $n } дархост ба /_ping: ҳадди ақал, медиана, p90, ҳадди аксар
terminal-help-trace = дархости худи ин саҳифа, қадам ба қадам
terminal-help-proof = ин пайваст аз назари сервери мо (/_proof)
terminal-help-lang = хондани ин саҳифа бо забони дигар
terminal-help-sky = таваққуф ё идомаи майдони ситораҳо
terminal-help-ctf = озмоиши консол
terminal-help-contact = гузариш ба шакли тамос
terminal-help-clear = тоза кардани экран (ё Ctrl+L)
terminal-help-exit = пӯшидани терминал (ё Esc)

## pillars

terminal-star-rigel = Ригел
terminal-star-saiph = Сайф
terminal-star-betelgeuse = Бетелгейзе
terminal-star-meissa = Мейсса
terminal-pillar-rigel = Самаранокӣ. Тезтар бо ёрии математика, на бо сахтафзор.
terminal-pillar-saiph = Амният. Мо аввал ба кори худ ҳамла мекунем.
terminal-pillar-betelgeuse = Сарбории баланд. Барои рӯзе, ки трафик чун супернова аланга мезанад.
terminal-pillar-meissa = Тарроҳии фаҳмо. Мутобиқ ба тарзи воқеии кори зеҳни инсон тарҳрезӣ шудааст.

## whoami

terminal-yes = ҳа
terminal-no = не
terminal-unknown-browser = браузери номаълум
terminal-gpu-no-adapter = API ҳаст, вале адаптер нест
terminal-who-browser = браузер
terminal-who-langs = забонҳо
terminal-who-tier = сатҳи тасвиркашӣ
terminal-who-tier-none = ҳанӯз гузориш нашудааст
terminal-who-motion = аниматсия
terminal-motion-reduced = камшуда, чунон ки танзимоти шумо мехоҳанд
terminal-motion-full = пурра
terminal-who-cores = ядроҳои мантиқӣ
terminal-who-isolated = crossOriginIsolated
terminal-who-gpu = WebGPU
terminal-who-local = Дар браузери шумо ҳисоб карда шуд. Ҳеҷ чиз фиристода нашуд.

## ping

terminal-ping-start = PING /_ping: { $n } дархост, яке паси дигаре…
terminal-ping-got = { $n } аз { $total } ҷавоб дод
terminal-ping-stats = ҳадди ақал { $min } · медиана { $median } · p90 { $p90 } · ҳадди аксар { $max }
terminal-ping-server = вақти худи сервери мо (Server-Timing: app), медиана { $ms }
terminal-ping-src = Браузери шумо вақтсанҷӣ кард: Resource Timing, аз requestStart то responseStart.
terminal-ping-src-clock = Бо соати саҳифа дар атрофи fetch() вақтсанҷӣ шуд: барои ин дархостҳо ягон сабти Resource Timing нест.

## trace

terminal-trace-head = Дархости саҳифаи шумо, қадам ба қадам (Navigation Timing ва Server-Timing):
terminal-trace-none = Барои ин саҳифа ягон сабти Navigation Timing нест.
terminal-trace-dns = ҷустуҷӯи DNS
terminal-trace-connect = пайваст (TCP ва TLS)
terminal-trace-tls = дастфишории TLS
terminal-trace-wait = аз дархост то байти аввал
terminal-trace-st-app = сервери мо (app)
terminal-trace-st-krtt = RTT-и ядро (krtt)
terminal-trace-st-qrtt = RTT-и QUIC (qrtt)
terminal-trace-st-tls = дастфишории TLS, тарафи мо (tls)
terminal-trace-download = боргирӣ
terminal-trace-total = ҳамагӣ
terminal-note-reused = пайваст такроран истифода шуд
terminal-note-plain = HTTP-и оддӣ (таҳия)

## proof, lang, sky, ctf, contact

terminal-proof-src = Аз /_proof, ҳозир гирифта шуд.
terminal-lang-same = Шумо аллакай версияи { $code }-ро мехонед.
terminal-lang-go = /{ $code }/ кушода мешавад…
terminal-lang-unknown = Версияи { $code }-и ин саҳифа нест. Дастрас: { $list }
terminal-lang-list = Истифода: lang <code>. Дастрас: { $list }
terminal-sky-on = Майдони ситораҳо идома ёфт.
terminal-sky-off = Майдони ситораҳо таваққуф шуд.
terminal-sky-none = Майдони ситораҳо дар ин дастгоҳ кор намекунад, бинобар ин чизе барои таваққуф нест.
terminal-ctf = Озмоиш ҳамзамон бо оғози кори сомона кушода мешавад.
terminal-contact-none = Дар ин саҳифа шакли тамос нест.
