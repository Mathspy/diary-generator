use serde::Deserialize;

mod deserializers {
    use super::LocaleConfig;
    use reqwest::Url;
    use serde::{
        de::{Deserializer, Error, Unexpected},
        Deserialize,
    };

    pub fn url<'a, D: Deserializer<'a>>(deserializer: D) -> Result<Option<Url>, D::Error> {
        Option::<String>::deserialize(deserializer)?
            .as_deref()
            .map(Url::parse)
            .transpose()
            .map_err(|error| D::Error::custom(error.to_string()))
    }

    pub(crate) fn locale<'a, D: Deserializer<'a>>(
        deserializer: D,
    ) -> Result<LocaleConfig, D::Error> {
        let locale = String::deserialize(deserializer)?;
        let mut locale_iter = locale.split('_');

        match (locale_iter.next(), locale_iter.next()) {
            (Some(lang), Some(_)) => Ok(LocaleConfig {
                lang: lang.to_string(),
                locale,
            }),
            _ => Err(D::Error::invalid_value(
                Unexpected::Str(&locale),
                &"a valid locale string",
            )),
        }
    }
}

#[derive(Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) author: Option<String>,
    pub(crate) cover: Option<String>,
    #[serde(deserialize_with = "deserializers::locale")]
    pub(crate) locale: LocaleConfig,
    #[serde(deserialize_with = "deserializers::url")]
    pub(crate) url: Option<reqwest::Url>,
    pub(crate) twitter: TwitterConfig,
}

#[derive(Clone)]
pub struct LocaleConfig {
    pub(crate) locale: String,
    pub(crate) lang: String,
}

#[derive(Clone, Deserialize)]
pub struct TwitterConfig {
    pub(crate) site: Option<String>,
    pub(crate) creator: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            name: "Diary".to_string(),
            description: "A neat diary".to_string(),
            author: None,
            cover: None,
            locale: LocaleConfig {
                locale: "en_US".to_string(),
                lang: "en".to_string(),
            },
            url: None,
            twitter: TwitterConfig {
                site: None,
                creator: None,
            },
        }
    }
}
