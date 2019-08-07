mod cmd;

#[macro_use]
extern crate lazy_static;

use std::collections::HashMap;
use std::env;
use std::fmt;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::iter;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{
    atomic::{self, AtomicBool},
    Arc,
};
use std::thread::sleep;
use std::time::Duration;

use chrono::{naive::NaiveTime, prelude::*};
use chrono_tz::Tz;
use clokwerk::{ScheduleHandle, Scheduler, TimeUnits};
use serde::{Deserialize, Serialize};
use serenity::{
    framework::{standard::Delimiter, StandardFramework},
    http::raw::Http,
    model::{prelude::*, user::OnlineStatus},
    prelude::*,
};

lazy_static! {
    static ref STATE_PATH: PathBuf = {
        let path = env!("CARGO_MANIFEST_DIR");
        let path = PathBuf::from(path);
        let path = path.join("state.json");
        path
    };
}

/// Customized version of `chrono::naive::NaiveTime`
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Serialize, Deserialize)]
pub struct Time(NaiveTime);

impl Time {
    const FMT: &'static str = "%I:%M %p";
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.format(Self::FMT))
    }
}

impl FromStr for Time {
    type Err = chrono::format::ParseError;

    fn from_str(s: &str) -> chrono::format::ParseResult<Self> {
        NaiveTime::parse_from_str(s, Self::FMT).map(Time)
    }
}

/// Information stored about a user
#[derive(Serialize, Deserialize)]
pub struct UserInfo {
    on: bool,
    time_zone: Option<Tz>,
    bedtime: Option<Time>,
    #[serde(skip)]
    awake: Arc<AtomicBool>,
    #[serde(skip)]
    allowed_awake: Arc<AtomicBool>,
    #[serde(skip)]
    sched: Option<ScheduleHandle>,
}

impl Default for UserInfo {
    fn default() -> Self {
        Self {
            on: true,
            time_zone: None,
            bedtime: None,
            awake: Arc::new(AtomicBool::new(true)),
            allowed_awake: Arc::new(AtomicBool::new(true)),
            sched: None,
        }
    }
}

fn send_nag_msg_in_dm(http: impl AsRef<Http>, chan: PrivateChannel) {
    let res = chan.say(&http, "Go to bed. 😴 🛏  💤");
    if let Err(err) = res {
        println!("Error sending user sleep reminder: {}", err);
    }
}

fn send_nag_msg(http: impl AsRef<Http>, id: UserId) {
    println!("Nagging user '{}'", id);
    let res = id.create_dm_channel(&http);
    match res {
        Ok(dm) => send_nag_msg_in_dm(http, dm),
        Err(err) => println!("Error creating DM channel: {}", err),
    }
}

fn maybe_nag(http: impl AsRef<Http>, id: UserId, awake: Arc<AtomicBool>) {
    let awake = awake.load(atomic::Ordering::Relaxed);

    println!("User '{}' awake status: '{}'", id, awake);

    if awake {
        send_nag_msg(&http, id);
        sleep(Duration::from_secs(5));
    }
}

fn sched_bedtime(
    ctx: &Context,
    time_zone: Tz,
    bedtime: Time,
    id: UserId,
    awake: Arc<AtomicBool>,
    allowed_awake: Arc<AtomicBool>,
) -> ScheduleHandle {
    let mut sched = Scheduler::with_tz(time_zone);
    let http = Arc::clone(&ctx.http);
    println!("Scheduling bedtime for user '{}'", id);
    sched
        .every(1.day())
        .plus(bedtime.0.hour().hours())
        .plus(bedtime.0.minute().minutes())
        .run(move || {
            println!("Reached nag loop for user '{}'", id);
            allowed_awake.store(false, atomic::Ordering::Relaxed);
            loop {
                if allowed_awake.load(atomic::Ordering::Relaxed) {
                    break;
                }

                let awake = Arc::clone(&awake);

                maybe_nag(&http, id, awake);
            }
        });
    sched.watch_thread(Duration::from_secs(1))
}

impl UserInfo {
    fn update_sched(&mut self, ctx: &Context, id: UserId) {
        match self {
            UserInfo {
                on,
                time_zone: Some(time_zone),
                bedtime: Some(bedtime),
                awake,
                allowed_awake,
                ..
            } if *on => {
                let awake = Arc::clone(&awake);
                let allowed_awake = Arc::clone(&allowed_awake);

                let sched = sched_bedtime(ctx, *time_zone, *bedtime, id, awake, allowed_awake);
                self.sched = Some(sched);
            }
            _ => {
                self.sched = None;
            }
        }
    }

    pub fn set_time_zone(&mut self, ctx: &Context, id: UserId, time_zone: Tz) {
        self.time_zone = Some(time_zone);
        self.update_sched(ctx, id);
    }

    pub fn set_bedtime(&mut self, ctx: &Context, id: UserId, bedtime: Time) {
        self.bedtime = Some(bedtime);
        self.update_sched(ctx, id);
    }

    pub fn on(&mut self, ctx: &Context, id: UserId) {
        self.on = true;
        self.update_sched(ctx, id);
    }

    pub fn off(&mut self, ctx: &Context, id: UserId) {
        self.on = false;
        self.update_sched(ctx, id);
    }

    pub fn awake(&mut self) {
        self.awake.store(true, atomic::Ordering::Relaxed)
    }

    pub fn asleep(&mut self) {
        self.awake.store(false, atomic::Ordering::Relaxed)
    }
}

impl fmt::Display for UserInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let time_zone = match self.time_zone {
            Some(tz) => tz.name(),
            None => "none",
        };

        let bedtime = match self.bedtime {
            Some(bedtime) => bedtime.to_string(),
            None => "none".to_string(),
        };

        write!(
            f,
            "**on**: {}\n\
             **time zone**: {}\n\
             **bedtime**: {}",
            self.on, time_zone, bedtime
        )
    }
}

/// Bot state
#[derive(Default, Serialize, Deserialize)]
pub struct State {
    users: HashMap<UserId, UserInfo>,
}

impl State {
    fn save(&self) {
        let f = File::create(&*STATE_PATH);
        let f = f.expect("Failed to create state file");
        let f = BufWriter::new(f);
        let v = serde_json::to_writer(f, self);
        v.expect("Failed to write state");
    }

    fn load() -> Self {
        let f = File::open(&*STATE_PATH);
        match f {
            Ok(f) => {
                let f = BufReader::new(f);
                let v = serde_json::from_reader(f);
                v.expect("Failed to read state")
            }
            Err(_) => Self::default(),
        }
    }
}

/// Field of `serenity::prelude::Context::data`
impl TypeMapKey for State {
    type Value = State;
}

/// Serenity handler for bot. This implements `EventHandler` to process all the
/// bot events.
struct Handler;

impl EventHandler for Handler {
    fn ready(&self, _: Context, ready: Ready) {
        println!("{} is ready!", ready.user.name);
    }

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

fn say_dbg<T: fmt::Debug>(ctx: &Context, msg: &Message, dbg: T) {
    say(ctx, msg, format!("```{:#?}```", dbg))
}

fn say_if_err<T, E: fmt::Debug>(ctx: &Context, msg: &Message, res: Result<T, E>) {
    if let Err(err) = res {
        say_dbg(ctx, msg, err)
    }
}

fn say<T: fmt::Display>(ctx: &Context, msg: &Message, content: T) {
    if let Err(err) = msg.channel_id.say(&ctx.http, &content) {
        println!("Error saying message '{}': {}", content, err);
    }
}

fn config_client(client: &mut Client) {
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
