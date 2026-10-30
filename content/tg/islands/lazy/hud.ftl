# Proof console (web/hud/index.ts): the panel's rows — labels, sources, values, real visitors and
# the A/B test — in a lazy bundle fetched when the panel first opens
# (/assets/strings/hud.<lang>.<hash>.json). The pill, the footer's live line and the panel's frame
# stay inline in ../hud.ftl. Every { $variable } is measured on this page view.
# { $n } is a count, read with tp() / pluralKey(): select on it for plurals.

hud-pending = ҳанӯз чен нашудааст
hud-na = дар ин браузер дастрас нест
hud-not-reported = сервер гузориш надод
hud-k-server = Вақти сервер
hud-k-weight = Вазни саҳифа
hud-k-third = Дархостҳои тарафи сеюм
hud-k-cookies = Cookie-ҳо
hud-k-tls = Пайваст
hud-k-headers = Сарлавҳаҳои амният
hud-k-lcp = Тасвиркашии калонтарин мундариҷа (LCP)
hud-k-inp = Сусттарин амал то ҳол (INP)
hud-k-cls = Ҷойивазкунии ҷамъшудаи тарҳ (CLS)
hud-k-tier = Сатҳи тасвиркашии саҳнаи аввал
hud-k-visitors = Меҳмонони воқеии ин сервер
hud-src-server = Сарлавҳаи Server-Timing-и ин саҳифа (app): аз таҳлили дархост то омода шудани ҷавоб
hud-src-weight = Resource Timing: ҷамъи transferSize-и ҳамаи дархостҳо то ҳол
hud-src-third = Resource Timing: дархостҳо аз рӯи манбаъ (origin) гурӯҳбандӣ шудаанд
hud-src-cookies = document.cookie ва шумораи Set-Cookie аз /_proof
hud-src-tls = /_proof: версияи TLS, шифр, мубодилаи калид ва версияи HTTP, ки сервери мо мувофиқа кард
hud-src-edge = /_proof мегӯяд, ки дархост аз канори хостинг омадааст; версияи HTTP — ҳамон аст, ки браузери шумо мебинад (Navigation Timing)
hud-src-headers = Дархости HEAD ба ин саҳифа, ки дар браузери шумо хонда шуд
hud-src-lcp = PerformanceObserver: largest-contentful-paint
hud-src-inp = Event Timing: дарозтарин амале, ки то ҳол дар ин саҳифа чен шудааст
hud-src-cls = PerformanceObserver: layout-shift, калонтарин равзанаи сессия
hud-src-tier = Муҳаррики саҳнаи аввал, дар ҳамин дастгоҳ
hud-src-visitors = GET /_rum: Web Vitals, ки браузерҳои меҳмонон аз лаҳзаи оғози кори ин сервер ба он фиристодаанд, ва шумораҳои санҷиши A/B аз рӯи вариантҳо; танҳо ҳамчун рақамҳои ҷамъбастӣ дар хотира нигоҳ дошта мешаванд. Бе cookie, суроғаҳои IP ва маълумоти браузер.
hud-requests = { $n } дархост, { $cached } аз кеш
hud-inp-none = ҳанӯз ягон амал нашудааст
hud-tier-a = A · WebGPU
hud-tier-b = B · WebGL2
hud-tier-c = C · Canvas 2D
hud-tier-d = D · тасвири статикӣ
hud-tier-none = ҳанӯз гузориш нашудааст
hud-plain = HTTP-и оддӣ (таҳия) · { $http }
hud-edge = HTTPS то прокси-сервери канории провайдери хостинги мо · { $http }. Сервери мо дар паси он аст ва тафсилоти TLS-ро намебинад.
hud-sent = фиристода шуд
hud-missing = нест
hud-cookies-ours = аз ҷониби ин сомона гузошта шудааст
hud-cookies-foreign = на аз ҷониби ин сомона
hud-https-only = танҳо HTTPS
hud-err-proof = Ба /_proof расидан нашуд. Барои кӯшиши дубора ин лавҳаро пӯшида, боз кушоед.
hud-err-head = Сарлавҳаҳоро хондан нашуд ({ $reason }). Барои кӯшиши дубора ин лавҳаро пӯшида, боз кушоед.
hud-offline = Шумо офлайн ҳастед ва ин сатр аз сервери мо мепурсад. Вақте ки пайваст шудед, лавҳаро боз кушоед.
hud-offline-copy = Чен нашуд: ин нусхаи саҳифа аз кеши офлайни браузери шумо омадааст, на аз сервери мо.
hud-visitors-since = Аз { $date } инҷониб { $n } тамошои саҳифа чен шудааст
hud-visitors-p75 = персентили 75-ум аз { $n }
hud-visitors-few = То ҳол { $n } ченак — барои персентили 75-ум кам аст (бояд { $min } бошад)
hud-visitors-declined = Браузери шумо аз сомонаҳо хоҳиш мекунад, ки онро чен накунанд (Global Privacy Control ё Do Not Track), бинобар ин ин тамошои саҳифа ба ҳисоб гирифта намешавад.
hud-visitors-error = Ба /_rum расидан нашуд ({ $reason }). Барои кӯшиши дубора ин лавҳаро пӯшида, боз кушоед.
hud-ab-lead = Санҷиши A/B { $id }: сервери мо дар ҳар боркунии саҳифа як вариантро тасодуфан интихоб мекунад ва ҳисоб мекунад, ки чанд тамошо бо ҳодисаи { $metric } анҷом меёбад. Ҳадди тамошо барои ҳар вариант: { $max }.
hud-ab-control = назоратӣ
hud-ab-yours = ҳамин тамошо
hud-ab-views = { $n } аз { $max } тамошои саҳифа
hud-ab-rate = { $metric }: { $n } ({ $rate })
hud-ab-wait = То он даме ки ҳар вариант ҳамаи тамошоҳояшро ҷамъ накунад, хулоса нест: фарқе, ки барвақт дида мешавад, аксар вақт тасодуф аст.
hud-ab-better = Беҳтар аз варианти назоратӣ (p = { $p }).
hud-ab-worse = Бадтар аз варианти назоратӣ (p = { $p }).
hud-ab-same = Фарқи равшан аз варианти назоратӣ нест (p = { $p }).
hud-ab-done = Анҷом ёфт: ҳисоб қатъ шуд. Фарқ равшан аст, агар p аз { $alpha } камтар бошад: ин сатҳи умумии { $family } аст, ки байни { $k } муқоиса тақсим шудааст.
