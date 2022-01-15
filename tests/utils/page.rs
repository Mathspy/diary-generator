use diary_generator::Properties;
use either::Either;
use notion_generator::response::{
    properties::{DateProperty, RichTextProperty, TitleProperty},
    NotionDate, Page, PageParent, RichText, RichTextType, Time,
};
use time::macros::date;

pub fn new(id: &str, title: &str, date: Time, description: &str) -> Page<Properties> {
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
