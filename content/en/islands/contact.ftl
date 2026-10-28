## S13 · Start a project (web/instruments/contact.ts) — the enhanced contact form.
## Error messages are keyed by the server's error codes (docs/CONTRACTS.md §8.2):
## contact-error-<code>. Each says what happened and what to do.

contact-sending = Sending…
contact-waiting = Finishing a small anti-spam check…
contact-invalid = Check these fields: { $fields }.
contact-contact = Email or Telegram
contact-sent = Your message is on its way.
# TODO Bill to confirm: the reply time promised here.
contact-sent-reply = We reply within one working day.
contact-pow = Your browser solved our anti-spam puzzle in { $ms } ({ $n ->
    [one] { $n } hash
   *[other] { $n } hashes
}) — no CAPTCHA to fill in.

contact-error-name-required = Add your name, so we know who we're writing to.
contact-error-name-too-long = Shorten your name to 120 characters or fewer.
contact-error-email-invalid = This email address looks incomplete. Check it, or leave it empty and add a Telegram handle.
contact-error-telegram-invalid = A Telegram username has 5 to 32 letters, digits or underscores and starts with a letter.
contact-error-contact-required = Add an email address or a Telegram handle, so we can reply.
contact-error-message-too-long = Shorten your message to 5,000 characters or fewer.
contact-error-company-too-long = Shorten the company name to 120 characters or fewer.
contact-error-choice-invalid = One of the options wasn't recognised. Reload the page and choose again.
contact-error-pow-invalid = The anti-spam check didn't pass. Send the form again and your browser will redo it.
contact-error-pow-expired = The anti-spam check expired. Send the form again and your browser will redo it.
contact-error-rate-limited = Too many messages came from your network in the last hour. Try again later, or write to us directly.
contact-error-too-large = The form is too large to send. Shorten your message and try again.
contact-error-server-error = Something went wrong on our side and your message wasn't sent. Try again in a minute.
contact-error-network = Your message wasn't sent: we couldn't reach the server. Check your connection and send again.
