use anyhow::{bail, Context as AnyhowContext};
use poise::serenity_prelude as serenity;
use serenity::{
    builder::CreateInteractionResponse, client::Context, model::application::ComponentInteraction,
};

use crate::{
    auth::HasAuth,
    responses::{edit_request, send_ephemeral_interaction_reply},
    s3bucket::upload,
    structs::Data,
};

pub async fn handle_component_interaction(
    ctx: &Context,
    data: &Data,
    mut component_interaction: ComponentInteraction,
) -> anyhow::Result<()> {
    let has_auth = component_interaction
        .member
        .as_ref()
        .context("Could not retrieve user from interaction")?
        .has_auth(ctx, data)
        .await?;

    let embed = component_interaction
        .message
        .embeds
        .first()
        .context("Could not get first embed")?
        .clone();

    let image_url = embed
        .thumbnail
        .clone()
        .context("Error parsing image url")?
        .url;

    let mut uid: Option<String> = None;

    for field in &embed.fields {
        match field.name.as_str() {
            "UID" => {
                uid = Some(field.value.clone());
                break;
            }
            _ => {}
        }
    }

    let uid: String = uid.context("Could not parse uid from embed")?;

    match component_interaction.data.custom_id.as_str() {
        "Approve" => {
            if has_auth {
                component_interaction
                    .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
                    .await
                    .context("Could not acknowledge component interaction")?;

                edit_request(
                    ctx,
                    &mut component_interaction.message,
                    "Uploading...",
                    Some(&image_url),
                    false,
                )
                .await
                .context("Could not update message to show loading state")?;

                let s3bucket_url = upload(data, image_url.clone(), uid.clone())
                    .await
                    .context("Could not upload image to s3bucket")?;

                edit_request(
                    ctx,
                    &mut component_interaction.message,
                    "Request Approved",
                    Some(&s3bucket_url),
                    false,
                )
                .await
                .context("Could not edit request message")?;
            } else {
                send_ephemeral_interaction_reply(
                    ctx,
                    component_interaction.clone(),
                    "You must wait for a moderator to approve/deny this background",
                )
                .await
                .context("Could not notify user of lack of auth")?;
            }
        }
        "Deny" => {
            if has_auth {
                component_interaction
                    .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
                    .await
                    .context("Could not acknowledge component interaction")?;

                edit_request(
                    ctx,
                    &mut component_interaction.message,
                    "Request denied",
                    None,
                    false,
                )
                .await
                .context("Could not edit request message")?;
            } else {
                send_ephemeral_interaction_reply(
                    ctx,
                    component_interaction.clone(),
                    "You must wait for a moderator to approve/deny this background",
                )
                .await
                .context("Could not tell user to wait for moderator approval")?;
            }
        }
        "Cancel" => {
            if component_interaction.user.id.get() == uid.trim().parse::<u64>().unwrap() {
                component_interaction
                    .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
                    .await
                    .context("Could not acknowledge component interaction")?;

                edit_request(
                    ctx,
                    &mut component_interaction.message,
                    "Request Cancelled",
                    None,
                    false,
                )
                .await
                .context("Could not edit request message")?;
            } else {
                send_ephemeral_interaction_reply(
                    ctx,
                    component_interaction.clone(),
                    "You cannot cancel someone else's background request",
                )
                .await
                .context(
                    "Could not tell user they cannot cancel someone else's background request",
                )?;
            }
        }
        &_ => {
            bail!("Invalid component ID");
        }
    }
    Ok(())
}
