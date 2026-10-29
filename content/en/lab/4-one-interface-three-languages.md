+++
slug = "one-interface-for-tajik-russian-and-english"
title = "Designing one interface for Tajik, Russian and English"
pillar = "design"
date = 2026-10-29
summary = "Our brand typeface has no Cyrillic, Tajik needs six letters Russian doesn't, and every language is written by people, not machines. So the site ships fonts split by script, fails its build when a character on any page has no glyph or a string is missing, marks every foreign word with its language, and exports plural forms for the browser to choose. The Russian and Tajik copy is being written by native speakers and is not live yet; this post is about the machinery that will carry it."
repo = "crates/site/src/fonts.rs"
+++

## The problem

Mintaka works from Dushanbe for clients in Tajikistan, across Central Asia and further away, so the site has three languages: Tajik, Russian and English. They use two scripts, and the two Cyrillic languages don't use the same letters. Tajik adds six letters to the Russian alphabet, each in lower and upper case: <span lang="tg">ғ ӣ қ ӯ ҳ ҷ</span>.

Our typeface for the brand, Space Grotesk, has no Cyrillic glyphs at all. A design that looks right in English and falls back to whatever the visitor's system has for Cyrillic would look like two different companies. A design that ships every glyph of every script to everyone would be slow. And text written in one language and machine-translated into the others would misrepresent us in exactly the languages our closest clients read.

Today only English is live. The Russian and Tajik copy is being written by native speakers, and a language is switched on only when all of its strings exist. What follows is the machinery that is already in place, and what we could and could not measure without that copy.

## Method

### One stack, split by script

The page has one sans-serif stack: Space Grotesk, then Onest, then a local font with adjusted metrics, then the system. Onest has every Tajik letter and the closest geometry to Space Grotesk of the fonts we tested. Browsers choose a font per character, so a Cyrillic word falls through to Onest without any markup. Code and measured numbers use IBM Plex Mono, which covers Latin, Cyrillic and the Tajik letters on its own.

Each font is split into files by script, and each file carries a `unicode-range`. A browser downloads a file only when a character on screen falls inside its range, and each language preloads the files its text will need: English the Latin file, Russian also Onest's Cyrillic file, Tajik also Onest's extended Cyrillic file. The Tajik letters are why that third file is needed: none of the twelve is in the basic Cyrillic file's ranges (mainly U+0400–045F, the Russian alphabet), and all of them are in the extended file's U+0460–052F.

### A build that fails on a missing glyph

[`fonts.rs`](repo:crates/site/src/fonts.rs) decodes each shipped WOFF2 file just far enough to read its character map, and intersects it with the file's `unicode-range`: a glyph that the CSS would never load from that file doesn't count. Then it reads the text of every generated page, splits it into sans and mono runs (code, keyboard keys and measured numbers are mono), and reports any character that no shipped font covers. A release build fails on the report. It checks the strings of the interactive parts too, and the Lab pages, this one included.

### Strings that cannot go missing

Page copy lives in `content/{lang}/page.ftl` and is written in Fluent. [`i18n.rs`](repo:crates/site/src/i18n.rs) renders it at build time; a message that is missing renders as a visible marker and fails the build, in every enabled language. The copy of the interactive parts is exported to the page as JSON. A message may select on one number, for plurals, and the generator exports every variant under its own key so the browser can pick one with `Intl.PluralRules` when it knows the count.

### Letters that belong to a language

The document says `lang="en"`, so a word in another language has to say so. The home page's story of our name contains the Tajik word <span lang="tg">минтақа</span> ("region"), and the generator wraps it in a `span` with `lang="tg"`; the Tajik and Russian words in this post are marked the same way. Screen readers switch voice on `lang`, and browsers use it for language-specific letter forms and hyphenation. The language switch, which appears as soon as a second language is enabled, names each language in itself (<span lang="ru">Русский</span>, <span lang="tg">Тоҷикӣ</span>) and gives each link both `lang` and `hreflang`.

### The t with a dot below

The name Mintaka comes from the Arabic for "the belt", and the careful transliteration writes one of its letters as t with a dot below, U+1E6D. Our first draft of the name story used that letter. It is in Space Grotesk, but not in its Latin file: it lives in the Latin Extended file, 18,940 bytes, which the browser then downloaded for one character on one line. A glyph check passes on it, because the glyph exists. The cost doesn't go away, so the copy now says al-mintaqa, and this post names the letter instead of showing it. The lesson we kept: the check guarantees every character has a glyph, but every rare character can still cost a font file.

