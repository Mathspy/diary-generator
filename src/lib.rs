mod config;
pub mod katex;
mod months;
mod syndication;

use crate::config::Config;
use crate::syndication::atom;
use anyhow::{bail, Context, Result};
use either::Either;
use futures_util::stream::{FuturesUnordered, StreamExt, TryStreamExt};
use itertools::Itertools;
use maud::{html, Markup, PreEscaped, Render, DOCTYPE};
use notion_generator::{
    download::Downloadables,
    options::HeadingAnchors,
    render::{Heading, Title},
    response::{
        properties::{DateProperty, RichTextProperty, TitleProperty},
        NotionDate, NotionId, Page, PlainText, RichText,
    },
    HtmlRenderer,
};
use reqwest::Client;
use serde::Deserialize;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    io,
    ops::{Bound, Not},
    path::{Path, PathBuf},
};
use time::{
    format_description::{well_known::Rfc3339, FormatItem},
    macros::format_description,
    Date, Month, OffsetDateTime,
};
use tokio::task::JoinHandle;
use tokio_stream::wrappers::ReadDirStream;
use tracing::{info, warn};

pub const EXPORT_DIR: &str = "output";
pub const DIARY_GENERATOR: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");

#[derive(Deserialize)]
pub struct Properties {
    pub name: TitleProperty,
    pub date: DateProperty,
    pub url: RichTextProperty,
    pub description: RichTextProperty,
    pub published: DateProperty,
}

impl Title for Properties {
    fn title(&self) -> &[RichText] {
        self.name.title.as_slice()
    }
}

fn render_article_time(date: Date) -> Result<Markup> {
    const HTML_FORMAT: &[FormatItem<'_>] = format_description!("[year]-[month]-[day]");
    const READABLE_DATE: &[FormatItem<'_>] = format_description!("[month repr:long] [day], [year]");

    Ok(html! {
        p {
            time datetime=(date.format(HTML_FORMAT)?) {
                (date.format(READABLE_DATE)?)
            }
        }
    })
}

fn get_date(date: &NotionDate) -> Date {
    match date.start.parsed {
        Either::Left(date) => date,
        Either::Right(datetime) => datetime.date(),
    }
}

fn render_paging_links(
    renderer: &HtmlRenderer,
    current_date: Date,
    prev_page: Option<(&Date, &Page<Properties>)>,
    next_page: Option<(&Date, &Page<Properties>)>,
) -> Result<Markup> {
    if next_page.is_none() && prev_page.is_none() {
        return Ok(PreEscaped(String::new()));
    }

    Ok(html! {
        nav class="paging-links" {
            @if let Some((&prev_date, prev_page)) = prev_page {
                a href=(format_day(prev_date, true)) {
                    article {
                        p {
                            @if prev_date.next_day() == Some(current_date) {
                                "Yesterday:"
                            } @else {
                                "Previously:"
                            }
                        }
                        header {
                            h3 { (renderer.render_rich_text(&prev_page.properties.name.title)) }
                            (render_article_time(prev_date)?)
                        }
                    }
                }
            }

            @if let Some((&next_date, next_page)) = next_page {
                a href=(format_day(next_date, true)) {
                    article {
                        p {
                            @if next_date.previous_day() == Some(current_date) {
                                "Tomorrow:"
                            } @else {
                                "Next up:"
                            }
                        }
                        header {
                            h3 { (renderer.render_rich_text(&next_page.properties.name.title)) }
                            (render_article_time(next_date)?)
                        }
                    }
                }
            }
        }
    })
}

#[inline]
fn format_year(year: i32) -> String {
    format!("{:0>4}", year)
}

#[inline]
fn format_month(year: i32, month: Month) -> String {
    format!("{:0>4}/{:0>2}", year, u8::from(month))
}

#[inline]
fn format_day(date: Date, is_link: bool) -> String {
    format!(
        "{}{:0>4}/{:0>2}/{:0>2}",
        if is_link { "/" } else { "" },
        date.year(),
        u8::from(date.month()),
        date.day()
    )
}

async fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> Result<()> {
    let path = path.as_ref();
    info!(msg = "Writing file", path = %path.display());
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("Failed to create parent directory {}", path.display()))?;
    }
    tokio::fs::write(path, contents.as_ref())
        .await
        .with_context(|| format!("Failed to write {} file", path.display()))?;
    Ok(())
}

