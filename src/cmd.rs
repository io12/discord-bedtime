use crate::state::State;

use std::collections::HashSet;
use std::sync::Arc;

use serenity::{
    framework::standard::{
        help_commands,
        macros::{command, group, help},
        Args, CommandGroup, CommandResult, HelpOptions,
    },
    model::prelude::*,
    prelude::*,
};

#[group]
#[commands(time_zone, bedtime, wake, info, on, off)]
pub struct General;

#[help]
async fn help(
    ctx: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    help_commands::with_embeds(ctx, msg, args, help_options, groups, owners).await?;
    Ok(())
}

#[command]
#[description = "Set your time zone. List of options here: http://ix.io/1Rbm"]
async fn time_zone(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let tz = args.parse()?;

    let mut data = ctx.data.write().await;

    let state = data.get_mut::<State>().expect("No state in context");

    let http = &ctx.http;

    state
        .users
        .entry(msg.author.id)
        .or_default()
        .set_time_zone(Arc::clone(http), msg.author.id, tz)
        .await;

    state.save();

    let resp = format!("Your time zone has been set to {}", tz.name());

    msg.channel_id.say(http, resp).await?;

    Ok(())
}

#[command]
#[description = "Set your bedtime"]
async fn bedtime(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let tm = args.parse()?;

    let mut data = ctx.data.write().await;

    let state = data.get_mut::<State>().expect("No state in context");

    let http = &ctx.http;

    state
        .users
        .entry(msg.author.id)
        .or_default()
        .set_bedtime(Arc::clone(http), msg.author.id, tm)
        .await;

    state.save();

    let resp = format!("Your bedtime has been set to {}", tm);

    msg.channel_id.say(http, resp).await?;

    Ok(())
}

#[command]
#[description = "Tell the bot that you woke up for the day"]
async fn wake(ctx: &Context, msg: &Message) -> CommandResult {
    ctx.data
        .write()
        .await
        .get_mut::<State>()
        .expect("No state in context")
        .users
        .entry(msg.author.id)
        .or_default()
        .allow_awake();

    msg.channel_id.say(&ctx.http, "Good morning 🌅").await?;

    Ok(())
}

#[command]
#[description = "View your settings"]
async fn info(ctx: &Context, msg: &Message) -> CommandResult {
    let resp = ctx
        .data
        .write()
        .await
        .get_mut::<State>()
        .expect("No state in context")
        .users
        .entry(msg.author.id)
        .or_default()
        .to_string();

    msg.channel_id.say(&ctx.http, resp).await?;

    Ok(())
}

#[command]
#[description = "Enable sleep reminders"]
async fn on(ctx: &Context, msg: &Message) -> CommandResult {
    let mut data = ctx.data.write().await;

    let state = data.get_mut::<State>().expect("No state in context");

    let http = &ctx.http;

    state
        .users
        .entry(msg.author.id)
        .or_default()
        .on(Arc::clone(http), msg.author.id)
        .await;

    state.save();

    msg.channel_id.say(http, "Sleep reminders enabled").await?;

    Ok(())
}

#[command]
#[description = "Disable sleep reminders"]
async fn off(ctx: &Context, msg: &Message) -> CommandResult {
    let mut data = ctx.data.write().await;

    let state = data.get_mut::<State>().expect("No state in context");

    let http = &ctx.http;

    state
        .users
        .entry(msg.author.id)
        .or_default()
        .off(Arc::clone(http), msg.author.id)
        .await;

    state.save();

    msg.channel_id.say(http, "Sleep reminders disabled").await?;

    Ok(())
}
