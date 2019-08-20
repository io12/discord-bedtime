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

group!({
    name: "general",
    commands: [time_zone, bedtime, wake, info, on, off],
});

#[help]
fn help(
    ctx: &mut Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    help_commands::with_embeds(ctx, msg, args, help_options, groups, owners)
}

#[command]
#[description = "Set your time zone. List of options here: http://ix.io/1Rbm"]
fn time_zone(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let tz = args.parse()?;

    let mut data = ctx.data.write();

    let state = data.get_mut::<State>().expect("No state in context");

    let http = &ctx.http;

    state.users.entry(msg.author.id).or_default().set_time_zone(
        Arc::clone(http),
        msg.author.id,
        tz,
    );

    state.save();

    let resp = format!("Your time zone has been set to {}", tz.name());

    msg.channel_id.say(http, resp)?;

    Ok(())
}

#[command]
#[description = "Set your bedtime"]
fn bedtime(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let tm = args.parse()?;

    let mut data = ctx.data.write();

    let state = data.get_mut::<State>().expect("No state in context");

    let http = &ctx.http;

    state
        .users
        .entry(msg.author.id)
        .or_default()
        .set_bedtime(Arc::clone(http), msg.author.id, tm);

    state.save();

    let resp = format!("Your bedtime has been set to {}", tm.to_string());

    msg.channel_id.say(http, resp)?;

    Ok(())
}

#[command]
#[description = "Tell the bot that you woke up for the day"]
fn wake(ctx: &mut Context, msg: &Message) -> CommandResult {
    ctx.data
        .write()
        .get_mut::<State>()
        .expect("No state in context")
        .users
        .entry(msg.author.id)
        .or_default()
        .allow_awake();

    msg.channel_id.say(&ctx.http, "Good morning 🌅")?;

    Ok(())
}

#[command]
#[description = "View your settings"]
fn info(ctx: &mut Context, msg: &Message) -> CommandResult {
    let resp = ctx
        .data
        .write()
        .get_mut::<State>()
        .expect("No state in context")
        .users
        .entry(msg.author.id)
        .or_default()
        .to_string();

    msg.channel_id.say(&ctx.http, resp)?;

    Ok(())
}

#[command]
#[description = "Enable sleep reminders"]
fn on(ctx: &mut Context, msg: &Message) -> CommandResult {
    let mut data = ctx.data.write();

    let state = data.get_mut::<State>().expect("No state in context");

    let http = &ctx.http;

    state
        .users
        .entry(msg.author.id)
        .or_default()
        .on(Arc::clone(http), msg.author.id);

    state.save();

    msg.channel_id.say(http, "Sleep reminders enabled")?;

    Ok(())
}

#[command]
#[description = "Disable sleep reminders"]
fn off(ctx: &mut Context, msg: &Message) -> CommandResult {
    let mut data = ctx.data.write();

    let state = data.get_mut::<State>().expect("No state in context");

    let http = &ctx.http;

    state
        .users
        .entry(msg.author.id)
        .or_default()
        .off(Arc::clone(http), msg.author.id);

    state.save();

    msg.channel_id.say(http, "Sleep reminders disabled")?;

    Ok(())
}