## The mathematics

Plural rules are arithmetic on the number. English and Tajik have two forms: one for exactly 1, another for everything else. Russian has four. CLDR, the data browsers use, gives them for whole numbers $n$:

$$
\text{one}: \; n \bmod 10 = 1 \text{ and } n \bmod 100 \ne 11
$$

$$
\text{few}: \; 2 \le n \bmod 10 \le 4 \text{ and not } 12 \le n \bmod 100 \le 14
$$

Every other whole number is "many", and fractions are "other". So 21 takes the same form as 1, but 11 does not; 22 is "few", 12 is "many". This is why the generator does not choose plural forms at build time: the count is only known in the browser, which runs the rule for the page's language, and the generator's job is to export one string per category.

## The code

The font files, their scripts and the languages that preload them:

```rust include="crates/site/src/fonts.rs" from="const FACES: &[FaceDef] = &[" to="];"
```

The check that runs over every page's text:

```rust include="crates/site/src/fonts.rs" from="pub fn uncovered(&self, html: &str)"
```

A plural message from the security section, in Fluent. The generator turns it into `armour-summary[one]`, `armour-summary[other]` and a default:

```ftl include="content/en/islands/armour.ftl" from="armour-summary = " to="in place on this page"
```

And the end of every build, per language:

```rust include="crates/site/src/i18n.rs" from="pub fn finish(&self)"
```

## The measurements

Font files as shipped, measured on 2026-10-29 from `assets/fonts/`: the size on disk, and the code points in each file's character map as `fonts.rs` reads it.

| File | Bytes | Code points | Preloaded for |
|---|---:|---:|---|
| Space Grotesk, Latin | 22,288 | 230 | English, Russian, Tajik |
| Space Grotesk, Latin Extended | 18,940 | 333 | none |
| Onest, Cyrillic | 15,860 | 104 | Russian, Tajik |
| Onest, Cyrillic Extended | 10,540 | 73 | Tajik |
| Onest, mathematical symbols | 25,480 | 131 | none |
| IBM Plex Mono, Latin | 14,708 | 229 | none |
| IBM Plex Mono, Cyrillic | 8,356 | 108 | none |
| IBM Plex Mono, Cyrillic Extended | 6,912 | 75 | none |

Table: The shipped WOFF2 files. Both extended Cyrillic files contain all twelve Tajik letters; the build's tests assert it for Onest.

So the first view preloads 22,288 bytes of fonts in English and will preload 48,688 in Tajik, most of it the Cyrillic files a Tajik reader cannot do without.

The table also corrected our own contract. It said that the Greek lambda, missing from Space Grotesk, falls through to Onest's mathematical file. That file has 131 code points and no Greek letter at all, so a lambda renders in a system font. The first draft of this paragraph printed the letter itself, and the glyph check flagged this page; a release build would have refused it. We fixed the contract, and the formulas in this Lab write a rate as $r$.

The plural rules above are the ones Node 26.12.1 applies (ICU 78.3, CLDR 48.0), checked with `Intl.PluralRules`:

```sh title="The plural categories, as the browser's engine sees them"
node -e 'for (const l of ["en", "ru", "tg"]) {
  const r = new Intl.PluralRules(l);
  console.log(l, [0, 1, 2, 5, 11, 12, 21, 22, 111, 1.5].map((n) => n + ":" + r.select(n)).join(" "));
}'
```

| Number | English | Russian | Tajik |
|---|---|---|---|
| 1 | one | one | one |
| 2 | other | few | other |
| 5 | other | many | other |
| 11 | other | many | other |
| 21 | other | one | other |
| 22 | other | few | other |
| 111 | other | many | other |
| 1.5 | other | other | other |

Table: The plural category for each number, from Node 26.12.1 on 2026-10-29. 0 and 12 were "many" in Russian and "other" in both other languages.

### What we could not measure

Text expansion. We don't have the Russian and Tajik copy, so any percentage here would be invented. What we can say is where the layout gives and where it doesn't. Headings balance their lines and paragraphs avoid orphans (`text-wrap`), grids use `minmax(0, …)` so long words shrink a column instead of widening the page, and the header's navigation becomes a native menu below 64 rem. Buttons and badges never wrap (`white-space: nowrap`), so every button label is a line to check at 320 pixels when the copy arrives. When it does, we will measure it the way the rest of this site is measured, and say what broke.
