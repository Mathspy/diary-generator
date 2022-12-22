mod utils;

use diary_generator::Generator;
use maud::{html, DOCTYPE};
use pretty_assertions::assert_eq;
use std::fs;
use utils::{function, new_entry, DirEntry, TestDir};

#[tokio::test]
async fn empty_index() {
    let cwd = TestDir::new(function!());

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
    let cwd = TestDir::new(function!());

    let generator = Generator::new(
        &cwd,
        vec![
            new_entry(
                "cf2bacc9d75c4226aab53601c336f295",
                "Day 0: Nannou, helping L, and lots of noise",
                "Every journey starts with 1 O'clock: assistance. \
I just didn't know mine will also start with noise.",
                Some("2021-11-07".parse().unwrap()),
                None,
            ),
            new_entry(
                "ac3fb543001f4be5a25e4978abd05b1d",
                "Day 1: Down the rabbit hole we go",
                "Alice starts making games by watching trains with the loveliest coding conductor.",
                Some("2021-11-08".parse().unwrap()),
                None,
            ),
            new_entry(
                "ac3fb543001f4be5a25e4978abd05b1d",
                "Day 2: Enter Bevy & Shaders are hard",
                "3 O’clock: departure. \
We are not entering the world of Bevy where we will actually make things happen. \
There’s no turning back now",
                Some("2021-11-09".parse().unwrap()),
                None,
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

#[tokio::test]
async fn with_config_url() {
    let cwd = TestDir::new(function!());

    fs::write(
        cwd.path().join("config.json"),
        r#"
            {
              "url": "https://gamediary.dev"
            }
        "#,
    )
    .unwrap();

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
            [
                DirEntry::file("config.json"),
                DirEntry::dir("output", [DirEntry::file("index.html")])
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
                    link rel="alternate" type="application/atom+xml" href="/feed.xml";
                    meta property="og:title" content="Diary";
                    meta property="og:description" content="A neat diary";
                    meta property="og:locale" content="en_US";
                    meta property="og:url" content="https://gamediary.dev/";
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
