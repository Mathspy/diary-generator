use anyhow::{Context, Result};
use async_recursion::async_recursion;
use futures_util::stream::{StreamExt, TryStreamExt};
use std::{io::ErrorKind, path::Path};
use tokio::{fs, task::JoinHandle};
use tokio_stream::wrappers::ReadDirStream;

#[async_recursion]
pub async fn copy_all<I, O>(input_dir: I, output_dir: O) -> Result<()>
where
    I: AsRef<Path> + Send,
    O: AsRef<Path> + Send,
{
    let input_dir = input_dir.as_ref();
    let output_dir = output_dir.as_ref();

    let files = fs::read_dir(input_dir).await;

    let files = match files {
        Ok(files) => files,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("Failed to read input directory {}", input_dir.display()))
        }
    };

    fs::create_dir_all(output_dir).await?;

    let files = ReadDirStream::new(files);

    files
        .map(|result| result.context("Failed to read file while recursively copying"))
        .and_then(|entry| async move {
            let file_name = entry.file_name();

            match entry.file_type().await? {
                file_type if file_type.is_dir() => {
                    copy_all(input_dir.join(&file_name), output_dir.join(&file_name)).await?;

                    Ok(())
                }
                _ => {
                    fs::copy(input_dir.join(&file_name), output_dir.join(&file_name)).await?;

                    Ok(())
                }
            }
        })
        .try_collect::<()>()
        .await?;

    Ok(())
}

pub fn spawn_copy_all(input: &'static Path, output: &'static Path) -> JoinHandle<Result<()>> {
    tokio::spawn(copy_all(input, output))
}