async fn read_partial_file<P: AsRef<Path>>(file: P) -> Result<String> {
    tokio::fs::read_to_string(Path::new("partials").join(file.as_ref()))
        .await
        .or_else(|error| match error.kind() {
            io::ErrorKind::NotFound => Ok(String::new()),
            _ => Err(error),
        })
        .with_context(|| format!("Failed to read partial file {}", file.as_ref().display()))
}

pub struct Generator {
    link_map: HashMap<NotionId, String>,
    lookup_tree: BTreeMap<Date, Page<Properties>>,
    article_pages: Vec<(String, Page<Properties>)>,
    downloadables: Downloadables,
    head: Markup,
    header: Markup,
    footer: Markup,
    config: Config,
    directory: PathBuf,
}

impl Generator {
    pub async fn new<P: AsRef<Path>>(dir: P, pages: Vec<Page<Properties>>) -> Result<Generator> {
        let dir = dir.as_ref();
        let length = pages.len();

        let today = time::OffsetDateTime::now_utc().date();

        let (link_map, lookup_tree, article_pages) = pages
            .into_iter()
            .filter(|page| {
                page.properties
                    .published
                    .date
                    .as_ref()
                    .map(|date| date.start <= today)
                    .unwrap_or(false)
            })
            .map(|page| {
                let date = page
                    .properties
                    .date
                    .date
                    .as_ref()
                    .map(|date| date.start.parsed);
                let url = page.properties.url.rich_text.plain_text();
                let url = Some(url).filter(|url| url.is_empty().not());

                let (path, identifier) = match (date, url) {
                    (Some(Either::Right(datetime)), _) => bail!(
                        "Diary dates must not contain time but page {} has datetime {}",
                        page.id,
                        datetime
                    ),
                    (Some(Either::Left(date)), Some(url)) => bail!("Diary currently doesn't support rendering a page with both a date and a URL but page {} has date {} and URL {}", page.id, date, url),
                    (None, None) => bail!("Diary pages must have either a date or a URL"),
                    (Some(Either::Left(date)), None) => {
                        (format_day(date, true), Either::Left(date))
                    }
                    (None, Some(url)) => (format!("/{}", url), Either::Right(url)),
                };

                Ok((page, path, identifier))
            })
            .fold::<Result<_>, _>(
                Ok((HashMap::with_capacity(length), BTreeMap::new(), Vec::new())),
                |acc, result: Result<_>| {
                    let (mut link_map, mut lookup_tree, mut article_pages) = acc?;
                    let (page, path, identifier) = result?;

                    link_map.insert(page.id, path);
                    match identifier {
                        Either::Left(date) => {
                            lookup_tree.insert(date, page);
                        }
                        Either::Right(url) => {
                            article_pages.push((url, page));
                        }
                    };

                    Ok((link_map, lookup_tree, article_pages))
                },
            )?;

        let read_config_file = async {
            tokio::fs::File::open(dir.join("config.json"))
                .await
                .map(Some)
                .or_else(|error| match error.kind() {
                    io::ErrorKind::NotFound => Ok(None),
                    _ => Err(error),
                })
                .context("Failed to read config.json file")
        };

        let (head, header, footer, config_file) = tokio::try_join!(
            read_partial_file(dir.join("head.html")),
            read_partial_file(dir.join("header.html")),
            read_partial_file(dir.join("footer.html")),
            read_config_file,
        )?;
        let head = PreEscaped(head);
        let header = PreEscaped(header);
        let footer = PreEscaped(footer);
        let config = match config_file {
            Some(file) => serde_json::from_reader::<_, Config>(file.into_std().await)
                .context("Failed to parse config.json")?,
            None => Default::default(),
        };

        let downloadables = Downloadables::new();

        Ok(Generator {
            downloadables,
            link_map,
            lookup_tree,
            article_pages,
            head,
            header,
            footer,
            config,
            directory: dir.to_owned(),
        })
    }

    pub fn get_first_and_last_dates(&self) -> Option<(Date, Date)> {
        match (
            self.lookup_tree.first_key_value(),
            self.lookup_tree.last_key_value(),
        ) {
            (Some((&first_date, _)), Some((&last_date, _))) => Some((first_date, last_date)),
            (Some((&first_date, _)), None) => Some((first_date, first_date)),
            (None, Some((&last_date, _))) => Some((last_date, last_date)),
            (None, None) => None,
        }
    }

