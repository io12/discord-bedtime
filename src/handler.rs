use crate::say;
use crate::State;
use crate::CMD_PREFIX;

use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Presence;
use serenity::model::gateway::Ready;
use serenity::model::user::OnlineStatus;
use serenity::prelude::*;

/// Serenity handler for bot. This implements `EventHandler` to process all the
/// bot events.
pub struct Handler;

/// Implementation of event handler
#[async_trait]
impl EventHandler for Handler {
    /// Print a log message when the bot is ready
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is ready!", ready.user.name);
    }

    /// When a user's presence updates, flag the user as either awake or asleep,
    /// depending on the new online status
    async fn presence_update(&self, ctx: Context, presence: Presence) {
        let mut data = ctx.data.write().await;
        let user_info = data
            .get_mut::<State>()
            .expect("No state in context")
            .users
            .entry(presence.user.id)
            .or_default();

        match presence.status {
            OnlineStatus::Offline => user_info.asleep(),
            _ => user_info.awake(),
        }
    }

    /// Reply with usage information when bot is pinged
    async fn message(&self, ctx: Context, msg: Message) {
        let bot_user_id = ctx
            .http
            .get_current_user()
            .await
            .expect("Failed getting current user")
            .id;

        let pinged = msg.mentions_user_id(bot_user_id);

        if pinged {
            let resp = format!(
                "My command prefix is `{}`. Try `{} help` for a list of commands.",
                CMD_PREFIX, CMD_PREFIX
            );

            say(&ctx, &msg, resp).await
        }
    }
}
