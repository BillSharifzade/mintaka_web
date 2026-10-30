//! The Lab's pages (spec §S11): the index and one page per post. Both are long-form pages — no
//! JavaScript, and the long-form stylesheet instead of the page CSS (see [`super::LongForm`]).
//! Post template: summary · the problem · method · mathematics · code · benchmark · a link to the
//! repository that reproduces it. The middle five are the post's own `##` sections.

use super::{Ctx, LongForm, Page, document, svg};
use crate::lab::{Post, feed_path, index_path};
use maud::{Markup, PreEscaped, html};
use std::collections::BTreeMap;

/// Every Lab page of `ctx.lang` as `(path, html)`: the index, then each post. `all` holds the
/// posts of every language, for the language switch and `hreflang` links (a post's slug is the
/// same in every language that has it). A language without posts of its own gets the index only,
/// listing the default language's posts in that language ([`Ctx::lab_fallback`]).
pub fn pages(ctx: &Ctx, all: &BTreeMap<String, Vec<Post>>, css: &str) -> Vec<(String, String)> {
    let Some((posts, posts_lang)) = listed(ctx) else { return vec![] };
    let default_has_posts = all.get(&ctx.cfg.default_lang).is_some_and(|ps| !ps.is_empty());
    let alternates = |slug: Option<&str>| -> Vec<(String, String)> {
        let has = |l: &String| match slug {
            Some(slug) => all.get(l).is_some_and(|ps| ps.iter().any(|p| p.slug == slug)),
            // Every language has an index while the default language has posts to fall back on.
            None => default_has_posts || all.get(l).is_some_and(|ps| !ps.is_empty()),
        };
        let path = |l: &String| slug.map_or_else(|| index_path(l), |slug| format!("/{l}/lab/{slug}/"));
        ctx.cfg.langs.iter().filter(|l| has(l)).map(|l| (l.clone(), path(l))).collect()
    };
    let s = ctx.s;
    let mut out = vec![];
    let path = index_path(ctx.lang);
    let long_form = Some(LongForm { css, og_type: "website", head: html! {}, alternates: alternates(None) });
    let body = index(ctx, posts, posts_lang);
    let page = Page { path: &path, title: format!("{} — Mintaka", s.t("lab-title")), description: s.t("lab-description"), is_home: false, body, index: true, long_form };
    out.push((path.clone(), document(ctx, page).into_string()));
    for p in ctx.lab {
        let path = p.path(ctx.lang);
        let long_form = Some(LongForm { css, og_type: "article", head: post_head(ctx, p), alternates: alternates(Some(&p.slug)) });
        let page = Page { path: &path, title: format!("{} — Mintaka", p.title), description: p.summary.clone(), is_home: false, body: post(ctx, p), index: true, long_form };
        out.push((path.clone(), document(ctx, page).into_string()));
    }
    out
}

/// The posts the Lab lists for `ctx.lang` and their language: its own, else the default
/// language's (`None` when neither has any).
pub fn listed<'a>(ctx: &Ctx<'a>) -> Option<(&'a [Post], &'a str)> {
    if !ctx.lab.is_empty() { Some((ctx.lab, ctx.lang)) } else { ctx.lab_fallback.map(|(lang, posts)| (posts, lang)) }
}

/// `/{lang}/lab/`: every post, newest first. Posts in another language (`posts_lang`) say so
/// first, and their titles and summaries carry that language.
pub fn index(ctx: &Ctx, posts: &[Post], posts_lang: &str) -> Markup {
    let s = ctx.s;
    let other = (posts_lang != ctx.lang).then_some(posts_lang);
    html! {
        section class="sec lab-index" aria-labelledby="lab-title" {
            div class="wrap" {
                header class="lab-index__head" {
                    p class="eyebrow" { (s.t("lab-eyebrow")) }
                    h1 class="sec__title" id="lab-title" { (s.t("lab-title")) }
                    p class="lead" { (s.t("lab-lead")) }
                    @if other.is_some() { p class="lab-index__note" { (s.t("lab-untranslated")) } }
                    p class="lab-index__feed" { a href=(ctx.url(&feed_path(posts_lang))) type="application/atom+xml" hreflang=[other] { (s.t("lab-feed-follow")) } }
                }
                ol class="lab-list" role="list" {
                    @for post in posts {
                        li class="lab-item" {
                            p class="lab-item__meta" {
                                span class="lab-item__pillar" data-star=(post.pillar.star()) { (s.t(&post.pillar.label_key())) }
                                span aria-hidden="true" { " · " }
                                time datetime=(post.date) { (post.date) }
                            }
                            h2 class="lab-item__title" lang=[other] { a href=(ctx.url(&post.path(posts_lang))) hreflang=[other] { (post.title) } }
                            p class="lab-item__summary" lang=[other] { (post.summary) }
                        }
                    }
                }
            }
        }
    }
}

/// `/{lang}/lab/{slug}/`
pub fn post(ctx: &Ctx, post: &Post) -> Markup {
    let s = ctx.s;
    html! {
        article class="post" aria-labelledby="post-title" {
            header class="post__head wrap" {
                p class="post__kicker" {
                    a href=(ctx.url(&index_path(ctx.lang))) { (s.t("lab-title")) }
                    span aria-hidden="true" { " · " }
                    span class="post__pillar" { (svg::star_mark(post.pillar.star())) (s.t(&post.pillar.label_key())) }
                }
                h1 class="post__title" id="post-title" { (post.title) }
                div class="post__summary" {
                    p class="post__label" { (s.t("lab-summary")) }
                    p class="post__lead" { (post.summary) }
                }
                p class="post__byline" {
                    span { (s.t("lab-byline")) }
                    span aria-hidden="true" { " · " }
                    time datetime=(post.date) { (post.date) }
                    @if post.updated != post.date {
                        span aria-hidden="true" { " · " }
                        (s.t("lab-updated")) " " time datetime=(post.updated) { (post.updated) }
                    }
                    span aria-hidden="true" { " · " }
                    a href=(post.repo_url) { (s.t("lab-code-for")) " " span class="mono" { (post.repo) } }
                }
            }
            div class="post__body wrap" { (PreEscaped(&post.html)) }
            footer class="post__foot wrap" {
                section class="post__repo" aria-labelledby="repo-title" {
                    h2 class="post__repo-title" id="repo-title" { (s.t("lab-reproduce")) }
                    p { (s.t("lab-reproduce-text")) }
                    p { a class="post__repo-link mono" href=(post.repo_url) { (post.repo_url.trim_start_matches("https://")) } }
                }
                nav class="post__nav" aria-label=(s.t("lab-title")) {
                    a href=(ctx.url(&index_path(ctx.lang))) { (s.t("lab-all")) }
                    a href=(ctx.url(&feed_path(ctx.lang))) type="application/atom+xml" { (s.t("lab-feed")) }
                }
            }
        }
    }
}

/// Open Graph article metadata for a post's `<head>`.
pub fn post_head(ctx: &Ctx, post: &Post) -> Markup {
    html! {
        meta property="article:published_time" content=(format!("{}T00:00:00Z", post.date));
        meta property="article:modified_time" content=(format!("{}T00:00:00Z", post.updated));
        meta property="article:section" content=(ctx.s.t(&post.pillar.label_key()));
    }
}
