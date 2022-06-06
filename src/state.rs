use crate::user_info::UserInfo;

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serenity::{model::id::UserId, prelude::*};

lazy_static! {
    /// Path to the state save file
    static ref STATE_PATH: PathBuf = {
        let path = env!("CARGO_MANIFEST_DIR");
        let path = PathBuf::from(path);
        path.join("state.json")
    };
}

/// Data containing the bot's state. This is serialized to a file as it's
/// updated.
#[derive(Default, Serialize, Deserialize)]
pub struct State {
    /// Map of user IDs to per-user state
    pub users: HashMap<UserId, UserInfo>,
}

impl State {
    /// Serialize state to a file. This should be called whenever `State` is
    /// updated.
    pub fn save(&self) {
        let f = File::create(&*STATE_PATH);
        let f = f.expect("Failed to create state file");
        let f = BufWriter::new(f);
        let v = serde_json::to_writer(f, self);
        v.expect("Failed to write state");
    }

    /// Try to load state from a file, and use the default if the file does not
    /// exist.
    pub fn load() -> Self {
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

/// Field of `serenity::prelude::Context::data` used to store the state in the
/// context.
impl TypeMapKey for State {
    type Value = State;
}
