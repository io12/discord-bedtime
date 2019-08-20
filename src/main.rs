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
    framework::{standard::Delimiter, StandardFramework},
    model::prelude::*,
    prelude::*,
};

/// Bot command prefix
pub static CMD_PREFIX: &'static str = "b,";

/// Reply to a message with the debug representation of `dbg`
fn say_dbg<T: fmt::Debug>(ctx: &Context, msg: &Message, dbg: T) {
    say(ctx, msg, format!("```{:#?}```", dbg))
}

/// If `res` holds an error, reply to a message with the error
fn say_if_err<T, E: fmt::Debug>(ctx: &Context, msg: &Message, res: Result<T, E>) {
    if let Err(err) = res {
        say_dbg(ctx, msg, err)
    }
}

/// Reply to a message with some content
pub fn say<T: fmt::Display>(ctx: &Context, msg: &Message, content: T) {
    if let Err(err) = msg.channel_id.say(&ctx.http, &content) {
        println!("Error saying message '{}': {}", content, err);
    }
}

/// Set client configuration options
fn config_client(client: &mut Client) {
    // Set up command framework
    client.with_framework(
        StandardFramework::new()
            .configure(|c| {
                c.prefix(CMD_PREFIX)
                    // Disable argument delimiters
                    .delimiters::<Delimiter, _>(iter::empty())
            })
            .group(&cmd::GENERAL_GROUP)
            .help(&cmd::HELP)
            .before(|_ctx, msg, cmd| {
                println!("Got command '{}' by user '{}'", cmd, msg.author.name);
                true
            })
            .after(|ctx, msg, _cmd, res| say_if_err(ctx, msg, res))
            .unrecognised_command(|ctx, msg, cmd| {
                let resp = format!("Command '{}' unrecognized", cmd);
                say(ctx, msg, resp);
            })
            .prefix_only(|ctx, msg| {
                say(ctx, msg, "Try the `help` sub-command for help.");
            }),
    );

    client_load_state(&client);
}

/// Load saved state from previous run, schedule bedtime alerts accordingly, and
/// store state in client context
fn client_load_state(client: &Client) {
    // Load state from previous run
    let state = State::load();

    // Schedule bedtime alerts
    let http = &client.cache_and_http.http;
    for (user_id, mut user_info) in state.users {
        let http = Arc::clone(http);
        user_info.update_sched(http, user_id);
    }

    // Store state in context
    client.data.write().insert::<State>(State::load());
}

fn main() {
    let tok = env::var("DISCORD_TOKEN").expect(
        "Bot token not specified. Please set the `DISCORD_TOKEN` \
         environment variable",
    );

    println!("Creating client...");
    let mut client = serenity::Client::new(&tok, Handler).expect("Couldn't create client");

    println!("Configuring client...");
    config_client(&mut client);

    println!("Starting client...");
    client.start().expect("Error running client");
}
