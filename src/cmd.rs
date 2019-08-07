use crate::state::State;

use std::collections::HashSet;

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
    commands: [time_zone, bedtime, info],
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
#[description = "Set your time zone"]
fn time_zone(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let tz = args.parse()?;

    let mut data = ctx.data.write();

    let state = data.get_mut::<State>().expect("No state in context");

    state
        .users
        .entry(msg.author.id)
        .or_default()
        .set_time_zone(ctx, msg.author.id, tz);

    state.save();

    let resp = format!("Your time zone has been set to {}", tz.name());

    msg.channel_id.say(&ctx.http, resp)?;

    Ok(())
}

#[command]
#[description = "Set your bedtime"]
fn bedtime(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let tm = args.parse()?;

    let mut data = ctx.data.write();

    let state = data.get_mut::<State>().expect("No state in context");

    state
        .users
        .entry(msg.author.id)
        .or_default()
        .set_bedtime(ctx, msg.author.id, tm);

    state.save();

    let resp = format!("Your bedtime has been set to {}", tm.to_string());

    msg.channel_id.say(&ctx.http, resp)?;

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
