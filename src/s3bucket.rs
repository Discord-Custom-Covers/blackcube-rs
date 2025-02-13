use std::{
    io::Cursor,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{bail, Context as AnyhowContext};
use image::ImageReader;
use s3::{creds::Credentials, Bucket, Region};

use crate::structs::{Config, Data};

pub async fn connect_bucket(config: &Config) -> Result<Bucket, anyhow::Error> {
    let region = Region::Custom {
        region: "us-east-1".to_owned(),
        endpoint: config.storage.url.to_owned(),
    };

    let credentials = Credentials::new(
        Some(&config.storage.access_key),
        Some(&config.storage.secret_key),
        None,
        None,
        None,
    )?;

    let bucket = Bucket::new(
        &config.storage.bucket_name,
        region.clone(),
        credentials.clone(),
    )?
    .with_path_style();

    Ok(*bucket)
}

pub async fn upload(data: &Data, image_url: String, uid: String) -> Result<String, anyhow::Error> {
    let http_client = &data.http_client;

    let response = http_client.get(image_url.clone()).send().await?;
    let image_bytes = response.bytes().await?;

    let content_type = ImageReader::new(Cursor::new(&image_bytes))
        .with_guessed_format()?
        .format()
        .context("Could not parse image format")?
        .to_mime_type();

    let config = &data.config;
    let bucket = &data.bucket;

    let path = format!("{}{}", config.storage.storage_path, uid);

    let response = bucket
        .put_object_with_content_type(path.clone(), &image_bytes, content_type)
        .await?;

    if response.status_code() != 200 {
        bail!("Error uploading image to minio")
    }

    let extension = content_type
        .split("/")
        .last()
        .context("Could not parse extension from content type")?;

    Ok(format!(
        "{}/{}{}{}?{}.{}",
        config.storage.url,
        config.storage.bucket_name,
        config.storage.storage_path,
        uid,
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
        extension
    ))
}

pub async fn delete(data: &Data, uid: String) -> Result<(), anyhow::Error> {
    let config = &data.config;
    let bucket = &data.bucket;

    let path = format!("{}{}", config.storage.storage_path, uid);

    let response = bucket.delete_object(path.clone()).await?;

    if response.status_code() != 204 {
        bail!("Error deleting image from minio")
    }

    Ok(())
}
