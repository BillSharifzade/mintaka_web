+++
slug = "one-interface-for-tajik-russian-and-english"
title = "Designing one interface for Tajik, Russian and English"
pillar = "design"
date = 2026-10-29
summary = "Our brand typeface has no Cyrillic, and Tajik needs six letters Russian doesn't. So the site ships fonts split by script, fails its build when a character on any page has no glyph or a string is missing, marks Tajik and Russian words in English text with their language, and exports plural forms for the browser to choose. The Russian and Tajik versions were drafted by Claude, an AI model, at our founder's request, and native speakers have yet to review them. This post is about the machinery that carries them."
repo = "crates/site/src/fonts.rs"
+++

## The problem

Mintaka is a software studio in Dushanbe, so its site has three languages: Tajik, Russian and English. They use two scripts, and the two Cyrillic languages don't use the same letters. Tajik writes six letters that Russian doesn't, each in lower and upper case: <span lang="tg">ғ ӣ қ ӯ ҳ ҷ</span>.

Our typeface for the brand, Space Grotesk, has no Cyrillic glyphs at all. A design that looks right in English and falls back to whatever the visitor's system has for Cyrillic would look like two different companies. A design that ships every glyph of every script to everyone would be slow.

Since 2026-10-30 all three languages are switched on. The Russian and Tajik versions were drafted by Claude, an AI model, at our founder's request, and native speakers have yet to review them; a language is switched on only when all of its strings exist. What follows is the machinery, and what we could and could not measure once that copy arrived.

## Method

### One stack, split by script

The page has one sans-serif stack: Space Grotesk, then Onest, then a local font with adjusted metrics, then the system. Onest has every Tajik letter and the closest geometry to Space Grotesk of the fonts we tested. Browsers choose a font per character, so a Cyrillic word falls through to Onest without any markup. Code and measured numbers use IBM Plex Mono, which covers Latin, Cyrillic and the Tajik letters on its own.

Each font is split into files by script, and each file carries a `unicode-range`. A browser downloads a file only when a character on the page falls inside its range, and each language preloads the files its first screen is drawn in: English the Latin file, Russian also Onest's Cyrillic file, Tajik both of Onest's Cyrillic files (the Latin it needs there, only for "Rust" and "English", comes when asked for). The Tajik letters are why the extended file is needed: none of the twelve is in the basic Cyrillic file's ranges (mainly U+0400–045F, the Russian alphabet), and all of them are in the extended file's U+0460–052F.

### A build that fails on a missing glyph

[`fonts.rs`](repo:crates/site/src/fonts.rs) decodes each shipped WOFF2 file just far enough to read its character map, and intersects it with the file's `unicode-range`: a glyph that the CSS would never load from that file doesn't count. Then it reads the text of every generated page, splits it into sans and mono runs (code, keyboard keys and measured numbers are mono), and reports any character that no shipped font covers. A release build fails on the report. It checks the strings of the interactive parts too, and the Lab pages, this one included. What it cannot see is text that TypeScript puts together outside those strings.

### Strings that cannot go missing

Page copy lives in `content/{lang}/page.ftl` and is written in Fluent. [`i18n.rs`](repo:crates/site/src/i18n.rs) renders it at build time; a message that is missing renders as a visible marker and fails the build, in every enabled language. The copy of the interactive parts is exported to the page as JSON. A message may select on one number, for plurals, and the generator exports every variant under its own key so the browser can pick one with `Intl.PluralRules` when it knows the count.

### Letters that belong to a language

The document says `lang="en"`, so a word in another language has to say so. The home page's story of our name contains the Tajik word <span lang="tg">минтақа</span> ("region"), and the generator wraps it in a `span` with `lang="tg"`; the Tajik and Russian words in this post are marked the same way. Screen readers switch voice on `lang`, and browsers use it for language-specific letter forms and hyphenation. The language switch, which appears as soon as a second language is enabled, names each language in itself (<span lang="ru">Русский</span>, <span lang="tg">Тоҷикӣ</span>) and gives each link both `lang` and `hreflang`.

### The t with a dot below

The name Mintaka comes from the Arabic for "the belt", and the careful transliteration writes one of its letters as t with a dot below, U+1E6D. Our first draft of the name story used that letter. It is in Space Grotesk, but not in its Latin file: it lives in the Latin Extended file, 18,940 bytes, which the browser then downloaded for one character on one line. A glyph check passes on it, because the glyph exists. The cost doesn't go away, so the copy now says al-mintaqa, and this post names the letter instead of showing it. The lesson we kept: the check guarantees every character has a glyph, but every rare character can still cost a font file.

## The mathematics

Plural rules are arithmetic on the number. English has two forms: one for exactly 1, another for everything else. Russian has four. CLDR, the data browsers use, gives them for whole numbers $n$:

$$
\text{one}: \; n \bmod 10 = 1 \text{ and } n \bmod 100 \ne 11
$$

$$
\text{few}: \; 2 \le n \bmod 10 \le 4 \text{ and not } 12 \le n \bmod 100 \le 14
$$

Every other whole number is "many", and fractions are "other". So 21 takes the same form as 1, but 11 does not; 22 is "few", 12 is "many". This is why the generator does not choose plural forms at build time: the count is only known in the browser, which runs the rule for the page's language, and the generator's job is to export one string per category.

