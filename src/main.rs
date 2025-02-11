mod auth;
mod handlers;
mod responses;
mod s3bucket;
mod structs;

use auth::HasAuth;
use handlers::components::handle_component_interaction;
use image::ImageFormat;
use poise::serenity_prelude::{self as serenity, CacheHttp, Interaction};
use reqwest::Client;
use responses::{edit_request, send_ephemeral_interaction_followup_reply};
use s3bucket::connect_bucket;
use serenity::GatewayIntents;
use std::fs;
use structs::{Config, ContentTypes, Data};

type Context<'a> = poise::Context<'a, Data, anyhow::Error>;

#[tokio::main]
async fn main() {
    let config_file_location;

    match std::env::consts::OS {
        "linux" => {
            config_file_location = "/etc/blackcube-rs/blackcube-rs.toml";
        }
        "windows" => {
            config_file_location = "%SystemDrive%\\ProgramData\\blackcube-rs\\blackcube-rs.toml";
        }
        _ => {
            unreachable!();
        }
    }

    let config: Config = toml::from_str(
        &fs::read_to_string(config_file_location)
            .expect("Could not read configuration file, make sure the config is located at /etc/blackcube-rs/blackcube-rs.toml or %SystemDrive%\\ProgramData\\blackcube-rs\\blackcube-rs.toml")
    ).expect("could not read config");

    // Should I be doing all of this just to allow for changes to the valid image types from the config?? The jury is still out.
    let valid_content_types = config
        .settings
        .image_types
        .iter()
        .map(|image_type_string| {
            ImageFormat::from_extension(image_type_string)
                .expect("Could not parse valid image types from config")
        })
        .collect();

    let image_types = config
        .settings
        .image_types
        .split_last()
        .expect("Could not split image types vector");
    let concatenated_content_types = format!("{}, or {}", image_types.1.join(", "), image_types.0);

    let token = config.bot.discord_token.clone();

    let bucket = connect_bucket(&config)
        .await
        .expect("Could not initialize storage bucket connection");

    let http_client: Client = Client::new();

    let options = poise::FrameworkOptions {
        commands: vec![handlers::commands::bg(), handlers::commands::rm()],
        event_handler: |ctx, event, framework, data| {
            Box::pin(event_handler(ctx, event, framework, data))
        },
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .setup(move |ctx, ready, framework| {
            Box::pin(async move {
                println!("{} connected!", ready.user.name);
                poise::builtins::register_in_guild(
                    ctx,
                    &framework.options().commands,
                    config.server.guild_id.into(),
                )
                .await?;
                Ok(Data {
                    config,
                    content_types: ContentTypes {
                        valid_content_types,
                        concatenated_content_types,
                    },
                    http_client,
                    bucket,
                })
            })
        })
        .options(options)
        .build();

    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    client.unwrap().start().await.unwrap();
}

async fn event_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, anyhow::Error>,
    data: &Data,
) -> anyhow::Result<()> {
    match event {
        serenity::FullEvent::InteractionCreate { interaction } => match interaction {
            Interaction::Component(component_interaction) => {
                let result =
                    handle_component_interaction(ctx, data, component_interaction.clone()).await;
                if result.is_err() {
                    println!("{:?}", result);

                    let embed = component_interaction.message.embeds.first();

                    match embed {
                        Some(embed) => {
                            let embed = embed.clone();

                            let thumbnail;

                            match &embed.thumbnail {
                                Some(embed_thumbnail) => {
                                    thumbnail = Some(embed_thumbnail.url.as_str());
                                }
                                None => {
                                    thumbnail = None;
                                }
                            }

                            let result = edit_request(
                                &ctx,
                                &mut component_interaction.clone().message,
                                "Request Pending",
                                thumbnail,
                                true,
                            )
                            .await;
                            if result.is_err() {
                                println!("{:?}", result);
                            }
                        }
                        None => {}
                    }

                    let result = send_ephemeral_interaction_followup_reply(
                        &ctx,
                        component_interaction.clone(),
                        "Failed to accept request",
                    )
                    .await;
                    match result {
                        Ok(()) => {}
                        Err(err) => {
                            println!("{}", err);
                        }
                    }
                }
            }
            _ => {}
        },
        serenity::FullEvent::Message { new_message } => {
            if new_message.channel_id == data.config.server.request_channel_id && !new_message.author.has_auth(ctx, data).await? {
                new_message.delete(ctx.http()).await?;
            }
        }
        _ => {}
    }
    Ok(())
}
