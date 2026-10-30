# S5 · Attack our code (web/instruments/fuzzer): what the fuzzer says once the visitor starts it or
# opens "Try to crash it yourself" — a lazy bundle (/assets/strings/fuzzer.<lang>.<hash>.json),
# fetched at that moment. What the instrument shows before is inline in ../fuzzer.ftl. Coverage
# points and parse errors are numbered as in crates/fuzzer/src/points.rs and target.rs: keep the ids.

## Log

fuzzer-log-start = фаззинг { $build } · { $mode } · тухм { $seed }
fuzzer-log-new = роҳи нав
fuzzer-log-crash = ФУРӮПОШӢ
fuzzer-log-done = бас шуд · { $inputs } вуруд · { $paths } роҳ
fuzzer-build-buggy = v1 (бо хато)
fuzzer-build-fixed = v2 (ислоҳшуда)
fuzzer-mode-guided = бо роҳнамоии фарогирӣ
fuzzer-mode-blind = тасодуфӣ, кӯр-кӯрона

## Result cards

fuzzer-crash-title = Фурӯпошӣ пас аз { $inputs } вуруд ёфт шуд · { $time }
fuzzer-crash-panic = Паёми panic-и худи Rust, ки дар браузери шумо гирифта шуд:
fuzzer-crash-min = Аз { $from } байт то { $to } байти муҳим кӯтоҳ карда шуд:
fuzzer-crash-why = Байти дарозии бахши ном байтҳои бештаре ваъда дод, назар ба он ки дар харита ҳаст. Сатри { $line } ба он бовар кард.
fuzzer-crash-safe = Дар Rust ин қатъи тоза ва бехатар аст. Дар C ҳамин сатр метавонист аз охири буфер берун хонад. Дар ҳар сурат, ин ҳеҷ гоҳ ба релиз намерасад.
fuzzer-crash-fix = Ислоҳ, як сатр:
fuzzer-fixed-title = Сохти ислоҳшуда: { $inputs } вуруд, 0 фурӯпошӣ
fuzzer-fixed-body = Ба { $paths } аз { $total } роҳи код расид. Ягона роҳи торикмонда харитаест, ки маҳз MINTAKA ном дорад: ҳафт байти дақиқ бе ягон шохаи байнашон барои алоқаи баръакси фарогирӣ қариб ноаёнанд. Фаззерҳои саноатӣ барои ин луғатҳо ва пайгирии муқоисаҳоро илова мекунанд.
fuzzer-blind-title = Кӯр-кӯрона: { $paths } аз { $total } роҳ дар { $inputs } вуруд
fuzzer-blind-body = Байтҳои тасодуфӣ, бе алоқаи баръакс, { $time } аз протсессори шумо. Бо роҳнамоии фарогирӣ худи ҳамин муҳаррик хаторо пас аз { $guided } вуруд ёфт. Тамоми ҳила дар алоқаи баръакс аст.
fuzzer-blind-body-no-guided = Байтҳои тасодуфӣ, бе алоқаи баръакс, { $time } аз протсессори шумо. Бо роҳнамоии фарогирӣ худи ҳамин муҳаррик хаторо дар як лаҳза меёбад.

## Try it yourself (built when the panel first opens)

fuzzer-try-lead = Харитаро ҳамчун матн бо рамзҳои \xNN ё ҳамчун ҷуфтҳои рақамҳои шонздаҳӣ нависед. То 64 байт.
fuzzer-try-format = Формат: «ORI» · версия (1–2) · шумораи ситораҳо (1–7) · 4 байт барои ҳар ситора (x, y, бузургии ситоравӣ, парчамҳо) · баъд, дар версияи 2, бахшҳо: C (хатҳо) ва N (ном: байти дарозӣ, баъд худи ном).
fuzzer-try-label = Вуруди шумо
fuzzer-try-run = Иҷро кунед
fuzzer-try-build = Бар зидди
fuzzer-try-example-valid = Харитаи дуруст
fuzzer-try-example-betelgeuse = Харита бо Бетелгейзе
fuzzer-try-ok = Бе хато таҳлил шуд · ба { $paths } роҳи код расид.
fuzzer-try-rejected = Бехатар рад шуд: { $reason } · ба { $paths } роҳи код расид.
fuzzer-try-invalid = Ин рамзи дуруст нест. \x ва ду рақами шонздаҳиро истифода баред.
fuzzer-try-crashed = Шумо онро аз кор баровардед. Шумо мисли ҳамлагар фикр мекунед — мо мехоҳем бо шумо шинос шавем.
fuzzer-try-careers = Ҷойҳои холии корӣ

