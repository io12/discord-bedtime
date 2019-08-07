use crate::time::Time;

use std::fmt;
use std::sync::atomic;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

use chrono::Timelike;
use chrono_tz::Tz;
use clokwerk::{ScheduleHandle, Scheduler, TimeUnits};
use serde::{Deserialize, Serialize};
use serenity::{
    http::raw::Http,
    model::{channel::PrivateChannel, id::UserId},
    prelude::*,
};

/// User-specific state
#[derive(Serialize, Deserialize)]
pub struct UserInfo {
    /// Whether the user has bedtime alerts enabled
    on: bool,

    /// The user's time zone, if one is set
    time_zone: Option<Tz>,

    /// The user's bedtime, if one is set
    bedtime: Option<Time>,

    /// Whether the user is detected to be awake
    #[serde(skip)]
    awake: Arc<AtomicBool>,

    /// Whether the user is awake past their bedtime
    #[serde(skip)]
    allowed_awake: Arc<AtomicBool>,

    /// Handle used to manage bedtime alert scheduling
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

/// In the specified private channel, send a sleep reminder
fn send_nag_msg_in_dm(http: impl AsRef<Http>, chan: PrivateChannel) {
    let res = chan.say(&http, "Go to bed. 😴 🛏  💤");
    if let Err(err) = res {
        println!("Error sending user sleep reminder: {}", err);
    }
}

/// Send a sleep reminder direct message to a user
fn send_nag_msg(http: impl AsRef<Http>, id: UserId) {
    println!("Nagging user '{}'", id);
    let res = id.create_dm_channel(&http);
    match res {
        Ok(dm) => send_nag_msg_in_dm(http, dm),
        Err(err) => println!("Error creating DM channel: {}", err),
    }
}

/// Send a sleep reminder direct message to a user if the awake flag is set
fn maybe_nag(http: impl AsRef<Http>, id: UserId, awake: Arc<AtomicBool>) {
    let awake = awake.load(atomic::Ordering::Relaxed);

    println!("User '{}' awake status: '{}'", id, awake);

    if awake {
        send_nag_msg(&http, id);
        sleep(Duration::from_secs(5));
    }
}

/// Schedule bedtime alerts for a user
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
    /// Update user's bedtime alert schedule based on their settings
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

    /// Set user's time zone
    pub fn set_time_zone(&mut self, ctx: &Context, id: UserId, time_zone: Tz) {
        self.time_zone = Some(time_zone);
        self.update_sched(ctx, id);
    }

    /// Set user's bedtime
    pub fn set_bedtime(&mut self, ctx: &Context, id: UserId, bedtime: Time) {
        self.bedtime = Some(bedtime);
        self.update_sched(ctx, id);
    }

    /// Enable sleep alerts for user
    pub fn on(&mut self, ctx: &Context, id: UserId) {
        self.on = true;
        self.update_sched(ctx, id);
    }

    /// Disable sleep alerts for user
    pub fn off(&mut self, ctx: &Context, id: UserId) {
        self.on = false;
        self.update_sched(ctx, id);
    }

    /// Set user awake flag
    pub fn awake(&mut self) {
        self.awake.store(true, atomic::Ordering::Relaxed)
    }

    /// Unset user awake flag
    pub fn asleep(&mut self) {
        self.awake.store(false, atomic::Ordering::Relaxed)
    }
}

impl fmt::Display for UserInfo {
    /// Pretty-print user-specific state
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
