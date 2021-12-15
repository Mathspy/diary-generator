#![feature(map_first_last)]

use anyhow::{bail, Context, Result};
use either::Either;
use futures_util::stream::{FuturesUnordered, TryStreamExt};
use itertools::Itertools;
use maud::{html, DOCTYPE};
use notion_generator::{
    client::NotionClient,
    options::HeadingAnchors,
    render::Title,
    response::{
        properties::{DateProperty, RichTextProperty, TitleProperty},
        Page, PlainText, RichText,
    },
    HtmlRenderer,
};
use reqwest::Client;
use serde::Deserialize;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    ops::Not,
    path::Path,
};
use time::{macros::format_description, Date, Month};
use tokio::task::JoinHandle;

const EXPORT_DIR: &str = "public";

#[derive(Deserialize)]
struct Properties {
    name: TitleProperty,
    date: DateProperty,
    url: RichTextProperty,
    description: RichTextProperty,
    published: DateProperty,
}

impl Title for Properties {
    fn title(&self) -> &[RichText] {
        self.name.title.as_slice()
    }
}

struct Generator {
    link_map: HashMap<String, String>,
    lookup_tree: BTreeMap<Date, Page<Properties>>,
    independent_pages: Vec<(String, Page<Properties>)>,
}

impl Generator {
    fn new(pages: Vec<Page<Properties>>) -> Result<Generator> {
        let length = pages.len();

        let (link_map, lookup_tree, independent_pages) = pages
            .into_iter()
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
                    (Some(Either::Left(date)), None) => (
                        date.format(format_description!("/[year]/[month]/[day]"))?,
                        Either::Left(date),
                    ),
                    (None, Some(url)) => (format!("/{}", url), Either::Right(url)),
                };

                Ok((page, path, identifier))
            })
            .fold::<Result<_>, _>(
                Ok((HashMap::with_capacity(length), BTreeMap::new(), Vec::new())),
                |acc, result: Result<_>| {
                    let (mut link_map, mut lookup_tree, mut independent_pages) = acc?;
                    let (page, path, identifier) = result?;

                    link_map.insert(page.id.clone(), path);
                    match identifier {
                        Either::Left(date) => {
                            lookup_tree.insert(date, page);
                        }
                        Either::Right(url) => {
                            independent_pages.push((url, page));
                        }
                    };

                    Ok((link_map, lookup_tree, independent_pages))
                },
            )?;

        Ok(Generator {
            link_map,
            lookup_tree,
            independent_pages,
        })
    }

    fn get_first_and_last_dates(&self) -> Option<(Date, Date)> {
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
}

async fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> Result<()> {
    let path = path.as_ref();
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

fn generate_years(
    lookup_tree: &BTreeMap<Date, Page<Properties>>,
    link_map: &HashMap<String, String>,
    first_date: Date,
    last_date: Date,
) -> Result<JoinHandle<Result<()>>> {
    let years = (first_date.year()..=last_date.year())
        .map(|year| {
            let first_day = Date::from_calendar_date(year, Month::January, 1).unwrap();
            let next_year = Date::from_calendar_date(year + 1, Month::January, 1).unwrap();

            let range = lookup_tree.range(first_day..next_year);

            let (current_pages, pages) = range
                .map(|(_, page)| (page.id.clone(), page))
                .unzip::<_, _, HashSet<_>, Vec<_>>();

            if pages.is_empty() {
                return Ok(None);
            }

            let renderer = HtmlRenderer {
                heading_anchors: HeadingAnchors::Icon,
                current_pages,
                link_map,
            };

            let rendered_pages = pages
                .into_iter()
                .map(|page| renderer.render_page(page).map(|(markup, _)| markup));

            let markup = html! {
                (DOCTYPE)
                html lang="en" {
                    head {
                        meta charset="utf-8";
                        meta name="viewport" content="width=device-width, initial-scale=1";
                        link rel="stylesheet" href="/katex/katex.min.css";

                        title { (year) }
                    }
                    body {
                        main {
                            @for block in rendered_pages {
                                (block?)
                            }
                        }
                    }
                }
            };

            let mut path = Path::new(EXPORT_DIR).join(format!("{:0>4}", year));
            path.set_extension("html");
            Ok(Some((path, markup)))
        })
        .map(|result| {
            result.map(|option| async move {
                match option {
                    Some((path, markup)) => write(path, markup.into_string()).await,
                    None => Ok(()),
                }
            })
        })
        .collect::<Result<FuturesUnordered<_>>>()?;

    Ok(tokio::spawn(years.try_collect::<()>()))
}

mod months {
    use time::Month;

    const MONTHS: &[Month] = &[
        Month::January,
        Month::February,
        Month::March,
        Month::April,
        Month::May,
        Month::June,
        Month::July,
        Month::August,
        Month::September,
        Month::October,
        Month::November,
        Month::December,
    ];

    pub fn all() -> std::slice::Iter<'static, Month> {
        MONTHS.iter()
    }
}

