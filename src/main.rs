mod utils;

use anyhow::{Context, Result};
use diary_generator::{katex, Generator, Properties, EXPORT_DIR};
use notion_generator::client::NotionClient;
use std::path::Path;
use utils::spawn_copy_all;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect::<Vec<String>>();
    let auth_token = std::env::var("NOTION_TOKEN").context("Missing NOTION_TOKEN env variable")?;
    let database_id = args.get(1).context("Missing page id as first argument")?;

    tracing::subscriber::set_global_default(tracing_subscriber::FmtSubscriber::new())?;

    let client = NotionClient::new(auth_token);
    let pages = client.get_database_pages::<Properties>(database_id).await?;

    let generator = Generator::new(pages).await?;

    let (first_date, last_date) = match generator.get_first_and_last_dates() {
        Some(dates) => dates,
        None => return Ok(()),
    };

    let results = tokio::try_join!(
        katex::download(client.client().clone()),
        generator.generate_years(first_date, last_date)?,
        generator.generate_months(first_date, last_date)?,
        generator.generate_days()?,
        generator.generate_article_pages()?,
        generator.generate_index_page()?,
        generator.generate_articles_page()?,
        generator.generate_independent_pages(),
        spawn_copy_all(Path::new("public"), Path::new(EXPORT_DIR))
    )?;

    match results {
        (Err(error), _, _, _, _, _, _, _, _) => return Err(error),
        (_, Err(error), _, _, _, _, _, _, _) => return Err(error),
        (_, _, Err(error), _, _, _, _, _, _) => return Err(error),
        (_, _, _, Err(error), _, _, _, _, _) => return Err(error),
        (_, _, _, _, Err(error), _, _, _, _) => return Err(error),
        (_, _, _, _, _, Err(error), _, _, _) => return Err(error),
        (_, _, _, _, _, _, Err(error), _, _) => return Err(error),
        (_, _, _, _, _, _, _, Err(error), _) => return Err(error),
        (_, _, _, _, _, _, _, _, Err(error)) => return Err(error),
        (Ok(()), Ok(()), Ok(()), Ok(()), Ok(()), Ok(()), Ok(()), Ok(()), Ok(())) => {}
    };

    generator.download_all(client.client().clone()).await?;

    Ok(())
}
