mod utils;

use diary_generator::Generator;
use either::Either;
use notion_generator::response::{Block, BlockType, Page, RichText, RichTextType, Time};
use pretty_assertions::assert_eq;
use std::{fs, io::Cursor};
use time::macros::date;
use utils::{function, new_article, new_entry, DirEntry, TestDir};
use xml::reader::XmlEvent;

fn xml_string_to_events(xml: &str) -> Vec<XmlEvent> {
    xml::EventReader::new(Cursor::new(xml.as_bytes()))
        .into_iter()
        .filter_map(|event| match event {
            Ok(XmlEvent::Whitespace(_)) => None,
            Ok(XmlEvent::Characters(characters)) => {
                Some(Ok(XmlEvent::Characters(characters.trim().to_owned())))
            }
            _ => Some(event),
        })
        .collect::<Result<_, _>>()
        .unwrap()
}

#[tokio::test]
async fn plentiful_configurations() {
    let cwd = TestDir::new(function!());

    fs::write(
        cwd.path().join("config.json"),
        r#"
            {
              "name": "Game Dev Diary",
              "description": "A really cool diary",
              "author": {
                "name": "Mathspy",
                "url": "https://mathspy.me"
              },
              "cover": "/media/cover.png",
              "locale": "en_US",
              "url": "https://gamediary.dev"
            }
        "#,
    )
    .unwrap();

    let generator = Generator::new(
        &cwd,
        vec![new_article(
            "78abd05b1dac3fb543001f4be5a25e49",
            "Some article about something",
            "some really interesting descritpion",
            "interesting_article",
            Some(date!(2021 - 12 - 08)),
        )],
    )
    .await
    .unwrap();
    generator
        .generate_atom_feed()
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
                DirEntry::dir("output", [DirEntry::file("feed.xml")])
            ]
        ),
    );

    assert_eq!(
        xml_string_to_events(
            &fs::read_to_string(cwd.path().join("output").join("feed.xml")).unwrap()
        ),
        xml_string_to_events(
            r##"
<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom" xml:lang="en">
   <id>https://gamediary.dev/</id>
   <title>Game Dev Diary</title>
   <updated>2021-12-08T00:00:00Z</updated>
   <author>
      <name>Mathspy</name>
      <uri>https://mathspy.me/</uri>
   </author>
   <generator uri="https://github.com/Mathspy/diary-generator" version="0.3.0">diary-generator</generator>
   <link rel="self" href="https://gamediary.dev/" />
   <link rel="alternate" href="https://gamediary.dev/feed.xml" />
   <logo>/media/cover.png</logo>
   <entry>
      <id>https://gamediary.dev/interesting_article</id>
      <title type="html">Some article about something</title>
      <updated>2021-12-06T09:25:00Z</updated>
      <published>2021-12-08T00:00:00Z</published>
      <summary>some really interesting descritpion</summary>
      <content type="html" />
   </entry>
</feed>
"##
        ),
    );
}