fn generate_months(
    lookup_tree: &BTreeMap<Date, Page<Properties>>,
    link_map: &HashMap<String, String>,
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

            let range = lookup_tree.range(first_day..next_month);

            let (current_pages, pages) = range
                .map(|(_, page)| (page.id.clone(), page))
                .unzip::<_, _, HashSet<_>, Vec<_>>();

            if pages.is_empty() {
                return Ok(None);
            }

            let renderer = HtmlRenderer {
                heading_anchors: HeadingAnchors::Icon,
                current_pages,
                link_map,
            };

            let rendered_pages = pages
                .into_iter()
                .map(|page| renderer.render_page(page).map(|(markup, _)| markup));

            let markup = html! {
                (DOCTYPE)
                html lang="en" {
                    head {
                        meta charset="utf-8";
                        meta name="viewport" content="width=device-width, initial-scale=1";
                        link rel="stylesheet" href="/katex/katex.min.css";

                        title { (format!("{} {}", month, year)) }
                    }
                    body {
                        main {
                            @for block in rendered_pages {
                                (block?)
                            }
                        }
                    }
                }
            };

            let mut path = Path::new(EXPORT_DIR)
                .join(format!("{:0>4}", year))
                .join(format!("{:0>2}", u8::from(month)));
            path.set_extension("html");
            Ok(Some((path, markup)))
        })
        .map(|result| {
            result.map(|option| async move {
                match option {
                    Some((path, markup)) => write(path, markup.into_string()).await,
                    None => Ok(()),
                }
            })
        })
        .collect::<Result<FuturesUnordered<_>>>()?;

    Ok(tokio::spawn(months.try_collect::<()>()))
}

fn generate_days(
    lookup_tree: &BTreeMap<Date, Page<Properties>>,
    link_map: &HashMap<String, String>,
) -> Result<JoinHandle<Result<()>>> {
    let days = lookup_tree
        .iter()
        .map(|(date, page)| {
            let renderer = HtmlRenderer {
                heading_anchors: HeadingAnchors::Icon,
                current_pages: HashSet::from([page.id.clone()]),
                link_map,
            };

            let rendered_page = renderer.render_page(page).map(|(markup, _)| markup)?;

            let title = page.properties.title().plain_text();
            let description = page
                .properties
                .description
                .rich_text
                .as_slice()
                .plain_text();

            let markup = html! {
                (DOCTYPE)
                html lang="en" {
                    head {
                        meta charset="utf-8";
                        meta name="viewport" content="width=device-width, initial-scale=1";
                        @if !description.is_empty() {
                            meta name="description" content=(description);
                        }
                        link rel="stylesheet" href="/katex/katex.min.css";
                        // TODO: Add `- Game Dev Diary` after each title
                        title { (title) }

                        meta property="og:title" content=(title);
                        // TODO: Rest of OG meta properties
                    }
                    body {
                        main {
                            (rendered_page)
                        }
                    }
                }
            };

            let mut path = Path::new(EXPORT_DIR)
                .join(format!("{:0>4}", date.year()))
                .join(format!("{:0>2}", u8::from(date.month())))
                .join(format!("{:0>2}", date.day()));
            path.set_extension("html");
            Ok(Some((path, markup)))
        })
        .map(|result| {
            result.map(|option| async move {
                match option {
                    Some((path, markup)) => write(path, markup.into_string()).await,
                    None => Ok(()),
                }
            })
        })
        .collect::<Result<FuturesUnordered<_>>>()?;

    Ok(tokio::spawn(days.try_collect::<()>()))
}

