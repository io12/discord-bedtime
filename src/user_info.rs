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
