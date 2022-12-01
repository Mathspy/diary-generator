mod utils;

use diary_generator::{Generator, Properties};
use notion_generator::response::{properties::DateProperty, Page};
use utils::{function, new_page, TestDir};

#[tokio::test]
async fn unpublished_pages_dont_cause_crashes() {
    let cwd = TestDir::new(function!());

    let page = new_page(
        "ac3fb543-001f-4be5-a25e-4978abd05b1d",
        "unpublished page with no date",
        "just a page without a publish date yet",
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
