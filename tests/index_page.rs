mod utils;

use diary_generator::{Generator, Properties};
use either::Either;
use maud::{html, DOCTYPE};
use notion_generator::response::{
    properties::{DateProperty, RichTextProperty, TitleProperty},
    NotionDate, Page, PageParent, RichText, RichTextType, Time,
};
use pretty_assertions::assert_eq;
use std::fs;
use tempdir::TempDir;
use time::macros::date;
use utils::DirEntry;

fn new_page(id: &str, title: &str, date: Time, description: &str) -> Page<Properties> {
    Page {
        object: "page".to_string(),
        id: id.parse().unwrap(),
        created_time: "2021-11-29T18:20:00.000Z".to_string(),
        last_edited_time: "2021-12-06T09:25:00.000Z".to_string(),
        cover: None,
        icon: None,
        archived: false,
        properties: Properties {
            name: TitleProperty {
                id: "title".to_string(),
                title: vec![RichText {
                    ty: RichTextType::Text {
                        content: title.to_string(),
                        link: None,
                    },
                    annotations: Default::default(),
                    plain_text: title.to_string(),
                    href: None,
                }],
            },
            published: DateProperty {
                id: "Fpr%3E".to_string(),
                date: Some(NotionDate {
                    start: Time {
                        original: "2021-12-24".to_string(),
                        parsed: Either::Left(date!(2021 - 12 - 24)),
                    },
                    end: None,
                    time_zone: None,
                }),
            },
            date: DateProperty {
                id: "TKGl".to_string(),
                date: Some(NotionDate {
                    start: date,
                    end: None,
                    time_zone: None,
                }),
            },
            url: RichTextProperty {
                id: "NB%3BU".to_string(),
                rich_text: vec![],
            },
            description: RichTextProperty {
                id: "QPqF".to_string(),
                rich_text: vec![RichText {
                    ty: RichTextType::Text {
                        content: description.to_string(),
                        link: None,
                    },
                    annotations: Default::default(),
                    plain_text: description.to_string(),
                    href: None,
                }],
            },
        },
        parent: PageParent::Database {
            id: "4045404e-233a-4278-84f0-b3389887b315".to_string(),
        },
        url: format!("https://www.notion.so/{}", id),
        children: vec![],
    }
}

#[tokio::test]
async fn empty_index() {
    let cwd = TempDir::new("empty_index").unwrap();

    let generator = Generator::new(&cwd, Vec::new()).await.unwrap();
    generator
        .generate_index_page()
        .unwrap()
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        DirEntry::breakdown(&cwd),
        DirEntry::dir(
            cwd.path().file_name().unwrap(),
            [DirEntry::dir("output", [DirEntry::file("index.html")])]
        ),
    );

    assert_eq!(
        fs::read_to_string(cwd.path().join("output").join("index.html")).unwrap(),
        html! {
            (DOCTYPE)
            html lang="en" {
                head {
                    meta charset="utf-8";
                    meta name="viewport" content="width=device-width, initial-scale=1";
                    meta name="description" content="A neat diary";
                    link rel="stylesheet" href="/katex/katex.min.css";
                    title { "Diary" }
                    meta property="og:title" content="Diary";
                    meta property="og:description" content="A neat diary";
                    meta property="og:locale" content="en_US";
                }
                body {
                    header {}
                    main {}
                    footer {}
                }
            }
        }
        .into_string(),
    );
}

#[tokio::test]
async fn simple_index() {
    let cwd = TempDir::new("simple_index").unwrap();

    let generator = Generator::new(
        &cwd,
        vec![
            new_page(
                "cf2bacc9d75c4226aab53601c336f295",
                "Day 0: Nannou, helping L, and lots of noise",
                Time {
                    original: "2021-11-07".to_string(),
                    parsed: Either::Left(date!(2021 - 11 - 07)),
                },
                "Every journey starts with 1 O'clock: assistance. \
I just didn't know mine will also start with noise.",
            ),
            new_page(
                "ac3fb543001f4be5a25e4978abd05b1d",
                "Day 1: Down the rabbit hole we go",
                Time {
                    original: "2021-11-08".to_string(),
                    parsed: Either::Left(date!(2021 - 11 - 08)),
                },
                "Alice starts making games by watching trains with the loveliest coding conductor.",
            ),
            new_page(
                "ac3fb543001f4be5a25e4978abd05b1d",
                "Day 2: Enter Bevy & Shaders are hard",
                Time {
                    original: "2021-11-09".to_string(),
                    parsed: Either::Left(date!(2021 - 11 - 09)),
                },
                "3 O’clock: departure. \
We are not entering the world of Bevy where we will actually make things happen. \
There’s no turning back now",
            ),
        ],
    )
    .await
    .unwrap();
    generator
        .generate_index_page()
        .unwrap()
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        DirEntry::breakdown(&cwd),
        DirEntry::dir(
            cwd.path().file_name().unwrap(),
            [DirEntry::dir("output", [DirEntry::file("index.html")])]
        ),
    );

    assert_eq!(
        fs::read_to_string(cwd.path().join("output").join("index.html")).unwrap(),
        html! {
            (DOCTYPE)
            html lang="en" {
                head {
                    meta charset="utf-8";
                    meta name="viewport" content="width=device-width, initial-scale=1";
                    meta name="description" content="A neat diary";
                    link rel="stylesheet" href="/katex/katex.min.css";
                    title { "Diary" }
                    meta property="og:title" content="Diary";
                    meta property="og:description" content="A neat diary";
                    meta property="og:locale" content="en_US";
                }
                body {
                    header {}
                    main {
                        section {
                            h1 { a href="2021" { "2021" } }
                            section {
                                h2 { a href="2021/11" { "November" } }
                                article {
                                    header {
                                        h3 {
                                            a href="/2021/11/09" {
                                                "Day 2: Enter Bevy & Shaders are hard"
                                            }
                                        }
                                        p { time datetime="2021-11-09" { "November 09, 2021" } }
                                    }
                                    p { "3 O’clock: departure. We are not entering the world of Bevy where we will actually make things happen. There’s no turning back now" }
                                }
                                article {
                                    header {
                                        h3 {
                                            a href="/2021/11/08" {
                                                "Day 1: Down the rabbit hole we go"
                                            }
                                        }
                                        p { time datetime="2021-11-08" { "November 08, 2021" } }
                                    }
                                    p { "Alice starts making games by watching trains with the loveliest coding conductor." }
                                }
                                article {
                                    header {
                                        h3 {
                                            a href="/2021/11/07" {
                                                "Day 0: Nannou, helping L, and lots of noise"
                                            }
                                        }
                                        p { time datetime="2021-11-07" { "November 07, 2021" } }
                                    }
                                    p { "Every journey starts with 1 O'clock: assistance. I just didn't know mine will also start with noise." }
                                }
                            }
                        }
                    }
                    footer {}
                }
            }
        }
        .into_string(),
    );
}