    async fn write_if_not_empty(option: Option<(PathBuf, Markup)>) -> Result<()> {
        match option {
            Some((path, markup)) => write(path, markup.into_string()).await,
            None => Ok(()),
        }
    }

    fn render_article<I>(
        &self,
        renderer: &HtmlRenderer,
        page: &Page<Properties>,
        blocks: I,
    ) -> Result<Markup>
    where
        I: Iterator<Item = Result<Markup>>,
    {
        let date = page
            .properties
            .date
            .date
            .as_ref()
            .map(get_date)
            .or_else(|| page.properties.published.date.as_ref().map(get_date));

        let cover = self.download_cover(page)?;

        Ok(html! {
            article {
                header {
                    (renderer.render_heading(page.id, None, Heading::H1, page.properties.title()))
                    @if let Some(date) = date {
                        (render_article_time(date)?)
                    }
                    @if let Some(cover) = cover {
                        img alt=(format!("{} cover", page.properties.title().plain_text())) src=(cover);
                    }
                }
                @for block in blocks {
                    (block?)
                }
            }
        })
    }

    pub async fn download_all(self, client: Client) -> Result<()> {
        self.downloadables
            .download_all(client, Path::new(EXPORT_DIR))
            .await
    }

    pub fn generate_years(
        &self,
        first_date: Date,
        last_date: Date,
    ) -> Result<JoinHandle<Result<()>>> {
        let years = (first_date.year()..=last_date.year())
            .map(|year| {
                let first_day = Date::from_calendar_date(year, Month::January, 1).unwrap();
                let next_year = Date::from_calendar_date(year + 1, Month::January, 1).unwrap();

                let range = self.lookup_tree.range(first_day..next_year);

                let (current_pages, pages) = range
                    .map(|(_, page)| page)
                    .map(|page| (page.id, page))
                    .unzip::<_, _, HashSet<_>, Vec<_>>();

                if pages.is_empty() {
                    return Ok(None);
                }

                let renderer = HtmlRenderer {
                    heading_anchors: HeadingAnchors::After("#"),
                    current_pages,
                    link_map: &self.link_map,
                    downloadables: &self.downloadables,
                };

                let rendered_pages = pages
                    .into_iter()
                    .map(|page| (page, renderer.render_blocks(&page.children, None, 1)));

                let title = format!("{} - {}", year, self.config.name);
                let path = format_year(year);

                let markup = html! {
                    (DOCTYPE)
                    html lang=(self.config.locale.lang) {
                        head {
                            meta charset="utf-8";
                            meta name="viewport" content="width=device-width, initial-scale=1";
                            link rel="stylesheet" href="/katex/katex.min.css";
                            title { (title) }
                            @if let Some(author) = &self.config.author {
                                meta name="author" content=(author.name);
                            }

                            meta property="og:title" content=(title);
                            // TODO: What's a good description for years? Should we just say
                            // something like "All entries for year 2021 from Diary"?
                            meta property="og:locale" content=(self.config.locale.locale);
                            // TODO: Should we use the first cover in the year as an image?
                            // Would be cool to generate some custom covers here
                            @if let Some(url) = &self.config.url {
                                meta property="og:url" content=(url.join(&path)?);
                            }
                            @if let Some(twitter_site) = &self.config.twitter.site {
                                meta name="twitter:site" content=(twitter_site);
                            }
                            @if let Some(twitter_creator) = &self.config.twitter.creator {
                                meta name="twitter:creator" content=(twitter_creator);
                            }

                            (self.head)
                        }
                        body {
                            header {
                                (self.header)
                            }
                            main {
                                @for (page, blocks) in rendered_pages {
                                    (self.render_article(&renderer, page, blocks)?)
                                }
                            }
                            footer {
                                (self.footer)
                            }
                        }
                    }
                };

                let mut path = self.directory.join(EXPORT_DIR).join(path);
                path.set_extension("html");
                Ok(Some((path, markup)))
            })
            .map_ok(Self::write_if_not_empty)
            .collect::<Result<FuturesUnordered<_>>>()?;

        Ok(tokio::spawn(years.try_collect::<()>()))
    }