fn generate_independents(
    floating_pages: Vec<(String, Page<Properties>)>,
    link_map: &HashMap<String, String>,
) -> Result<JoinHandle<Result<()>>> {
    let independents = floating_pages
        .iter()
        .map(|(url, page)| {
            let renderer = HtmlRenderer {
                heading_anchors: HeadingAnchors::Icon,
                current_pages: HashSet::from([page.id.clone()]),
                link_map,
            };

            let rendered_page = renderer.render_page(page).map(|(markup, _)| markup)?;

            let title = page.properties.title().plain_text();
            let description = page
                .properties
                .description
                .rich_text
                .as_slice()
                .plain_text();

            let markup = html! {
                (DOCTYPE)
                html lang="en" {
                    head {
                        meta charset="utf-8";
                        meta name="viewport" content="width=device-width, initial-scale=1";
                        @if !description.is_empty() {
                            meta name="description" content=(description);
                        }
                        link rel="stylesheet" href="/katex/katex.min.css";
                        // TODO: Add `- Game Dev Diary` after each title
                        title { (title) }

                        meta property="og:title" content=(title);
                        // TODO: Rest of OG meta properties
                    }
                    body {
                        main {
                            (rendered_page)
                        }
                    }
                }
            };

            let mut path = Path::new(EXPORT_DIR).join(url);
            path.set_extension("html");
            Ok(Some((path, markup)))
        })
        .map(|result| {
            result.map(|option| async move {
                match option {
                    Some((path, markup)) => write(path, markup.into_string()).await,
                    None => Ok(()),
                }
            })
        })
        .collect::<Result<FuturesUnordered<_>>>()?;

    Ok(tokio::spawn(independents.try_collect::<()>()))
}

fn katex_download(client: Client) -> JoinHandle<Result<()>> {
    const CDN_URL: &str = "https://cdn.jsdelivr.net/npm/katex@0.15.1/dist/";
    const KATEX_DIR: &str = "katex";

    async fn download_file(client: &Client, file: &str) -> Result<()> {
        let response = client.get(format!("{}{}", CDN_URL, file)).send().await?;

        let status = response.status();
        if status.is_client_error() || status.is_server_error() {
            bail!(
                "Download request for file {} failed with status code {}",
                file,
                status
            )
        }

        let bytes = response.bytes().await?;

        write(Path::new(EXPORT_DIR).join(KATEX_DIR).join(file), bytes).await?;

        Ok(())
    }

    tokio::spawn(async move {
        let response = client
            .get(format!("{}{}", CDN_URL, "katex.min.css"))
            .send()
            .await?;

        let katex_styles = response.text().await?;

        let assets_downloads = katex_styles
            .split("url(")
            .skip(1)
            .map(|part| part.split(')').next())
            .map(|file| {
                file.ok_or_else(|| {
                    anyhow::format_err!("Failed to parse asset URL from Katex stylesheet")
                })
            })
            .map(|result| result.map(|file| download_file(&client, file)))
            .collect::<Result<FuturesUnordered<_>>>()?;

        tokio::try_join!(
            write(
                Path::new(EXPORT_DIR).join(KATEX_DIR).join("katex.min.css"),
                &katex_styles
            ),
            assets_downloads.try_collect::<()>(),
        )?;

        Ok(())
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect::<Vec<String>>();
    let auth_token = std::env::var("NOTION_TOKEN").context("Missing NOTION_TOKEN env variable")?;
    let database_id = args.get(1).context("Missing page id as first argument")?;

    let client = NotionClient::new(auth_token);
    let pages = client.get_database_pages::<Properties>(database_id).await?;

    let generator = Generator::new(pages)?;

    let (first_date, last_date) = match generator.get_first_and_last_dates() {
        Some(dates) => dates,
        None => return Ok(()),
    };

    let results = tokio::try_join!(
        katex_download(client.client().clone()),
        generate_years(
            &generator.lookup_tree,
            &generator.link_map,
            first_date,
            last_date
        )?,
        generate_months(
            &generator.lookup_tree,
            &generator.link_map,
            first_date,
            last_date
        )?,
        generate_days(&generator.lookup_tree, &generator.link_map)?,
        generate_independents(generator.independent_pages, &generator.link_map)?,
    )?;

    match results {
        (Err(error), _, _, _, _) => return Err(error),
        (_, Err(error), _, _, _) => return Err(error),
        (_, _, Err(error), _, _) => return Err(error),
        (_, _, _, Err(error), _) => return Err(error),
        (_, _, _, _, Err(error)) => return Err(error),
        (Ok(()), Ok(()), Ok(()), Ok(()), Ok(())) => {}
    };

    Ok(())
}