## Screen-reader announcements

fuzzer-announce-start = Фаззер оғоз шуд.
fuzzer-announce-paths = { $paths } аз { $total } роҳи код ёфт шуд.
fuzzer-announce-crash = Фурӯпошӣ пас аз { $inputs } вуруд ёфт шуд.
fuzzer-announce-done = Иҷро анҷом ёфт: { $inputs } вуруд, { $paths } роҳ, { $n } фурӯпошӣ.

## Why a chart was rejected (parse errors in crates/fuzzer/src/target.rs)

fuzzer-reason-bad-magic = бо «ORI» оғоз намешавад
fuzzer-reason-bad-version = версияи номаълум
fuzzer-reason-no-stars = дар харита ақаллан як ситора бояд бошад
fuzzer-reason-too-many-stars = Орион ҳафт ситора дорад, на бештар
fuzzer-reason-truncated = харита нопурра аст
fuzzer-reason-reserved-flags = битҳои захиравии парчамҳо гузошта шудаанд
fuzzer-reason-unknown-section = бахши номаълум
fuzzer-reason-bad-line = хат ба ситорае ишора мекунад, ки вуҷуд надорад

## Coverage points, by id (crates/fuzzer/src/points.rs). Keep the ids.

fuzzer-point-0 = аломати «O»
fuzzer-point-1 = аломати «OR»
fuzzer-point-2 = аломати «ORI»
fuzzer-point-3 = версияи 1
fuzzer-point-4 = версияи 2
fuzzer-point-5 = версияи номаълум
fuzzer-point-6 = харита бе ситора
fuzzer-point-7 = зиёда аз 7 ситора
fuzzer-point-8 = шумораи ситораҳо қабул шуд
fuzzer-point-9 = сабтҳои ситораҳо нопурраанд
fuzzer-point-10 = ситораи аввал хонда шуд
fuzzer-point-11 = ситораи дуюм
fuzzer-point-12 = ситораи сеюм — ин аллакай камарбанд аст
fuzzer-point-13 = ситораи ҳафтум — тамоми Орион
fuzzer-point-14 = дурахшонтар аз бузургии 0
fuzzer-point-15 = бузургии якум
fuzzer-point-16 = бузургии дуюм
fuzzer-point-17 = ситораи хира
fuzzer-point-18 = чоряки шимолу шарқӣ
fuzzer-point-19 = чоряки шимолу ғарбӣ
fuzzer-point-20 = чоряки ҷанубу шарқӣ
fuzzer-point-21 = чоряки ҷанубу ғарбӣ
fuzzer-point-22 = ситораи тағйирёбанда
fuzzer-point-23 = ситораи бисёркарата
fuzzer-point-24 = гигант
fuzzer-point-25 = супергигант
fuzzer-point-26 = битҳои захиравии парчамҳо
fuzzer-point-27 = Бетелгейзе: супергиганти дурахшони тағйирёбанда
fuzzer-point-28 = Минтақа: гиганти бисёркарата
fuzzer-point-29 = анҷоми тозаи харита
fuzzer-point-30 = бахши номаълум
fuzzer-point-31 = бахши хатҳо
fuzzer-point-32 = хатҳо санҷида шуданд
fuzzer-point-33 = хат ба ситораи ғоиб
fuzzer-point-34 = бахши ном
fuzzer-point-35 = дарозии ном хонда шуд
fuzzer-point-36 = ном хонда шуд
fuzzer-point-37 = ном бо ҳарфҳои калон
fuzzer-point-38 = харита бо номи MINTAKA
