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
/// same in every language that has it).
pub fn pages(ctx: &Ctx, all: &BTreeMap<String, Vec<Post>>, css: &str) -> Vec<(String, String)> {
    if ctx.lab.is_empty() {
        return vec![];
    }
    let alternates = |slug: Option<&str>| -> Vec<(String, String)> {
        let has = |l: &String| all.get(l).is_some_and(|ps| slug.map_or(!ps.is_empty(), |slug| ps.iter().any(|p| p.slug == slug)));
        let path = |l: &String| slug.map_or_else(|| index_path(l), |slug| format!("/{l}/lab/{slug}/"));
        ctx.cfg.langs.iter().filter(|l| has(l)).map(|l| (l.clone(), path(l))).collect()
    };
    let s = ctx.s;
    let mut out = vec![];
    let path = index_path(ctx.lang);
    let long_form = Some(LongForm { css, og_type: "website", head: html! {}, alternates: alternates(None) });
    let page = Page { path: &path, title: format!("{} — Mintaka", s.t("lab-title")), description: s.t("lab-description"), is_home: false, body: index(ctx), index: true, long_form };
    out.push((path.clone(), document(ctx, page).into_string()));
    for p in ctx.lab {
        let path = p.path(ctx.lang);
        let long_form = Some(LongForm { css, og_type: "article", head: post_head(ctx, p), alternates: alternates(Some(&p.slug)) });
        let page = Page { path: &path, title: format!("{} — Mintaka", p.title), description: p.summary.clone(), is_home: false, body: post(ctx, p), index: true, long_form };
        out.push((path.clone(), document(ctx, page).into_string()));
    }
    out
}

/// `/{lang}/lab/`: every post, newest first.
pub fn index(ctx: &Ctx) -> Markup {
    let s = ctx.s;
    html! {
        section class="sec lab-index" aria-labelledby="lab-title" {
            div class="wrap" {
                header class="lab-index__head" {
                    p class="eyebrow" { (s.t("lab-eyebrow")) }
                    h1 class="sec__title" id="lab-title" { (s.t("lab-title")) }
                    p class="lead" { (s.t("lab-lead")) }
                    p class="lab-index__feed" { a href=(feed_path(ctx.lang)) type="application/atom+xml" { (s.t("lab-feed-follow")) } }
                }
                ol class="lab-list" role="list" {
                    @for post in ctx.lab {
                        li class="lab-item" {
                            p class="lab-item__meta" {
                                span class="lab-item__pillar" data-star=(post.pillar.star()) { (s.t(&post.pillar.label_key())) }
                                span aria-hidden="true" { " · " }
                                time datetime=(post.date) { (post.date) }
                            }
                            h2 class="lab-item__title" { a href=(post.path(ctx.lang)) { (post.title) } }
                            p class="lab-item__summary" { (post.summary) }
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
                    a href=(index_path(ctx.lang)) { (s.t("lab-title")) }
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
                    a href=(index_path(ctx.lang)) { (s.t("lab-all")) }
                    a href=(feed_path(ctx.lang)) type="application/atom+xml" { (s.t("lab-feed")) }
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
