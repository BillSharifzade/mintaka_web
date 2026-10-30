# Translation — Tajik, Russian, English

**Status (2026-10-30).** The Russian and Tajik copy was drafted by Claude, an AI model, at the
founder's request on 2026-10-30. No native speaker has reviewed it yet. Spec §11 asks for copy
written natively by the team; that promise holds only once a native speaker has read every file
listed below and signed it off here. Until then, nothing on the site or in the repository may say
the translations are native.

| Language | Drafted | Reviewed by | Date |
|---|---|---|---|
| Russian (`ru`) | Claude, at the founder's request | *nobody yet* | |
| Tajik (`tg`) | Claude, at the founder's request | *nobody yet* | |

## Where the strings live

| Path (`{lang}` = `en`, `ru`, `tg`) | What it holds |
|---|---|
| `content/{lang}/page.ftl` | Everything rendered at build time: pages, curl mode, feeds, `/sent/`, 404, Telegram bot replies |
| `content/{lang}/islands/*.ftl` | Strings shipped inside each page, for islands that need them at once |
| `content/{lang}/islands/lazy/*.ftl` | Strings fetched on first use (`loadStrings('<name>')`): terminal, HUD panel, fuzzer, console help, contact form messages, trace hop notes, CTF |
| `content/{lang}/lab/*.md` | Lab posts. Same file name and `slug` in every language; code blocks, `include=` excerpts, `repo:` links, numbers and maths are identical |
| `content/site.toml` | `langs` (its order sets the language switch and hreflang lists) and the English-only values listed under *Deliberate deviations* |

English (`content/en/`) is the source. When you change an English string, change the other two in
the same commit, or the build will tell you.

## What the build checks

`cargo xtask build` compares every Russian and Tajik file with its English counterpart
(`check_translation` in `crates/site/src/i18n.rs`):

- **Fails** when a file or message is missing, when a message gains or loses a value or an
  attribute, when it uses a `$variable` English never passes, or when a variant key is neither a
  plural category (`zero one two few many other`) nor a number.
- **Warns** when a message is not in English (unused), or leaves out a variable English uses. The
  intended cases are listed under *Deliberate deviations*; any other is a mistake.
- The font check (`crates/site/src/fonts.rs`) fails the build if any page holds a character the
  self-hosted subsets can't draw. Onest Cyrillic Ext carries ғ ӣ қ ӯ ҳ ҷ; Tajik pages preload it.
- Budgets are per language: first screen ≤ 14 kB and whole home page ≤ 32 kB, brotli. Russian and
  Tajik weigh more than English (Cyrillic is two bytes a letter in UTF-8), so long additions show
  up there first.

## Glossary

Keep these the same everywhere: pages, islands, Lab posts, bot replies, alt text. **Bold** marks
the choices a native reviewer should confirm first.

