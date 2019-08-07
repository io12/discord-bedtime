use crate::State;

use serenity::model::event::PresenceUpdateEvent;
use serenity::model::gateway::Ready;
use serenity::model::user::OnlineStatus;
use serenity::prelude::*;

/// Serenity handler for bot. This implements `EventHandler` to process all the
/// bot events.
pub struct Handler;

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
