pub mod cmd;
pub mod handler;
pub mod state;
pub mod time;
pub mod user_info;

#[macro_use]
extern crate lazy_static;

use handler::Handler;
use state::State;

use std::env;
use std::fmt;
use std::iter;
use std::sync::Arc;

use serenity::{
    framework::{
        standard::{macros::hook, CommandResult, Delimiter},
        StandardFramework,
    },
    model::{gateway::GatewayIntents, prelude::*},
    prelude::*,
    Result,
};

/// Bot command prefix
pub static CMD_PREFIX: &str = "b,";

/// Reply to a message with the debug representation of `dbg`
async fn say_dbg<T: fmt::Debug>(ctx: &Context, msg: &Message, dbg: T) {
    say(ctx, msg, format!("```{:#?}```", dbg)).await
}

/// If `res` holds an error, reply to a message with the error
async fn say_if_err(ctx: &Context, msg: &Message, res: &CommandResult) {
    if let Err(err) = res {
        say_dbg(ctx, msg, err).await
    }
}

/// Reply to a message with some content
pub async fn say<T: fmt::Display>(ctx: &Context, msg: &Message, content: T) {
    if let Err(err) = msg.channel_id.say(&ctx.http, &content).await {
        println!("Error saying message '{}': {}", content, err);
    }
}

#[hook]
async fn before_command_hook(_ctx: &Context, msg: &Message, cmd: &str) -> bool {
    println!("Got command '{}' by user '{}'", cmd, msg.author.name);
    true
}

#[hook]
async fn after_command_hook(ctx: &Context, msg: &Message, _cmd: &str, res: CommandResult) {
    say_if_err(ctx, msg, &res).await
}

#[hook]
async fn unrecognized_command_hook(ctx: &Context, msg: &Message, cmd: &str) {
    let resp = format!("Command '{}' unrecognized", cmd);
    say(ctx, msg, resp).await
}

#[hook]
async fn prefix_only_hook(ctx: &Context, msg: &Message) {
    say(ctx, msg, "Try the `help` sub-command for help.").await
}

async fn create_client(token: &str) -> Result<Client> {
    Client::builder(token, GatewayIntents::all())
        .event_handler(Handler)
        .framework(
            StandardFramework::new()
                .configure(|c| {
                    c.prefix(CMD_PREFIX)
                        // Disable argument delimiters
                        .delimiters::<Delimiter, _>(iter::empty())
                })
                .group(&cmd::GENERAL_GROUP)
                .help(&cmd::HELP)
                .before(before_command_hook)
                .after(after_command_hook)
                .unrecognised_command(unrecognized_command_hook)
                .prefix_only(prefix_only_hook),
        )
        .await
}

/// Load saved state from previous run, schedule bedtime alerts accordingly, and
/// store state in client context
async fn client_load_state(client: &Client) {
    // Load state from previous run
    let state = State::load();

    // Store state in context
    client.data.write().await.insert::<State>(state);

    // Schedule bedtime alerts

    let http = &client.cache_and_http.http;
    let mut map = client.data.write().await;
    let users = map
        .get_mut::<State>()
        .expect("No state in client")
        .users
        .iter_mut();

    for (&user_id, ref mut user_info) in users {
        let http = Arc::clone(http);
        user_info.update_sched(http, user_id).await;
    }
}

#[tokio::main]
async fn main() {
    let tok = env::var("DISCORD_TOKEN").expect(
        "Bot token not specified. Please set the `DISCORD_TOKEN` \
         environment variable",
    );

    println!("Creating client...");
    let mut client = create_client(&tok).await.expect("Couldn't create client");

    println!("Loading previous state...");
    client_load_state(&client).await;

    println!("Starting client...");
    client.start().await.expect("Error running client");
}