Tajik is the exception: the engines we checked have no plural rule for it at all (see the measurements below). That costs little, because in Tajik, as in Persian, a noun stays singular after a number. A Tajik message can make do with its default form, which the page uses for any category a message doesn't have.

## The code

The font files, their scripts and the languages that preload them:

```rust include="crates/site/src/fonts.rs" from="const FACES: &[FaceDef] = &[" to="];"
```

The check that runs over every page's text:

```rust include="crates/site/src/fonts.rs" from="pub fn uncovered(&self, html: &str)"
```

A plural message from the security section, in Fluent. The generator turns it into `armour-summary[one]` and a default, `armour-summary`; a category whose text is the default's (here `[other]`) isn't exported, since the browser falls back to the default anyway:

```ftl include="content/en/islands/armour.ftl" from="armour-summary = " to="in place on this page"
```

And the end of every build, per language:

```rust include="crates/site/src/i18n.rs" from="pub fn finish(&self)"
```

## The measurements

Font files as shipped, measured on 2026-10-29 from `assets/fonts/`: the size on disk, and the code points in each file's character map as `fonts.rs` reads it.

| File | Bytes | Code points | Preloaded for |
|---|---:|---:|---|
| Space Grotesk, Latin | 22,288 | 230 | English, Russian |
| Space Grotesk, Latin Extended | 18,940 | 333 | none |
| Onest, Cyrillic | 15,860 | 104 | Russian, Tajik |
| Onest, Cyrillic Extended | 10,540 | 73 | Tajik |
| Onest, mathematical symbols | 25,480 | 131 | none |
| IBM Plex Mono, Latin | 14,708 | 229 | none |
| IBM Plex Mono, Cyrillic | 8,356 | 108 | none |
| IBM Plex Mono, Cyrillic Extended | 6,912 | 75 | none |

Table: The shipped WOFF2 files. Both extended Cyrillic files contain all twelve Tajik letters; the build's tests assert it for Onest.

So the first view preloads 22,288 bytes of fonts in English, 38,148 in Russian and 26,400 in Tajik: a Tajik page takes only the two Onest files a Tajik reader cannot do without.

The table also corrected our own contract. It said that the Greek lambda, missing from Space Grotesk, falls through to Onest's mathematical file. That file has 131 code points and no Greek letter at all, so a lambda renders in a system font. The first draft of this paragraph printed the letter itself, and the glyph check flagged this page; a release build would have refused it. We fixed the contract, and the formulas in this Lab write a rate as $r$. One lambda still gets through: the overload lab on the home page builds its Little's law line in TypeScript, where the check cannot see it, so that letter is drawn by a system font.

The plural rules above are the ones Node 26.12.1 applies (ICU 78.3, CLDR 48.0), checked with `Intl.PluralRules`. The second word of each line is the language whose rules were actually used:

```sh title="The plural categories, as Node sees them"
node -e 'for (const l of ["en", "ru", "tg"]) {
  const r = new Intl.PluralRules(l);
  console.log(l, r.resolvedOptions().locale, [0, 1, 2, 5, 11, 12, 21, 22, 111, 1.5].map((n) => n + ":" + r.select(n)).join(" "));
}'
```

| Number | English | Russian |
|---|---|---|
| 1 | one | one |
| 2 | other | few |
| 5 | other | many |
| 11 | other | many |
| 21 | other | one |
| 22 | other | few |
| 111 | other | many |
| 1.5 | other | other |

Table: The plural category for each number, from Node 26.12.1. 0 and 12 were "many" in Russian and "other" in English.

For `tg`, the answer came from `en-US`. Node 26.12.1 and Chrome 156 have no plural rules for Tajik: `Intl.PluralRules.supportedLocalesOf(["tg"])` is empty in both, and `new Intl.PluralRules("tg")` quietly uses the default language instead, which is English on our machines (Node started with a Russian locale answers with Russian rules). Nothing warns about it: a Tajik column made this way looks plausible and is really English. So a Tajik page relies on the default form described above.

Text expansion, measured on 2026-10-30 when the drafts arrived. Over the 977 messages all three languages share, Russian runs 11 % more characters than English and Tajik 13 %. Bytes grow more, because a Cyrillic letter takes two bytes in UTF-8: both languages come to about twice the English (×1.96 and ×1.99), so the same page weighs more in Russian and Tajik even compressed. The layout gives where it was built to: headings balance their lines and paragraphs avoid orphans (`text-wrap`), grid columns use `minmax(0, …)` so that one long word cannot widen a column past its share, and buttons and badges never wrap (`white-space: nowrap`), which made every label a line to check. At 320 pixels and at desktop widths, two things broke, and both are fixed. The Russian section names didn't fit the full header between 1025 and about 1125 pixels, so Russian keeps the `details` menu up to 72 rem, where English and Tajik switch at 64 rem. And <span lang="ru">Конфиденциальность</span>, Russian for "privacy", made the privacy page wider than a 320-pixel screen; as a heading it now breaks inside the word.

### What we could not measure

Whether the language is good. The build proves that every string exists and every letter has a glyph, not that the Russian and Tajik read well; that is for the native speakers who review them.
