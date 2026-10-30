# S2 · Edge of physics (web/instruments/physics.ts).
# Every { $variable } is a value measured on this page view; the island fills it in.
# { $city } and { $server_city } are place names from the time zone database and the server's
# configuration, in Latin script.

physics-intro = Сервери мо ин саҳифаро дар { $server } сохт.
physics-measure = Масофаи маро чен кунед
physics-measuring = Чен карда истодаем…
physics-again = Боз чен кунед
physics-hint = Рафтуомади шуморо то сервери мо вақтсанҷӣ мекунад ва минтақаи вақти дастгоҳатонро мехонад — ҳамааш дар браузери шумо.
physics-status-ready = омода
physics-status-measured = чен шуд

physics-rtt-label = Рафтуомади шумо
physics-bound-label = Шумо на дуртар аз
physics-floor-label = Ба рӯшноӣ ҳадди ақал лозим аст
physics-src-kernel = Вақти рафтуомади ин пайваст аз рӯи ядрои Linux-и сервери мо (Server-Timing: krtt)
physics-src-navigation = Браузери шумо, аз requestStart то responseStart, бе вақти сервери мо
physics-src-quic = Арзёбии худи QUIC аз вақти рафтуомад барои ин пайвасти HTTP/3, аз рӯи тасдиқҳояш (Server-Timing: qrtt)
physics-src-ping = Тезтарин аз { $n } дархост ба /_ping, бе вақти сервери мо
physics-bound-src = Нисфи рафтуомад × 204,2 км/мс, суръати рӯшноӣ дар шиша (c ÷ 1,468)
physics-floor-src = Минтақаи вақт { $zone }: { $city }, дар масофаи { $km }. 2 × масофа ÷ 204,2 км/мс.
physics-floor-none = Шаҳри минтақаи вақти шумо ва ҷойгиршавии сервери мо лозим аст.

physics-bound = Бастаҳои шумо дар { $rtt } баргаштанд, пас шумо аз сервери мо на бештар аз { $km } дур ҳастед — рӯшноӣ дар шиша дар ин муддат дуртар рафта наметавонад.
physics-floor = Ба рӯшноӣ дар нахи оптикӣ барои рафтан аз { $city } то сервери мо дар { $server_city } ва баргаштан ҳадди ақал { $floor } лозим аст. Рафтуомади шумо { $rtt } вақт гирифт.
physics-gap = { $detours }-и боқимонда ба шабака рафт: кабелҳое, ки аз рӯи доираи калон намераванд, роутерҳо ва пайвасти худи шумо.
physics-server = Сервери мо болои он { $server } илова кард.
physics-faster = Физика мегӯяд, ки шумо дар { $city } буда наметавонед: рафтуомади шумо аз он чи ба рӯшноӣ лозим аст, кӯтоҳтар буд. Шумо аз сервери мо на бештар аз { $km } дур ҳастед.
physics-no-zone = Минтақаи вақти дастгоҳи шумо ({ $zone }) ҷои мушаххасро нишон намедиҳад, бинобар ин чизе барои муқоиса нест.
physics-no-server = Ҷойгиршавии сервери мо танзим нашудааст (сервери таҳия), бинобар ин танҳо ҳадди болоӣ нишон дода мешавад.
physics-no-proof = Ба /_proof барои ҷойгиршавии сервери мо расидан нашуд, бинобар ин танҳо ҳадди болоӣ нишон дода мешавад.
physics-error = Рафтуомадро вақтсанҷӣ кардан нашуд: { $reason }. Боз кӯшиш кунед.
physics-offline = Шумо офлайн ҳастед. Барои ченкунӣ як рафтуомад то сервери мо лозим аст, бинобар ин вақте ки пайваст шудед, тугмаро боз пахш кунед.

physics-bar-label = Вақт ба куҷо рафт · { $total }
physics-part-floor = Ҳадди физика
physics-part-net = Печутоби шабака
physics-part-srv = Сервери мо
physics-part-rtt = Рафтуомади шабака
physics-foot = Ҷойгиршавӣ аз рӯи минтақаи вақти дастгоҳатон дар браузери шумо тахмин зада мешавад. Ба мо ҳеҷ чиз фиристода намешавад.