#[tokio::test]
async fn can_create_feed_from_articles_and_entries() {
    let cwd = TestDir::new(function!());

    fs::write(
        cwd.path().join("config.json"),
        r#"{"url": "https://example.com"}"#,
    )
    .unwrap();

    let generator = Generator::new(
        &cwd,
        vec![
            new_entry(
                "cf2bacc9d75c4226aab53601c336f295",
                "Day 0: Nannou, helping L, and lots of noise",
                "Every journey starts with 1 O'clock: assistance. \
I just didn't know mine will also start with noise.",
                Some(Time {
                    original: "2021-11-07".to_string(),
                    parsed: Either::Left(date!(2021 - 11 - 07)),
                }),
                Some(date!(2021-12-05))
            ),
            Page {
                children: vec![
                    Block {
                        object: "block".to_string(),
                        id: "4fb9dd79-2fc7-45b1-b3a2-8efae49992ed".parse().unwrap(),
                        created_time: "2021-11-15T18:03:00.000Z".to_string(),
                        last_edited_time: "2021-11-16T11:23:00.000Z".to_string(),
                        has_children: true,
                        archived: false,
                        ty: BlockType::Paragraph {
                            text: vec![
                                RichText {
                                    plain_text: "You can also create these rather interesting nested paragraphs".to_string(),
                                    href: None,
                                    annotations: Default::default(),
                                    ty: RichTextType::Text {
                                        content: "You can also create these rather interesting nested paragraphs".to_string(),
                                        link: None,
                                    },
                                },
                            ],
                            children: vec![
                                Block {
                                    object: "block".to_string(),
                                    id: "817c0ca1-721a-4565-ac54-eedbbe471f0b".parse().unwrap(),
                                    created_time: "2021-11-16T11:23:00.000Z".to_string(),
                                    last_edited_time: "2021-11-16T11:23:00.000Z".to_string(),
                                    has_children: false,
                                    archived: false,
                                    ty: BlockType::Paragraph {
                                        text: vec![
                                            RichText {
                                                plain_text: "Possibly more than once too!".to_string(),
                                                href: None,
                                                annotations: Default::default(),
                                                ty: RichTextType::Text {
                                                    content: "Possibly more than once too!".to_string(),
                                                    link: None,
                                                },
                                            },
                                        ],
                                        children: vec![],
                                    },
                                },
                            ],
                        },
                    },
                ],
                ..new_entry(
                    "ac3fb543001f4be5a25e4978abd05b1d",
                    "Day 1: Down the rabbit hole we go",
                    "Alice starts making games by watching trains with the loveliest coding conductor.",
                    Some(Time {
                        original: "2021-11-08".to_string(),
                        parsed: Either::Left(date!(2021 - 11 - 08)),
                    }),
                Some(date!(2021-12-07))
                )
            },
            new_entry(
                "ac3fb543001f4be5a25e4978abd05b1d",
                "Day 2: Enter Bevy & Shaders are hard",
                "3 O’clock: departure. \
We are not entering the world of Bevy where we will actually make things happen. \
There’s no turning back now",
                Some(Time {
                    original: "2021-11-09".to_string(),
                    parsed: Either::Left(date!(2021 - 11 - 09)),
                }),
                Some(date!(2021-12-09))
            ),
            new_article(
                "78abd05b1dac3fb543001f4be5a25e49",
                "Some article about something",
                "some really interesting descritpion",
                "interesting_article",
                Some(date!(2021-12-08))
            )
        ],
    )
    .await
    .unwrap();
    generator
        .generate_atom_feed()
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
                DirEntry::dir("output", [DirEntry::file("feed.xml")])
            ]
        ),
    );

    assert_eq!(
        xml_string_to_events(
            &fs::read_to_string(cwd.path().join("output").join("feed.xml")).unwrap()
        ),
        xml_string_to_events(
            r##"
<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom" xml:lang="en">
   <id>https://example.com/</id>
   <title>Diary</title>
   <updated>2021-12-09T00:00:00Z</updated>
   <generator uri="https://github.com/Mathspy/diary-generator" version="0.3.0">diary-generator</generator>
   <link rel="self" href="https://example.com/" />
   <link rel="alternate" href="https://example.com/feed.xml" />
   <entry>
      <id>/2021/11/07</id>
      <title type="html">Day 0: Nannou, helping L, and lots of noise</title>
      <updated>2021-12-06T09:25:00Z</updated>
      <published>2021-12-05T00:00:00Z</published>
      <summary>Every journey starts with 1 O'clock: assistance. I just didn't know mine will also start with noise.</summary>
      <content type="html" />
   </entry>
   <entry>
      <id>/2021/11/08</id>
      <title type="html">Day 1: Down the rabbit hole we go</title>
      <updated>2021-12-06T09:25:00Z</updated>
      <published>2021-12-07T00:00:00Z</published>
      <summary>Alice starts making games by watching trains with the loveliest coding conductor.</summary>
      <content type="html">&lt;div id="4fb9dd792fc745b1b3a28efae49992ed"&gt;&lt;p&gt;You can also create these rather interesting nested paragraphs&lt;/p&gt;&lt;p id="817c0ca1721a4565ac54eedbbe471f0b" class="indent"&gt;Possibly more than once too!&lt;/p&gt;&lt;/div&gt;</content>
   </entry>
   <entry>
      <id>https://example.com/interesting_article</id>
      <title type="html">Some article about something</title>
      <updated>2021-12-06T09:25:00Z</updated>
      <published>2021-12-08T00:00:00Z</published>
      <summary>some really interesting descritpion</summary>
      <content type="html" />
   </entry>
   <entry>
      <id>/2021/11/09</id>
      <title type="html">Day 2: Enter Bevy &amp; Shaders are hard</title>
      <updated>2021-12-06T09:25:00Z</updated>
      <published>2021-12-09T00:00:00Z</published>
      <summary>3 O’clock: departure. We are not entering the world of Bevy where we will actually make things happen. There’s no turning back now</summary>
      <content type="html" />
   </entry>
</feed>
"##
        ),
    );
}
