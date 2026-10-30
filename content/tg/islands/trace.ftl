## S6 · Follow one request (web/instruments/trace.ts).
## The visitor's own page request, hop by hop: Navigation Timing in their browser plus our
## Server-Timing header. Durations are filled in live — never put an example number here. The
## hops' explainers are in lazy/trace.ftl, fetched when a hop is first opened.

trace-status = чен шуд
trace-status-none = дастрас нест
trace-unsupported = Ин браузер Navigation Timing-ро гузориш намедиҳад, бинобар ин қадамҳои дархости ин саҳифаро дар ин ҷо нишон додан ғайриимкон аст.
trace-cached = Браузери шумо ин саҳифаро аз кеши худаш нишон дод, бинобар ин ягон дархост аз шабака нагузашт. Барои пайгирӣ дархости нав фиристед.
trace-page = Дархости саҳифаи шумо
trace-latest = Дархости охирин
trace-size = { $size } тавассути шабака
trace-total = ҳамагӣ { $total }
trace-again = Дархости дигар фиристед
trace-failed = Дархост нагузашт. Пайвастатонро санҷида, боз кӯшиш кунед.
trace-offline = Шумо офлайн ҳастед, бинобар ин дархост ба сервери мо намерасад. Вақте ки пайваст шудед, онро боз фиристед.
trace-done = Дархости { $n }: ҳамагӣ { $total }, аз он { $app } дар дохили сервери мо.
trace-done-short = Дархости { $n }: ҳамагӣ { $total }.
trace-gap = { $ms }-и боқимонда ба санҷиши кеши браузер ва ба навбат гузоштани дархост рафт.

trace-dns = Ҷустуҷӯи DNS
trace-dns-short = DNS
trace-tcp = Пайваст
trace-tcp-short = TCP
trace-tls = Дастфишории TLS 1.3
trace-tls-short = TLS
trace-tls-server = тарафи мо { $ms }
trace-net = Шабака, рафту баргашт
trace-net-short = ШАБАКА
trace-krtt = RTT-и ядро { $ms }
trace-qrtt = RTT-и QUIC { $ms }
trace-app = Дар дохили сервери мо
trace-app-short = СЕРВЕР
trace-app-split = масир { $route } · рендер { $render }
trace-ttfb = Аз дархост то байти аввал
trace-ttfb-short = ИНТИЗОР
trace-down = Боргирӣ
trace-down-short = ҶАВОБ

trace-zero-reused = пайваст такроран истифода шуд
trace-zero-cached = ҷавоб аллакай дар кеш аст
trace-zero-plain = HTTP-и оддӣ, бе TLS (таҳия)
trace-zero-quic = қисми дастфишории QUIC
