## S13 · Start a project (web/instruments/contact.ts): what the enhanced form says once someone
## uses it — a lazy bundle (/assets/strings/contact.<lang>.<hash>.json), fetched on the first focus
## and awaited on Send. Error messages are keyed by the server's error codes (docs/CONTRACTS.md
## §8.2): contact-error-<code>. Each says what happened and what to do.

contact-invalid = Ин майдонҳоро санҷед: { $fields }.
contact-contact = Почтаи электронӣ ё Telegram
contact-sent = Паёми шумо дар роҳ аст.
# TODO Bill to confirm: the reply time promised here (and in ct-lead, sent-body in page.ftl).
contact-sent-reply = Мо дар давоми як рӯзи корӣ ҷавоб медиҳем.
# Only when the server runs our Telegram bot: its answer then carries the link (docs/CONTRACTS.md §19).
# A button that never wraps: the short part stays on it, the rest goes into the note.
contact-bot = Идома дар Telegram
contact-bot-note = Бот фавран ҷавоб медиҳад: дар он ҷо метавонед дархостатонро пурра кунед ва ҷавобҳои даста ба ҳамон чат мерасанд. Пайванд 7 рӯз эътибор дорад.
contact-pow = Браузери шумо муаммои зидди спами моро дар { $ms } ({ $n } хеш) ҳал кард — бе ягон CAPTCHA.
contact-error-name-required = Номатонро нависед, то донем ба кӣ менависем.
contact-error-name-too-long = Номатонро то 120 аломат кӯтоҳ кунед.
contact-error-email-invalid = Чунин менамояд, ки ин суроғаи почта нопурра аст. Онро санҷед ё холӣ гузошта, номи корбарии Telegram-ро илова кунед.
contact-error-telegram-invalid = Номи корбарии Telegram аз 5 то 32 ҳарфи лотинӣ, рақам ё зерхат иборат аст ва бо ҳарф оғоз меёбад.
contact-error-contact-required = Почтаи электронӣ ё номи корбарии Telegram-ро илова кунед, то мо ҷавоб дода тавонем.
contact-error-message-too-long = Паёматонро то 5000 аломат кӯтоҳ кунед.
contact-error-company-too-long = Номи ширкатро то 120 аломат кӯтоҳ кунед.
contact-error-choice-invalid = Яке аз интихобҳо шинохта нашуд. Саҳифаро аз нав бор карда, боз интихоб кунед.
contact-error-pow-invalid = Санҷиши зидди спам нагузашт. Шаклро боз фиристед — браузери шумо онро аз нав иҷро мекунад.
contact-error-pow-expired = Мӯҳлати санҷиши зидди спам гузашт. Шаклро боз фиристед — браузери шумо онро аз нав иҷро мекунад.
contact-error-rate-limited = Дар як соати охир аз шабакаи шумо паёмҳои аз ҳад зиёд омад. Баъдтар кӯшиш кунед ё бевосита ба мо нависед.
contact-error-too-large = Шакл барои фиристодан хеле калон аст. Паёматонро кӯтоҳ карда, боз кӯшиш кунед.
contact-error-server-error = Дар тарафи мо чизе хато шуд ва паёми шумо фиристода нашуд. Пас аз як дақиқа боз кӯшиш кунед.
