mod utils;

use std::fs;

use diary_generator::{Generator, Properties};
use maud::{html, DOCTYPE};
use notion_generator::response::{properties::DateProperty, Page};
use pretty_assertions::assert_eq;
use utils::{function, new_entry, DirEntry, TestDir};

#[tokio::test]
async fn unpublished_pages_dont_cause_crashes() {
    let cwd = TestDir::new(function!());

    let page = new_entry(
        "ac3fb543-001f-4be5-a25e-4978abd05b1d",
        "unpublished page with no date",
        "just a page without a publish date yet",
        None,
        None,
    );

    Generator::new(
        &cwd,
        vec![Page {
            properties: Properties {
                date: DateProperty {
                    date: None,
                    ..page.properties.date
                },
                published: DateProperty {
                    date: None,
                    ..page.properties.published
                },
                ..page.properties
            },
            ..page
        }],
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn able_to_locate_partials() {
    let cwd = TestDir::new(function!());
    let partials_dir = cwd.path().join("partials");

    fs::create_dir_all(&partials_dir).unwrap();
    fs::write(
        partials_dir.join("head.html"),
        r#"<link rel="icon" href="/favicon.ico" sizes="any">"#,
    )
    .unwrap();
    fs::write(
        partials_dir.join("header.html"),
        r#"<a href="/">Homepage</a>"#,
    )
    .unwrap();
    fs::write(
        partials_dir.join("footer.html"),
        r#"<a href="/feed.xml">Feed</a>"#,
    )
    .unwrap();

    let generator = Generator::new(&cwd, vec![]).await.unwrap();

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
            [
                DirEntry::dir("output", [DirEntry::file("index.html")]),
                DirEntry::dir(
                    "partials",
                    [
                        DirEntry::file("head.html"),
                        DirEntry::file("header.html"),
                        DirEntry::file("footer.html")
                    ]
                )
            ]
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
                    link rel="icon" href="/favicon.ico" sizes="any";
                }
                body {
                    header {
                        a href="/" { "Homepage" }
                    }
                    main {}
                    footer {
                        a href="/feed.xml" { "Feed" }
                    }
                }
            }
        }
        .into_string(),
    );
}
