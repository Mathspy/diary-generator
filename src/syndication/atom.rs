use maud::{html, Markup, Render};
use time::format_description::well_known::Rfc3339;

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

enum LinkType {
    Alternate,
    Self_,
}

struct Link<'a> {
    ty: LinkType,
    href: &'a str,
}

struct XmlDoc;

impl Render for XmlDoc {
    fn render_to(&self, buffer: &mut String) {
        buffer.push_str(r#"<?xml version="1.0" encoding="utf-8" ?>"#);
    }
}

impl<'a> Render for Feed<'a> {
    fn render(&self) -> Markup {
        html! {
            (XmlDoc)
            feed xmlns="http://www.w3.org/2005/Atom" xml:lang=(self.lang) {
                id { (self.url) }
                title { (self.title) }
                updated { (self.last_changed.format(&Rfc3339).unwrap()) }

                @for author in &self.authors {
                    (*author)
                }

                (self.generator)

                (Link {
                    href: self.feed_url.as_str(),
                    ty: LinkType::Self_
                })
                (Link {
                    href: self.url.as_str(),
                    ty: LinkType::Alternate
                })

                @if let Some(icon) = self.icon {
                    icon { (icon) }
                }

                @if let Some(cover) = self.cover {
                    logo { (cover) }
                }

                @for entry in &self.entries {
                    (*entry)
                }
           }
        }
    }
}

impl<'a> Render for Person<'a> {
    fn render(&self) -> Markup {
        html! {
            author {
                name { (self.name) }

                @if let Some(email) = self.email {
                    email { (email) }
                }

                @if let Some(url) = &self.url {
                    uri { (url) }
                }
            }
        }
    }
}

impl Render for Generator {
    fn render(&self) -> Markup {
        html! {
            generator uri=(self.uri) version=(self.version) {
                (self.value)
            }
        }
    }
}

impl Render for Entry {
    fn render(&self) -> Markup {
        html! {
            entry {
                id { (self.url) }
                title type="html" { (self.title) }
                updated { (self.updated.format(&Rfc3339).unwrap()) }
                published { (self.published.format(&Rfc3339).unwrap()) }
                summary { (self.summary) }
                content type="html" { (self.content.0) }
            }
        }
    }
}

impl Render for LinkType {
    fn render_to(&self, buffer: &mut String) {
        match self {
            LinkType::Alternate => buffer.push_str("alternate"),
            LinkType::Self_ => buffer.push_str("self"),
        }
    }
}

impl<'a> Render for Link<'a> {
    fn render_to(&self, buffer: &mut String) {
        // In case of alternate which is the longer of the two link types the non-href parts of the
        // link is 32 1-byte characters long
        buffer.reserve(32 + self.href.len());
        buffer.push_str("<link ");

        buffer.push_str("rel=");
        buffer.push('"');
        self.ty.render_to(buffer);
        buffer.push_str(r#"" "#);

        buffer.push_str("href=");
        buffer.push('"');
        self.href.render_to(buffer);
        buffer.push_str(r#"" "#);

        buffer.push_str("/>")
    }
}

#[cfg(test)]
mod tests {
    use super::{Link, LinkType};
    use maud::Render;

    #[test]
    fn links_render() {
        assert_eq!(
            Link {
                href: "https://gamediary.dev/feed.xml",
                ty: LinkType::Self_
            }
            .render()
            .into_string(),
            r#"<link rel="self" href="https://gamediary.dev/feed.xml" />"#
        );

        assert_eq!(
            Link {
                href: "https://gamediary.dev",
                ty: LinkType::Alternate
            }
            .render()
            .into_string(),
            r#"<link rel="alternate" href="https://gamediary.dev" />"#
        );
    }
}
