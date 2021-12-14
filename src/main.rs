#![feature(map_first_last)]

use anyhow::{bail, Context, Result};
use either::Either;
use futures_util::stream::{FuturesUnordered, TryStreamExt};
use maud::{html, DOCTYPE};
use notion_generator::{
    client::NotionClient,
    options::HeadingAnchors,
    render::Title,
    response::{
        properties::{DateProperty, RichTextProperty, TitleProperty},
        Page, RichText,
    },
    HtmlRenderer,
};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::Path,
};
use time::{macros::format_description, Date, Month};
use tokio::task::JoinHandle;

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

fn bisect_pages(
    pages: Vec<Page<Properties>>,
) -> Result<(
    HashMap<String, String>,
    BTreeMap<Date, Page<Properties>>,
    Vec<(String, Page<Properties>)>,
)> {
    let length = pages.len();

    Ok(pages
        .into_iter()
        .map(|page| {
            let date = page.properties.date.date.as_ref().map(|date| date.start.parsed);

            let (path, identifier) = match (date, page.properties.url.rich_text.get(0)) {
                (Some(Either::Right(datetime)), _) => bail!(
                    "Diary dates must not contain time but page {} has datetime {}",
                    page.id,
                    datetime
                ),
                (Some(Either::Left(date)), Some(url)) => bail!(
                    "Diary currently doesn't support rendering a page with both a date and a URL but page {} has date {} and URL {}",
                    page.id,
                    date,
                    url.plain_text
                ),
                (None, None) => bail!("Diary pages must have either a date or a URL"),
                (Some(Either::Left(date)), None) => (date.format(format_description!("/[year]/[month]/[day]"))?, Either::Left(date)),
                (None, Some(url)) => (format!("/{}", url.plain_text), Either::Right(url.plain_text.clone()))
            };

            Ok((page, path, identifier))
        })
        .fold::<Result<_>, _>(
            Ok((HashMap::with_capacity(length), BTreeMap::new(), Vec::new())),
            |acc, result: Result<_>| {
                let (mut link_map, mut lookup_tree, mut floating_pages) = acc?;
                let (page, path, identifier) = result?;

                link_map.insert(page.id.clone(), path);
                match identifier {
                    Either::Left(date) => {
                        lookup_tree.insert(date, page);
                    },
                    Either::Right(url) => {
                        floating_pages.push((url, page));
                    },
                };

                Ok((link_map, lookup_tree, floating_pages))
            },
        )?)
}

fn generate_years(
    lookup_tree: &BTreeMap<Date, Page<Properties>>,
    link_map: &HashMap<String, String>,
    first_date: &Date,
    last_date: &Date,
) -> Result<JoinHandle<std::io::Result<()>>> {
    let years = (first_date.year()..=last_date.year())
        .map(|year| {
            let first_day = Date::from_calendar_date(year, Month::January, 1).unwrap();
            let next_year = Date::from_calendar_date(year + 1, Month::January, 1).unwrap();

            let range = lookup_tree.range(first_day..next_year);

            let (current_pages, pages) = range
                .map(|(_, page)| (page.id.clone(), page))
                .unzip::<_, _, HashSet<_>, Vec<_>>();

            let renderer = HtmlRenderer {
                heading_anchors: HeadingAnchors::Icon,
                current_pages,
                link_map: link_map.clone(),
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
                        link rel="stylesheet" href="styles/katex.css";

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

            let mut path = Path::new("public").join(format!("{:0>4}", year));
            path.set_extension("html");
            Ok(tokio::fs::write(path, markup.into_string()))
        })
        .collect::<Result<FuturesUnordered<_>>>()?;

    Ok(tokio::spawn(years.try_collect::<()>()))
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect::<Vec<String>>();
    let auth_token = std::env::var("NOTION_TOKEN").context("Missing NOTION_TOKEN env variable")?;
    let database_id = args.get(1).context("Missing page id as first argument")?;

    let client = NotionClient::new(auth_token);
    let pages = client.get_database_pages::<Properties>(database_id).await?;

    let (link_map, lookup_tree, floating_pages) = bisect_pages(pages)?;

    let (first_date, last_date) =
        match (lookup_tree.first_key_value(), lookup_tree.last_key_value()) {
            (Some((first_date, _)), Some((last_date, _))) => (first_date, last_date),
            (Some((first_date, _)), None) => (first_date, first_date),
            (None, Some((last_date, _))) => (last_date, last_date),
            (None, None) => return Ok(()),
        };

    generate_years(&lookup_tree, &link_map, first_date, last_date)?.await??;

    Ok(())
}
