use std::io::Cursor;

use image::ImageReader;
use poise::serenity_prelude as serenity;

use crate::{auth::HasAuth, responses::create_request_log_message, s3bucket::delete, Context};

/// Request a Background
#[poise::command(slash_command, ephemeral)]
pub async fn bg(
    ctx: Context<'_>,
    #[description = "Image"] file: serenity::Attachment,
) -> anyhow::Result<()> {
    if file.size > 10000000 {
        ctx.say("Image must be smaller than 10MB").await?;
        return Ok(());
    }

    // Screw you, Discord, for making me do this. PLEASE stop parsing your content type from file extensions.
    let file_data = file.download().await?;
    let content_type = ImageReader::new(Cursor::new(file_data))
        .with_guessed_format()?
        .format();

    if let Some(content_type) = content_type {
        if !ctx
            .data()
            .content_types
            .valid_content_types
            .contains(&content_type)
        {
            ctx.say(format!(
                "Image must be either {}. The image you uploaded is: {}.",
                ctx.data().content_types.concatenated_content_types,
                content_type.to_mime_type()
            ))
            .await?;
            return Ok(());
        }
    } else {
        ctx.say(format!(
            "Image must be either {}. Your file is not an image.",
            ctx.data().content_types.concatenated_content_types
        ))
        .await?;
        return Ok(());
    }

    let created_message_link = create_request_log_message(ctx, file.url).await?;
    ctx.say(format!("Created Request: {}", created_message_link))
        .await?;
    Ok(())
}

/// Remove a Background
#[poise::command(slash_command)]
pub async fn rm(
    ctx: Context<'_>,
    #[description = "User ID to remove: Leave blank to remove your own"] user: Option<String>,
) -> anyhow::Result<()> {
    match user {
        Some(user) => {
            if ctx
                .author()
                .has_auth(ctx.serenity_context(), ctx.data())
                .await?
                || ctx.author().id.to_string() == user
            {
                delete(ctx.data(), user.clone()).await?;
                ctx.send(
                    poise::CreateReply::default()
                        .content(format!("Removed Banner for {}", user))
                        .ephemeral(true),
                )
                .await?;
            } else {
                ctx.send(
                    poise::CreateReply::default()
                        .content("You do not have authorization to remove another user's banner")
                        .ephemeral(true),
                )
                .await?;
            }
        }
        None => {
            delete(ctx.data(), ctx.author().id.to_string()).await?;
            ctx.send(
                poise::CreateReply::default()
                    .content("Removed your banner")
                    .ephemeral(true),
            )
            .await?;
        }
    }
    Ok(())
}
