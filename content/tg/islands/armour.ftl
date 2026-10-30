# S5 · This page's armour (web/instruments/armour.ts).
# A live scan in the visitor's browser: the headers of a HEAD request to this page, and /_proof.

armour-status-scanning = санҷида истодаем
armour-status-done = санҷиши зинда
armour-scanning = Сарлавҳаҳое, ки ин саҳифа бо онҳо омадааст, хонда мешаванд…
armour-summary = Дар ҷавоби ин саҳифа { $ok } аз { $n } сарлавҳаи амният мавҷуд аст.
armour-summary-http = HSTS танҳо тавассути HTTPS фиристода мешавад, бинобар ин дар ин сервери таҳия ба ҳисоб гирифта намешавад.
armour-head-src = Ҳозир дар браузери шумо аз дархости HEAD ба ин саҳифа хонда шуд.
armour-head-error = Сарлавҳаҳои ин саҳифаро хондан нашуд ({ $reason }). Барои кӯшиши дубора саҳифаро аз нав бор кунед.
armour-offline = Шумо офлайн ҳастед, бинобар ин сарлавҳаҳои ин саҳифаро аз сервери мо гирифтан ғайриимкон аст. Вақте ки пайваст барқарор шавад, санҷиш худ ба худ оғоз меёбад.

armour-caption = Сарлавҳаҳои амният, ки ин саҳифа бо онҳо омадааст
armour-col-header = Сарлавҳа
armour-col-status = Ҳолат
armour-col-value = Қимат
armour-st-ok = ҳаст
armour-st-weak = заиф
armour-st-missing = нест
armour-st-na = танҳо HTTPS
armour-not-sent = фиристода нашуд
armour-https-only = танҳо тавассути HTTPS фиристода мешавад

armour-why-content-security-policy = Скриптҳои воридшударо манъ мекунад: танҳо файлҳои худи мо иҷро шуда метавонанд ва Trusted Types ҳар нуқтаи воридкунии HTML-ро муҳофизат мекунанд.
armour-why-cross-origin-opener-policy = Тирезаҳои сомонаҳои дигарро ба раванди ин саҳифа роҳ намедиҳад.
armour-why-cross-origin-embedder-policy = Танҳо захираҳоеро бор мекунад, ки ба ҷойгиршавӣ розӣ ҳастанд — ин ҷудокуниро аз манбаъҳои дигар пурра мекунад.
armour-why-cross-origin-resource-policy = Ба сомонаҳои дигар ҷойгир кардани файлҳои моро манъ мекунад.
armour-why-permissions-policy = Имкониятҳои браузерро, ки ин саҳифа истифода намебарад, хомӯш мекунад: камера, микрофон ва ҷойгиршавӣ.
armour-why-referrer-policy = Ба сомонаҳои дигар намегӯяд, ки шумо аз кадом саҳифа омадед.
armour-why-x-content-type-options = Ба браузер намегузорад, ки навъи файлҳоро тахмин занад, бинобар ин маълумот ҳеҷ гоҳ ҳамчун скрипт иҷро намешавад.
armour-why-strict-transport-security = Ба браузери шумо мегӯяд, ки ин сомонаро танҳо тавассути HTTPS кушояд, ҳатто агар дар пайванд http навишта шуда бошад.

armour-tls-title = Пайваст
armour-tls-version = Версияи TLS
armour-tls-cipher = Маҷмӯи шифрҳо
armour-tls-group = Мубодилаи калид
armour-tls-unreported = гузориш дода нашуд
armour-pq-badge-hybrid = постквантӣ
armour-pq-badge-pq = постквантӣ
armour-pq-badge-classical = классикӣ
armour-pq-badge-unknown = гузориш дода нашуд
armour-pq-badge-plain = бе TLS
armour-pq-badge-edge = дар канори хостинг
armour-pq-hybrid = Пайвасти шумо { $group }-ро истифода бурд — омехтаи мубодилаи калиди классикӣ ва постквантӣ.
armour-pq-pq = Пайвасти шумо { $group }-ро истифода бурд — мубодилаи калиди постквантӣ.
armour-pq-classical = Браузери шумо дар бораи { $group } (классикӣ) мувофиқа кард. Мо мубодилаи калиди постквантиро ба ҳар браузере, ки онро дастгирӣ мекунад, пешниҳод мекунем.
armour-pq-unknown = Сервери мо муайян карда натавонист, ки ин пайваст кадом мубодилаи калидро истифода бурд, бинобар ин ба ҷои тахмин ҳеҷ чиз гузориш намедиҳад.
armour-pq-plain = Сервери таҳия: HTTP-и оддӣ, бе TLS.
armour-pq-edge = HTTPS барои ин суроға дар прокси-сервери канории провайдери хостинги мо анҷом меёбад, бинобар ин сервери мо намебинад, ки браузери шумо дар бораи кадом версияи TLS, шифр ва мубодилаи калид мувофиқа кардааст, ва тахмин ҳам намезанад. Онҳоро лавҳаи маълумоти сомона дар браузери шумо нишон медиҳад.
armour-tls-src = Аз /_proof: он чи сервери мо бо браузери шумо барои ин пайваст мувофиқа кард.
armour-tls-error = Ба /_proof расидан нашуд ({ $reason }), бинобар ин тафсилоти пайваст нест. Барои кӯшиши дубора саҳифаро аз нав бор кунед.

armour-iso-title = Ҷудокунӣ
armour-coi = Ҷудо аз манбаъҳои дигар
armour-coi-yes = ҳа
armour-coi-no = не
armour-coi-yes-line = COOP ва COEP кор мекунанд: ин саҳифа дар гурӯҳи контекстҳои тамошои худаш иҷро мешавад ва ин инчунин SharedArrayBuffer-ро дастрас мекунад.
armour-coi-no-line = Ин саҳифа аз манбаъҳои дигар ҷудо нест, гарчанде ки COOP ва COEP фиристода шуданд.
armour-coi-no-headers-line = Ин ҷавоб ҳамзамон COOP ва COEP надошт, бинобар ин саҳифа аз манбаъҳои дигар ҷудо нест.
armour-tt = Trusted Types
armour-tt-enforced = фаъол
armour-tt-open = фаъол нест
armour-tt-unsupported = дастгирӣ намешавад
armour-tt-enforced-line = Мо кӯшиш кардем, ки сатри оддиро ба innerHTML-и як унсури муваққатӣ нависем. Браузери шумо онро манъ кард.
armour-tt-open-line = Мо кӯшиш кардем, ки сатри оддиро ба innerHTML-и як унсури муваққатӣ нависем ва он гузашт, гарчанде ки сиёсати мо Trusted Types-ро талаб мекунад.
armour-tt-open-policy-line = Мо кӯшиш кардем, ки сатри оддиро ба innerHTML-и як унсури муваққатӣ нависем ва он гузашт: Content Security Policy-и ин ҷавоб Trusted Types-ро талаб намекунад.
armour-tt-unsupported-line = Браузери шумо Trusted Types-ро дастгирӣ намекунад. Content Security Policy ба ҳар ҳол скриптҳои дарунсохтро манъ мекунад.
armour-iso-src = Ҳангоми боршавии ин лавҳа дар браузери шумо санҷида шуд.