| English | Russian | Tajik | Note |
|---|---|---|---|
| Mintaka (the company, the mark) | Mintaka | Mintaka | Latin in every language, like Rust or Telegram |
| Mintaka (the star, the pass) | Минтака | Минтақа | Tajik *минтақа* also means "region, zone" (spec, Appendix D) |
| Orion · Orion's belt | Орион · пояс Ориона | Орион · камарбанди Орион | **Tajik: or a traditional Persian name for the constellation?** |
| Betelgeuse | Бетельгейзе | Бетелгейзе | Tajik writes no ь |
| Rigel | Ригель | Ригел | |
| Saiph | Саиф | Сайф | Tajik follows Arabic *saif*, "sword" |
| Meissa | Мейсса | Мейсса | |
| Bellatrix | Беллатрикс | Беллатрикс | |
| Alnitak · Alnilam | Альнитак · Альнилам | Алнитак · Алнилам | |
| "the girdle" (Alnitak) | «кушак» | «миёнбанд» | Star epithets, Appendix D |
| "the string of pearls" (Alnilam) | «нить жемчуга» | «риштаи марворид» | |
| "the belt" (Mintaka; the belt in our mark) | «пояс» | «камарбанд» | |
| Efficiency | Эффективность | **Самаранокӣ** | |
| Security | Безопасность | **Амният** | |
| Highload | Высокие нагрузки (menu: Нагрузки) | **Сарбории баланд** (menu: Сарборӣ) | The header menu takes the short forms |
| Human-intuitive design | Интуитивный дизайн (menu: Дизайн) | **Тарроҳии фаҳмо** (menu: Тарроҳӣ) | |
| Proof Pack | **Пакет доказательств** | **Бастаи далелҳо** | Spec said "to decide"; these are proposals |
| proof console | консоль доказательств | консоли далелҳо | The button says only Открыть консоль / Кушодани консол |
| Proof wall | Стена доказательств | Девори далелҳо | |
| The Lab | Лаборатория | **Озмоишгоҳ** | |
| Overload lab (S6) | Полигон перегрузок | Майдони санҷиши изофабор | |
| Start a project (CTA) | **Обсудить проект** | **Оғози лоиҳа** | Russian chose the shorter, friendlier verb; literal: Начать проект |
| brief (the contact form's message) | бриф | **дархост** | Tajik *дархост* also means an HTTP request; see review notes |
| request (HTTP) | запрос | дархост | |
| software · hardware | софт · железо | нармафзор · сахтафзор | Russian keeps the engineers' register; formal: ПО · аппаратура |
| website | сайт | сомона | |
| the edge of physics | предел физики | марзи физика | |
| post-quantum | постквантовый | постквантӣ | |
| route | маршрут | масир | |
| latency | задержка | таъхир | |
| edge proxy (hosting) | пограничный прокси (short: прокси хостинга) | прокси-сервери канорӣ | |
| Privacy | Конфиденциальность | Махфият | |
| cookie | cookie | cookie | Kept in Latin, as the technical name |
| terminal | терминал | терминал | |

Never translated: product and protocol names (Rust, rustls, aws-lc-rs, Telegram, OpenStreetMap,
Atom, TLS, QUIC, HTTP/3, X25519MLKEM768, Contraction Hierarchies), code, commands, file names,
keys (`~`, `P`), console calls (`mintaka.ctf()`), and the RA/Dec labels of star coordinates.

## Conventions

**Typography, both languages.** Quotes are «ёлочки», with „лапки“ inside them. A dash between
words is an em dash with spaces, the first one a no-break space (U+00A0) so the dash never starts
a line. A number and its unit are joined by a no-break space (7495 м, 204 км). No-break spaces are
invisible in most editors; keep the ones already there when you edit a line.

**Numbers and units.** Russian and Tajik group thousands with a no-break space and use a decimal
comma: 17 578, 0,1 с. In hand-written prose a four-digit number stays whole (7495, 8393), as
Russian typesetting prefers. Units are SI symbols in Cyrillic: мкс, мс, с, Б, кБ, МБ, км.
Numbers that code writes follow CLDR, which groups from four digits (1 234 км). They go through
`Strings::int`, `Strings::dec` and `unit_symbol` in Rust (`crates/site/src/i18n.rs`), and
through `numberLocale`, `dateTime`, `list` and `unit` in TypeScript (`web/shared/i18n.ts`,
`web/shared/format.ts`). Keep the two unit tables in step.

**Browsers have no Tajik locale data.** Chrome and Node ship no CLDR data for `tg`: `Intl` would
quietly answer in the browser's own language. So Tajik pages borrow Russian *digits and
separators* only (`SAME_NUMBERS`), write dates in figures (30.10.2026, 12:40), join lists with
commas, and take plural categories from English (one, other). Words, units and plural forms
always stay Tajik.

**Plurals.** Russian messages carry `one`, `few`, `many` and `other` (CLDR gives `other` only to
fractions such as 1,5). A number key catches one exact value: `[1]` writes *один раз* instead of
*1 раз*. Tajik keeps a noun singular after a numeral (*5 сервер*, never *5 серверҳо*), so most
Tajik messages need no variant at all; don't define `few` or `many` in Tajik. Variants identical
to the default are dropped from the JSON the islands load.

**Tajik orthography.** Literary Tajik in Cyrillic with ғ ӣ қ ӯ ҳ ҷ, and no ц, щ, ы or ь. In loans
ц becomes тс, or с after к (симулятсия, функсия), and ь is dropped (консол, Бетелгейзе). Izofa
joins noun phrases (*камарбанди Орион*); the object marker -ро follows definite objects.

**Placeables and markup.** `{ $variable }`, `**bold**`, backticks and link syntax stay exactly as
in English. Reorder them freely; don't drop, rename or add them.

## Deliberate deviations

These Russian and Tajik messages leave out an English variable on purpose, because the value in
`content/site.toml` is an English phrase that would otherwise appear mid-sentence:

| Message | Leaves out | Written out as | Change by hand if |
|---|---|---|---|
| `ct-lead`, `sent-body` (page.ftl), `contact-sent-reply` (islands/lazy/contact.ftl), `bot-linked` (page.ftl) | `$within` | в течение одного рабочего дня · дар давоми як рӯзи корӣ | `contact.reply_within` changes |
| `pv-hosting` (page.ftl) | `$city`, `$country` | Душанбе, Таджикистан · Душанбе, Тоҷикистон | the server moves |

The lasting fix is to move those values out of `site.toml` into the FTL files.

## Still English on Russian and Tajik pages

- **curl mode**: tokens the server fills in at request time: `{{tls}}` ("post-quantum hybrid",
  "plain HTTP (development)") and the µs in `{{server_time}}`; and the labels of the contact page
  (email, telegram, phone, web), written in `crates/site/src/text.rs`.
- **Physics table (S2)**: the visitor's and the server's city come from the server's location data
  in Latin script (Dushanbe).
- Star coordinates keep the international RA/Dec labels.

## Notes for the native reviewer

Read in this order: what most visitors see first, then what costs most if it's wrong.

1. **First screen**: `hero-*`, `nav-*`, `meta-title`, `meta-description` in page.ftl. The A/B test
   swaps `hero-title` for `hero-title-hardware` or `hero-title-cycle`: keep the three about as
   long as each other in each language, or the test measures the line count instead of the words.
   The Russian hero says *софт*, a deliberate engineers' word: confirm it, or choose *ПО* or
   *программы*.
   Tajik *Нармафзор дар марзи физика*: does *марз* carry "edge" the way the English does, or is
   *ҳадд* better?
2. **Privacy page** (`pv-*`): it makes legal and factual statements. Check meaning before style.
3. **Contact form, `/sent/`, Telegram bot** (`ct-*`, `sent-*`, `bot-*`, `contact-*`): the promises
   to clients.
4. **Section titles**: Tajik `ds-title` is long and literal; `hl-title` borrows *супернова*. Is
   there a natural Tajik word or turn of phrase for both?
5. **Terms marked bold in the glossary**, especially:
   - Tajik *дархост* means both the client's brief and an HTTP request. On the privacy page and
     in the HUD the two sit close together. Consider *тавсифи лоиҳа* or *бриф* for the brief.
   - The star names: we kept the international forms. If a traditional Persian-Tajik name reads
     better for a Tajik audience (for the constellation, or for Betelgeuse and Rigel), change it in
     every file at once.
   - Russian *Пакет доказательств* and *Обсудить проект*.
6. **Lab posts** (`content/{ru,tg}/lab/`): technical prose; the Tajik terminology is the least
   settled part of the whole draft. Post 4 says in every language that the Russian and Tajik copy
   was drafted by Claude at the founder's request. Once you have reviewed it, update that
   sentence, this file's status table and the comments at the top of `content/site.toml`,
   `content/ru/page.ftl` and `content/tg/page.ftl`.

**Text expansion.** Over the 977 messages all three languages share (placeables left out, a
select counted by its default variant), Russian runs 11.1 % more characters than English (3.8 %
fewer words), Tajik 12.9 % more characters (4.8 % more words). UTF-8 doubles the bytes (×1.96 and
×1.99). Buttons, the menu and the header were checked at 320 px, 360 px
and 1025–1280 px with the current copy. The Russian section names don't fit the full desktop
header below about 1125 px, so Russian keeps the menu up to 72rem (1152 px;
`web/styles/sections/layout.css`). A longer label anywhere in the header or on a button can break
the 320 px layout: check it there. Below about 358 px the Russian privacy title
«Конфиденциальность» is wider than the screen and breaks without a hyphen; a soft hyphen (U+00AD)
after «Конфиденциаль» in `pv-title` would hyphenate it instead (it also lands in the page's
`<title>`, invisibly).
