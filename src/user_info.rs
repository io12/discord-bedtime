use crate::time::Time;

use std::fmt;
use std::sync::atomic;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

use chrono::Timelike;
use chrono_tz::Tz;
use clokwerk::{AsyncScheduler, Job, TimeUnits};
use serde::{Deserialize, Serialize};
use serenity::{
    http::{CacheHttp, Http},
    model::{channel::PrivateChannel, id::UserId},
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
    sched: Option<tokio::task::JoinHandle<()>>,
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
async fn send_nag_msg_in_dm(http: impl AsRef<Http>, chan: PrivateChannel) {
    let res = chan.say(&http, "Go to bed. 😴 🛏  💤").await;
    if let Err(err) = res {
        println!("Error sending user sleep reminder: {}", err);
    }
}

/// Send a sleep reminder direct message to a user
async fn send_nag_msg(cache_http: impl CacheHttp, id: UserId) {
    println!("Nagging user '{}'", id);
    let res = id.create_dm_channel(&cache_http).await;
    match res {
        Ok(dm) => send_nag_msg_in_dm(cache_http.http(), dm).await,
        Err(err) => println!("Error creating DM channel: {}", err),
    }
}

/// Send a sleep reminder direct message to a user if the awake flag is set
async fn maybe_nag(cache_http: impl CacheHttp, id: UserId, awake: Arc<AtomicBool>) {
    let awake = awake.load(atomic::Ordering::Relaxed);

    println!("User '{}' awake status: '{}'", id, awake);

    if awake {
        send_nag_msg(cache_http, id).await;
        sleep(Duration::from_secs(5));
    }
}

async fn nag_loop(
    http: Arc<Http>,
    id: UserId,
    awake: Arc<AtomicBool>,
    allowed_awake: Arc<AtomicBool>,
) {
    println!("Reached nag loop for user '{}'", id);
    allowed_awake.store(false, atomic::Ordering::Relaxed);
    loop {
        if allowed_awake.load(atomic::Ordering::Relaxed) {
            break;
        }

        let awake = Arc::clone(&awake);

        maybe_nag(&http, id, awake).await;
    }
}

/// Schedule bedtime alerts for a user
async fn sched_bedtime(
    http: Arc<Http>,
    time_zone: Tz,
    bedtime: Time,
    id: UserId,
    awake: Arc<AtomicBool>,
    allowed_awake: Arc<AtomicBool>,
) -> tokio::task::JoinHandle<()> {
    let mut sched = AsyncScheduler::with_tz(time_zone);
    let http = Arc::clone(&http);
    println!("Scheduling bedtime for user '{}'", id);
    sched
        .every(1.day())
        .plus(bedtime.0.hour().hours())
        .plus(bedtime.0.minute().minutes())
        .run(move || nag_loop(http.clone(), id, awake.clone(), allowed_awake.clone()));
    tokio::spawn(async move {
        loop {
            sched.run_pending().await;
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    })
}

impl UserInfo {
    /// Update user's bedtime alert schedule based on their settings
    pub async fn update_sched(&mut self, http: Arc<Http>, id: UserId) {
        if let Some(sched) = &self.sched {
            sched.abort()
        }
        match self {
            UserInfo {
                on,
                time_zone: Some(time_zone),
                bedtime: Some(bedtime),
                awake,
                allowed_awake,
                ..
            } if *on => {
                let awake = Arc::clone(awake);
                let allowed_awake = Arc::clone(allowed_awake);

                let sched =
                    sched_bedtime(http, *time_zone, *bedtime, id, awake, allowed_awake).await;
                self.sched = Some(sched);
            }
            _ => {
                self.sched = None;
            }
        }
    }

    /// Set user's time zone
    pub async fn set_time_zone(&mut self, http: Arc<Http>, id: UserId, time_zone: Tz) {
        self.time_zone = Some(time_zone);
        self.update_sched(http, id).await;
    }

    /// Set user's bedtime
    pub async fn set_bedtime(&mut self, http: Arc<Http>, id: UserId, bedtime: Time) {
        self.bedtime = Some(bedtime);
        self.update_sched(http, id).await;
    }

    /// Enable sleep alerts for user
    pub async fn on(&mut self, http: Arc<Http>, id: UserId) {
        self.on = true;
        self.update_sched(http, id).await;
    }

    /// Disable sleep alerts for user
    pub async fn off(&mut self, http: Arc<Http>, id: UserId) {
        self.on = false;
        self.update_sched(http, id).await;
    }

    /// Set user awake flag
    pub fn awake(&mut self) {
        self.awake.store(true, atomic::Ordering::Relaxed)
    }

    /// Unset user awake flag
    pub fn asleep(&mut self) {
        self.awake.store(false, atomic::Ordering::Relaxed)
    }

    /// Set user allowed awake flag
    pub fn allow_awake(&mut self) {
        self.allowed_awake.store(true, atomic::Ordering::Relaxed)
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
