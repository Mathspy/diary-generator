use maud::Markup;

pub struct Feed<'a> {
    /// The title of the feed
    pub title: &'a str,
    /// The URL from which the diary itself will be served
    pub url: reqwest::Url,
    /// The URL from which the feed will be served from
    pub feed_url: reqwest::Url,
    /// The last time the feed was changed
    pub last_changed: time::OffsetDateTime,
    pub authors: Vec<Person<'a>>,

    // TODO: Diary generator doesn't currently support tags
    // categories: &'a [&'a str],
    /// The generator that is generating this feed
    pub generator: Generator,
    pub icon: Option<&'a str>,
    pub cover: Option<&'a str>,
    pub lang: &'a str,
    pub entries: Vec<Entry>,
}

pub struct Person<'a> {
    pub name: &'a str,
    pub email: Option<&'a str>,
    pub url: Option<reqwest::Url>,
}

pub struct Generator {
    pub value: &'static str,
    pub uri: &'static str,
    pub version: &'static str,
}

pub struct Entry {
    pub title: String,
    pub url: String,
    pub updated: time::OffsetDateTime,
    pub published: time::OffsetDateTime,
    // TODO: Should each entry have an author
    // TODO: tags AKA categories
    pub summary: String,
    pub content: Markup,
}