    pub fn generate_months(
        &self,
        first_date: Date,
        last_date: Date,
    ) -> Result<JoinHandle<Result<()>>> {
        let months = (first_date.year()..=last_date.year())
            .cartesian_product(months::all())
            .map(|(year, &month)| {
                let first_day = Date::from_calendar_date(year, month, 1).unwrap();
                let the_year_next_month = if month == Month::December {
                    year + 1
                } else {
                    year
                };
                let next_month =
                    Date::from_calendar_date(the_year_next_month, month.next(), 1).unwrap();

                let range = self.lookup_tree.range(first_day..next_month);

                let (current_pages, pages) = range
                    .map(|(_, page)| page)
                    .map(|page| (page.id, page))
                    .unzip::<_, _, HashSet<_>, Vec<_>>();

                if pages.is_empty() {
                    return Ok(None);
                }

                let renderer = HtmlRenderer {
                    heading_anchors: HeadingAnchors::After("#"),
                    current_pages,
                    link_map: &self.link_map,
                    downloadables: &self.downloadables,
                };

                let rendered_pages = pages
                    .into_iter()
                    .map(|page| (page, renderer.render_blocks(&page.children, None, 1)));

                let title = format!("{} {} - {}", month, year, self.config.name);
                let path = format_month(year, month);

                let markup = html! {
                    (DOCTYPE)
                    html lang=(self.config.locale.lang) {
                        head {
                            meta charset="utf-8";
                            meta name="viewport" content="width=device-width, initial-scale=1";
                            link rel="stylesheet" href="/katex/katex.min.css";
                            title { (title) }
                            @if let Some(author) = &self.config.author {
                                meta name="author" content=(author.name);
                            }

                            meta property="og:title" content=(title);
                            // TODO: What's a good description for months? Should we just say
                            // something like "All entries for Nov 2021 from Diary"?
                            meta property="og:locale" content=(self.config.locale.locale);
                            // TODO: Should we use the first cover in the months as an image?
                            // Would be cool to generate some custom covers here
                            @if let Some(url) = &self.config.url {
                                meta property="og:url" content=(url.join(&path)?);
                            }
                            @if let Some(twitter_site) = &self.config.twitter.site {
                                meta name="twitter:site" content=(twitter_site);
                            }
                            @if let Some(twitter_creator) = &self.config.twitter.creator {
                                meta name="twitter:creator" content=(twitter_creator);
                            }

                            (self.head)
                        }
                        body {
                            header {
                                (self.header)
                            }
                            main {
                                @for (page, blocks) in rendered_pages {
                                    (self.render_article(&renderer, page, blocks)?)
                                }
                            }
                            footer {
                                (self.footer)
                            }
                        }
                    }
                };

                let mut path = self.directory.join(EXPORT_DIR).join(path);
                path.set_extension("html");
                Ok(Some((path, markup)))
            })
            .map_ok(Self::write_if_not_empty)
            .collect::<Result<FuturesUnordered<_>>>()?;

        Ok(tokio::spawn(months.try_collect::<()>()))
    }

