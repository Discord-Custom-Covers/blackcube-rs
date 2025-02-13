use anyhow::Context as AnyhowContext;
use poise::serenity_prelude::{
    all::{ButtonStyle, InteractionResponseFlags, MessageFlags},
    builder::{
        CreateActionRow, CreateButton, CreateEmbed, CreateInteractionResponse,
        CreateInteractionResponseFollowup, CreateInteractionResponseMessage, CreateMessage,
        EditMessage,
    },
    client::Context,
    model::{application::ComponentInteraction, channel::Message},
};

use crate::Context as PoiseContext;

pub async fn edit_request(
    ctx: &Context,
    msg: &mut Message,
    message: &str,
    thumbnail: Option<&str>,
    keep_components: bool,
) -> anyhow::Result<()> {
    let embed = &msg.embeds[0];
    let fields: Vec<(_, _, bool)> = embed
        .fields
        .iter()
        .map(|field| (field.name.clone(), field.value.clone(), field.inline))
        .collect();

    let mut components = vec![];

    if keep_components {
        components = vec![CreateActionRow::Buttons(vec![
            CreateButton::new("Approve")
                .style(ButtonStyle::Success)
                .label("Approve"),
            CreateButton::new("Deny")
                .style(ButtonStyle::Danger)
                .label("Deny"),
            CreateButton::new("Cancel")
                .style(ButtonStyle::Secondary)
                .label("Cancel"),
        ])];
    }

    let mut embed_builder = CreateEmbed::new().title(message).fields(fields);

    match thumbnail {
        Some(thumbnail) => {
            embed_builder = embed_builder.thumbnail(thumbnail);
        }
        None => {}
    }

    msg.edit(
        &ctx.http,
        EditMessage::new()
            .components(components)
            .embed(embed_builder),
    )
    .await?;
    Ok(())
}

pub async fn send_ephemeral_interaction_reply(
    ctx: &Context,
    component_interaction: ComponentInteraction,
    message: &str,
) -> anyhow::Result<()> {
    component_interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(message)
                    .flags(InteractionResponseFlags::EPHEMERAL),
            ),
        )
        .await
        .context("Could not create ephemeral response")?;
    Ok(())
}

pub async fn send_ephemeral_interaction_followup_reply(
    ctx: &Context,
    component_interaction: ComponentInteraction,
    message: &str,
) -> anyhow::Result<()> {
    component_interaction
        .create_followup(
            &ctx.http,
            CreateInteractionResponseFollowup::new()
                .content(message)
                .flags(MessageFlags::EPHEMERAL),
        )
        .await
        .context("Could not create ephemeral response")?;
    Ok(())
}

pub async fn create_request_log_message(
    ctx: PoiseContext<'_>,
    file_url: String,
) -> anyhow::Result<String> {
    let created_message = ctx
        .data()
        .config
        .server
        .log_channel_id
        .send_message(
            &ctx.http(),
            CreateMessage::new()
                .components(vec![CreateActionRow::Buttons(vec![
                    CreateButton::new("Approve")
                        .style(ButtonStyle::Success)
                        .label("Approve"),
                    CreateButton::new("Deny")
                        .style(ButtonStyle::Danger)
                        .label("Deny"),
                    CreateButton::new("Cancel")
                        .style(ButtonStyle::Secondary)
                        .label("Cancel"),
                ])])
                .embed(
                    CreateEmbed::new()
                        .title("Request Pending")
                        .field("User", ctx.author().name.clone(), true)
                        .field("UID", ctx.author().id.to_string(), true)
                        .thumbnail(file_url),
                ),
        )
        .await
        .context("Could not create request log message")?;
    Ok(created_message.link())
}
