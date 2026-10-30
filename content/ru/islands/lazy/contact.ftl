## S13 · Start a project (web/instruments/contact.ts): what the enhanced form says once someone
## uses it — a lazy bundle (/assets/strings/contact.<lang>.<hash>.json), fetched on the first focus
## and awaited on Send. Error messages are keyed by the server's error codes (docs/CONTRACTS.md
## §8.2): contact-error-<code>. Each says what happened and what to do.

contact-invalid = Проверьте поля: { $fields }.
contact-contact = Email или Telegram
contact-sent = Ваше сообщение в пути.
# TODO Bill to confirm: the reply time promised here (and in ct-lead, sent-body in page.ftl).
contact-sent-reply = Мы отвечаем в течение одного рабочего дня.
# Only when the server runs our Telegram bot: its answer then carries the link (docs/CONTRACTS.md §19).
# A button that never wraps: the short part stays on it, the rest goes into the note.
contact-bot = Продолжить в Telegram
contact-bot-note = Бот ответит сразу: там можно дополнить бриф, а ответы команды придут в тот же чат. Ссылка действует 7 дней.
contact-pow = Ваш браузер решил нашу антиспам-задачу за { $ms } ({ $n ->
    [one] { $n } хеш
    [few] { $n } хеша
    [many] { $n } хешей
   *[other] { $n } хеша
}) — и никакой капчи.
contact-error-name-required = Укажите имя, чтобы мы знали, кому пишем.
contact-error-name-too-long = Сократите имя до 120 символов.
contact-error-email-invalid = Похоже, адрес почты неполный. Проверьте его или оставьте поле пустым и укажите имя пользователя в Telegram.
contact-error-telegram-invalid = Имя пользователя в Telegram — от 5 до 32 латинских букв, цифр или подчёркиваний, и начинается оно с буквы.
contact-error-contact-required = Укажите email или имя пользователя в Telegram, чтобы мы могли ответить.
contact-error-message-too-long = Сократите сообщение до 5000 символов.
contact-error-company-too-long = Сократите название компании до 120 символов.
contact-error-choice-invalid = Один из вариантов не распознан. Перезагрузите страницу и выберите снова.
contact-error-pow-invalid = Проверка от спама не прошла. Отправьте форму ещё раз — браузер выполнит её заново.
contact-error-pow-expired = Срок проверки от спама истёк. Отправьте форму ещё раз — браузер выполнит её заново.
contact-error-rate-limited = За последний час из вашей сети пришло слишком много сообщений. Попробуйте позже или напишите нам напрямую.
contact-error-too-large = Форма слишком большая для отправки. Сократите сообщение и попробуйте снова.
contact-error-server-error = У нас что-то пошло не так, и сообщение не отправилось. Попробуйте через минуту.