    pub fn generate_days(&self) -> Result<JoinHandle<Result<()>>> {
        let days = self
            .lookup_tree
            .iter()
            .map(|(date, page)| {
                let renderer = HtmlRenderer {
                    heading_anchors: HeadingAnchors::After("#"),
                    current_pages: HashSet::from([page.id]),
                    link_map: &self.link_map,
                    downloadables: &self.downloadables,
                };

                let blocks = renderer.render_blocks(&page.children, None, 1);

                let title = format!(
                    "{} - {}",
                    page.properties.title().plain_text(),
                    self.config.name
                );
                let description = page
                    .properties
                    .description
                    .rich_text
                    .as_slice()
                    .plain_text();

                let prev_page = self
                    .lookup_tree
                    .range((Bound::Unbounded, Bound::Excluded(date)))
                    .rev()
                    .next();
                let next_page = self
                    .lookup_tree
                    .range((Bound::Excluded(date), Bound::Unbounded))
                    .next();

                let cover = self.download_cover(page)?;
                let path = format_day(*date, false);

                let markup = html! {
                    (DOCTYPE)
                    html lang=(self.config.locale.lang) {
                        head {
                            meta charset="utf-8";
                            meta name="viewport" content="width=device-width, initial-scale=1";
                            link rel="stylesheet" href="/katex/katex.min.css";
                            title { (title) }
                            @if !description.is_empty() {
                                meta name="description" content=(description);
                            }
                            @if let Some(author) = &self.config.author {
                                meta name="author" content=(author.name);
                            }

                            meta property="og:title" content=(title);
                            @if !description.is_empty() {
                                meta property="og:description" content=(description);
                            }
                            meta property="og:locale" content=(self.config.locale.locale);
                            @if let Some(cover) = cover {
                                meta property="og:image" content=(cover);
                                meta name="twitter:card" content="summary_large_image";
                            }
                            @if let Some(url) = &self.config.url {
                                meta property="og:url" content=(url.join(&path)?);
                            }
                            @if let Some(twitter_site) = &self.config.twitter.site {
                                meta name="twitter:site" content=(twitter_site);
                            }
                            @if let Some(twitter_creator) = &self.config.twitter.creator {
                                meta name="twitter:creator" content=(twitter_creator);
                            }
                            // TODO: Rest of OG meta properties

                            (self.head)
                        }
                        body {
                            header {
                                (self.header)
                            }
                            main {
                                (self.render_article(&renderer, page, blocks)?)
                                (render_paging_links(&renderer, *date, prev_page, next_page)?)
                            }
                            footer {
                                (self.footer)
                            }
                        }
                    }
                };

                let mut path = self.directory.join(EXPORT_DIR).join(path);
                path.set_extension("html");
                Ok(Some((path, markup)))
            })
            .map_ok(Self::write_if_not_empty)
            .collect::<Result<FuturesUnordered<_>>>()?;

        Ok(tokio::spawn(days.try_collect::<()>()))
    }

    pub fn generate_index_page(&self) -> Result<JoinHandle<Result<()>>> {
        struct IndexMonth {
            month: (i32, Month),
            markup: String,
        }

        struct IndexYear {
            year: i32,
            markup: String,
        }

        let renderer = HtmlRenderer {
            heading_anchors: HeadingAnchors::After("#"),
            current_pages: HashSet::new(),
            link_map: &self.link_map,
            downloadables: &self.downloadables,
        };

        let years = self
            .lookup_tree
            .iter()
            .rev()
            .map(|(&date, page)| IndexMonth {
                month: (date.year(), date.month()),
                markup: (html! {
                    article {
                        header {
                            h3 {
                                a href=(format_day(date, true)) {
                                    (renderer.render_rich_text(page.properties.title()))
                                }
                            }
                            (render_article_time(date).unwrap())
                        }
                        p {
                            (page.properties.description.rich_text.plain_text())
                        }
                    }
                })
                .into_string(),
            })
            .coalesce(|a, b| {
                if a.month == b.month {
                    Ok(IndexMonth {
                        month: a.month,
                        markup: a.markup + &b.markup,
                    })
                } else {
                    Err((a, b))
                }
            })
            .map(
                |IndexMonth {
                     month: (year, month),
                     markup,
                 }| IndexYear {
                    year,
                    markup: (html! {
                        section {
                            h2 {
                                a href=(format_month(year, month)) {
                                    (month)
                                }
                            }
                            (PreEscaped(markup))
                        }
                    })
                    .into_string(),
                },
            )
            .coalesce(|a, b| {
                if a.year == b.year {
                    Ok(IndexYear {
                        year: a.year,
                        markup: a.markup + &b.markup,
                    })
                } else {
                    Err((a, b))
                }
            })
            .map(|IndexYear { year, markup }| {
                html! {
                    section {
                        h1 {
                            a href=(format_year(year)) {
                                (year)
                            }
                        }
                        (PreEscaped(markup))
                    }
                }
            });

        let markup = html! {
            (DOCTYPE)
            html lang=(self.config.locale.lang) {
                head {
                    meta charset="utf-8";
                    meta name="viewport" content="width=device-width, initial-scale=1";
                    meta name="description" content=(self.config.description);
                    link rel="stylesheet" href="/katex/katex.min.css";
                    title { (self.config.name) }
                    @if let Some(author) = &self.config.author {
                        meta name="author" content=(author.name);
                    }

                    meta property="og:title" content=(self.config.name);
                    meta property="og:description" content=(self.config.description);
                    meta property="og:locale" content=(self.config.locale.locale);
                    @if let Some(cover) = &self.config.cover {
                        meta property="og:image" content=(cover);
                        meta name="twitter:card" content="summary_large_image";
                    }
                    @if let Some(url) = &self.config.url {
                        meta property="og:url" content=(url);
                    }
                    @if let Some(twitter_site) = &self.config.twitter.site {
                        meta name="twitter:site" content=(twitter_site);
                    }
                    @if let Some(twitter_creator) = &self.config.twitter.creator {
                        meta name="twitter:creator" content=(twitter_creator);
                    }
                    // TODO: Rest of OG meta properties

                    (self.head)
                }
                body {
                    header {
                        (self.header)
                    }
                    main {
                        @for year in years {
                            (year)
                        }
                    }
                    footer {
                        (self.footer)
                    }
                }
            }
        };

        let mut path = self.directory.join(EXPORT_DIR).join("index");
        path.set_extension("html");

        Ok(tokio::spawn(write(path, markup.into_string())))
    }

