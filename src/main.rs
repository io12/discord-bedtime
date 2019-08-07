pub mod cmd;
pub mod state;
pub mod time;
pub mod user_info;

#[macro_use]
extern crate lazy_static;

use state::State;

use std::{env, fmt, iter};

use serenity::{
    framework::{standard::Delimiter, StandardFramework},
    model::{prelude::*, user::OnlineStatus},
    prelude::*,
};

/// Serenity handler for bot. This implements `EventHandler` to process all the
/// bot events.
struct Handler;

/// Implementation of event handler
impl EventHandler for Handler {
    /// Print a log message when the bot is ready
    fn ready(&self, _: Context, ready: Ready) {
        println!("{} is ready!", ready.user.name);
    }

    /// When a user's presence updates, flag the user as either awake or asleep,
    /// depending on the new online status
    fn presence_update(&self, ctx: Context, ev: PresenceUpdateEvent) {
        let mut data = ctx.data.write();
        let user_info = data
            .get_mut::<State>()
            .expect("No state in context")
            .users
            .entry(ev.presence.user_id)
            .or_default();

        match ev.presence.status {
            OnlineStatus::Offline => user_info.asleep(),
            _ => user_info.awake(),
        }
    }
}

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
fn say<T: fmt::Display>(ctx: &Context, msg: &Message, content: T) {
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
                c.prefix("!bed")
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

    // Load state from previous run and store in the context
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