    pub fn generate_atom_feed(&self) -> Result<JoinHandle<Result<()>>> {
        const FEED_FILE: &str = "feed.xml";

        let url = if let Some(url) = &self.config.url {
            url.clone()
        } else {
            warn!("Cannot generate Atom feed without a unique URL to identify it");
            return Ok(tokio::spawn(async { Ok(()) }));
        };

        let authors = if let Some(author) = &self.config.author {
            vec![atom::Person {
                name: &author.name,
                email: None,
                url: author.url.clone(),
            }]
        } else {
            Vec::new()
        };

        enum UrlOrDate {
            Url(String),
            Date(Date),
        }

        let publications_ordered = self
            .article_pages
            .iter()
            .map(|(url, page)| (UrlOrDate::Url(url.to_owned()), page))
            .chain(
                self.lookup_tree
                    .iter()
                    .map(|(date, page)| (UrlOrDate::Date(*date), page)),
            )
            .filter_map(|(id, page)| {
                page.properties.published.date.as_ref().map(|date| {
                    let datetime = match date.start.parsed {
                        Either::Left(date) => date.with_time(time::Time::MIDNIGHT).assume_utc(),
                        Either::Right(datetime) => datetime,
                    };
                    (datetime, id, page)
                })
            })
            .sorted_unstable_by_key(|page| page.0)
            .collect::<Vec<_>>();

        let last_publication = if let Some((time, _, _)) = publications_ordered.last() {
            *time
        } else {
            return Ok(tokio::spawn(async { Ok(()) }));
        };

        let renderer = HtmlRenderer {
            heading_anchors: HeadingAnchors::None,
            current_pages: publications_ordered
                .iter()
                .map(|(_, _, page)| page.id)
                .collect(),
            link_map: &self.link_map,
            downloadables: &self.downloadables,
        };

        let entries = publications_ordered
            .into_iter()
            .map(|(time, id, page)| {
                let blocks = renderer.render_blocks(&page.children, None, 0);

                let url = match id {
                    UrlOrDate::Url(url) => url,
                    UrlOrDate::Date(date) => format_day(date, true),
                };

                Ok(atom::Entry {
                    title: page.properties.name.title.plain_text(),
                    url,
                    updated: OffsetDateTime::parse(&page.last_edited_time, &Rfc3339)?,
                    published: time,
                    summary: page.properties.description.rich_text.plain_text(),
                    content: html! {
                        @for block in blocks {
                            (block?)
                        }
                    },
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let feed = atom::Feed {
            title: &self.config.name,
            url: url.clone(),
            feed_url: url.join(FEED_FILE)?,
            last_changed: last_publication,
            authors,
            generator: atom::Generator {
                value: DIARY_GENERATOR,
                uri: REPOSITORY,
                version: VERSION,
            },
            icon: self.config.icon.as_deref(),
            cover: self.config.cover.as_deref(),
            lang: &self.config.locale.locale,
            entries,
        };

        let path = self.directory.join(EXPORT_DIR).join(FEED_FILE);
        Ok(tokio::spawn(write(path, feed.render().into_string())))
    }

    pub fn generate_article_pages(&self) -> Result<JoinHandle<Result<()>>> {
        let articles = self
            .article_pages
            .iter()
            .map(|(url, page)| {
                let renderer = HtmlRenderer {
                    heading_anchors: HeadingAnchors::After("#"),
                    current_pages: HashSet::from([page.id]),
                    link_map: &self.link_map,
                    downloadables: &self.downloadables,
                };

                let blocks = renderer.render_blocks(&page.children, None, 1);

                let title = format!(
                    "{} - {}",
                    page.properties.title().plain_text(),
                    self.config.name
                );
                let description = page
                    .properties
                    .description
                    .rich_text
                    .as_slice()
                    .plain_text();

                let cover = self.download_cover(page)?;

                let markup = html! {
                    (DOCTYPE)
                    html lang=(self.config.locale.lang) {
                        head {
                            meta charset="utf-8";
                            meta name="viewport" content="width=device-width, initial-scale=1";
                            link rel="stylesheet" href="/katex/katex.min.css";
                            title { (title) }
                            @if !description.is_empty() {
                                meta name="description" content=(description);
                            }
                            @if let Some(author) = &self.config.author {
                                meta name="author" content=(author.name);
                            }

                            meta property="og:title" content=(title);
                            @if !description.is_empty() {
                                meta property="og:description" content=(description);
                            }
                            meta property="og:locale" content=(self.config.locale.locale);
                            @if let Some(cover) = cover {
                                meta property="og:image" content=(cover);
                                meta name="twitter:card" content="summary_large_image";
                            }
                            @if let Some(site_url) = &self.config.url {
                                meta property="og:url" content=(site_url.join(url)?);
                            }
                            @if let Some(twitter_site) = &self.config.twitter.site {
                                meta name="twitter:site" content=(twitter_site);
                            }
                            @if let Some(twitter_creator) = &self.config.twitter.creator {
                                meta name="twitter:creator" content=(twitter_creator);
                            }
                            // TODO: Rest of OG meta properties

                            (self.head)
                        }
                        body {
                            header {
                                (self.header)
                            }
                            main {
                                (self.render_article(&renderer, page, blocks)?)
                            }
                            footer {
                                (self.footer)
                            }
                        }
                    }
                };

                let mut path = self.directory.join(EXPORT_DIR).join(url);
                path.set_extension("html");
                Ok(Some((path, markup)))
            })
            .map_ok(Self::write_if_not_empty)
            .collect::<Result<FuturesUnordered<_>>>()?;

        Ok(tokio::spawn(articles.try_collect::<()>()))
    }

    pub fn generate_articles_page(&self) -> Result<JoinHandle<Result<()>>> {
        let renderer = HtmlRenderer {
            heading_anchors: HeadingAnchors::After("#"),
            current_pages: HashSet::from([]),
            link_map: &self.link_map,
            downloadables: &self.downloadables,
        };

        let articles = self.article_pages.iter().filter_map(|(url, page)| {
            let published_date = page.properties.published.date.as_ref().map(get_date);

            let published_date = match published_date {
                Some(published_date) => published_date,
                _ => return None,
            };

            Some(html! {
                article {
                    header {
                        h3 {
                            a href=(url) {
                                (renderer.render_rich_text(page.properties.title()))
                            }
                        }
                        (render_article_time(published_date).unwrap())
                    }
                    p {
                        (page.properties.description.rich_text.plain_text())
                    }
                }
            })
        });

        let title = format!("Articles - {}", self.config.name);

        let markup = html! {
            (DOCTYPE)
            html lang=(self.config.locale.lang) {
                head {
                    meta charset="utf-8";
                    meta name="viewport" content="width=device-width, initial-scale=1";
                    link rel="stylesheet" href="/katex/katex.min.css";
                    title { (title) }
                    @if let Some(author) = &self.config.author {
                        meta name="author" content=(author.name);
                    }

                    meta property="og:title" content=(title);
                    // TODO: What's a good description for the articles page?
                    // TODO: Rest of OG meta properties
                    meta property="og:locale" content=(self.config.locale.locale);
                    // TODO: One could generate a custom image for this page once
                    @if let Some(url) = &self.config.url {
                        meta property="og:url" content=(url.join("articles")?);
                    }
                    @if let Some(twitter_site) = &self.config.twitter.site {
                        meta name="twitter:site" content=(twitter_site);
                    }
                    @if let Some(twitter_creator) = &self.config.twitter.creator {
                        meta name="twitter:creator" content=(twitter_creator);
                    }

                    (self.head)
                }
                body {
                    header {
                        (self.header)
                    }
                    main {
                        @for article in articles {
                            (article)
                        }
                    }
                    footer {
                        (self.footer)
                    }
                }
            }
        };

        let mut path = self.directory.join(EXPORT_DIR).join("articles");
        path.set_extension("html");
        Ok(tokio::spawn(write(path, markup.into_string())))
    }

    /// Generate independent pages by reading the pages/ directory and using each of the file in it
    /// as partial content for a page
    /// The pages titles currently depend on the file name as well
    /// These pages are called independent as they don't depend on Notion
    pub fn generate_independent_pages(&self) -> JoinHandle<Result<()>> {
        // We need to clone these so that the spawned future is 'static (AKA owns everything inside
        // of it)
        let head = self.head.clone();
        let header = self.header.clone();
        let footer = self.footer.clone();
        let config = self.config.clone();
        let directory = self.directory.clone();

        tokio::spawn(async move {
            let files = ReadDirStream::new(tokio::fs::read_dir("pages").await?);

            // We do this so that the inner futures in `.and_then` don't take ownership of these
            // causing them to be unusable by subsequent calls to `.and_then`
            let head_ref = &head;
            let header_ref = &header;
            let footer_ref = &footer;
            let config_ref = &config;
            let directory_ref = &directory;

            files
                .map(|result| {
                    result.context(
                        "Failed to read file while recursively generating independent pages",
                    )
                })
                .and_then(|entry| async move {
                    let file_type = entry.file_type().await?;
                    let path = entry.path();

                    if !file_type.is_file() {
                        bail!(
                            "pages/ must only contain HTML files. {} which is not a file.",
                            path.display(),
                        );
                    }

                    let (file_name, file_ext) =
                        match path.file_name().and_then(std::ffi::OsStr::to_str) {
                            Some(file_name) => {
                                if let Some(file_without_ext) = file_name.strip_suffix(".html") {
                                    (file_without_ext, "html")
                                } else {
                                    bail!(
                                        "File {} isn't an HTML file, make sure it ends with .html",
                                        file_name
                                    )
                                }
                            }
                            None => bail!("Not a valid html file {}", path.display()),
                        };

                    let content = tokio::fs::read_to_string(&path).await?;

                    // For title we want the first letter to be uppercase
                    let mut title = file_name.to_string();
                    if let Some(first_char) = title.get_mut(0..1) {
                        first_char.make_ascii_uppercase();
                    }
                    let title = format!("{} - {}", title, config_ref.name);

                    let markup = html! {
                        (DOCTYPE)
                        html lang=(config_ref.locale.lang) {
                            head {
                                meta charset="utf-8";
                                meta name="viewport" content="width=device-width, initial-scale=1";
                                title { (title) }
                                @if let Some(author) = &config_ref.author {
                                    meta name="author" content=(author.name);
                                }

                                meta property="og:title" content=(title);
                                // TODO: Should there be a mechanism to set the description
                                // for independent pages?
                                meta property="og:locale" content=(config_ref.locale.locale);
                                // TODO: Same as description but for images
                                @if let Some(url) = &config_ref.url {
                                    meta property="og:url" content=(url.join(file_name)?);
                                }
                                @if let Some(twitter_site) = &config_ref.twitter.site {
                                    meta name="twitter:site" content=(twitter_site);
                                }
                                @if let Some(twitter_creator) = &config_ref.twitter.creator {
                                    meta name="twitter:creator" content=(twitter_creator);
                                }

                                (*head_ref)
                            }
                            body {
                                header {
                                    (*header_ref)
                                }
                                (PreEscaped(content))
                                footer {
                                    (*footer_ref)
                                }
                            }
                        }
                    };

                    let mut path = directory_ref.join(EXPORT_DIR).join(file_name);
                    path.set_extension(file_ext);
                    write(path, markup.into_string()).await
                })
                .try_collect::<()>()
                .await
        })
    }

    fn download_cover(&self, page: &Page<Properties>) -> Result<Option<String>> {
        let cover = page
            .cover
            .as_ref()
            // Even though a page's cover doesn't have a unique id, since we know nothing else
            // will use that id as media we will give it to the cover
            .map(|file| file.as_downloadable(page.id))
            .transpose()?;

        let src = cover.as_ref().map(|downloadable| downloadable.src_path());

        if let Some(cover) = cover {
            self.downloadables.insert(cover);
        }

        Ok(src)
    }
}
