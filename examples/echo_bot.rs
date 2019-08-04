extern crate serde;
extern crate serde_json as json;
extern crate tweetust;
extern crate twitter_stream;

use std::fs::File;
use std::path::PathBuf;

use serde::de;
use serde::Deserialize;
use twitter_stream::rt::{self, Future, Stream};
use twitter_stream::{Error, Token, TwitterStreamBuilder};

#[derive(Deserialize)]
#[serde(untagged)]
enum StreamMessage {
    Tweet(Tweet),
    Other(de::IgnoredAny),
}

#[derive(Deserialize)]
struct Tweet {
    created_at: String,
    entities: Entities,
    id: i64,
    text: String,
    user: User,
}

#[derive(Deserialize)]
struct Entities {
    user_mentions: Vec<UserMention>,
}

#[derive(Deserialize)]
struct UserMention {
    id: i64,
}

#[derive(Deserialize)]
struct User {
    id: i64,
    screen_name: String,
}

fn main() {
    const TRACK: &str = "@NAME_OF_YOUR_ACCOUNT";

    // `credential.json` must have the following form:
    // {"consumer_key": "...", "consumer_secret": "...", "access_key": "...", "access_secret": "..."}

    let mut credential_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    credential_path.pop();
    credential_path.push("credential.json");

    let credential = File::open(credential_path).unwrap();
    let token: Token = json::from_reader(credential).unwrap();

    let stream = TwitterStreamBuilder::filter(token.borrowed())
        .track(Some(TRACK))
        .listen()
        .unwrap()
        .flatten_stream();
    let rest = tweetust::TwitterClient::new(
        token,
        tweetust::DefaultHttpHandler::with_https_connector().unwrap(),
    );

    // Information of the authenticated user:
    let user = rest
        .account()
        .verify_credentials()
        .execute()
        .unwrap()
        .object;

    let bot = stream
        .for_each(move |json| {
            if let Ok(StreamMessage::Tweet(tweet)) = json::from_str(&json) {
                if tweet.user.id != user.id
                    && tweet
                        .entities
                        .user_mentions
                        .iter()
                        .any(|mention| mention.id == user.id)
                {
                    println!(
                        "On {}, @{} tweeted: {:?}",
                        tweet.created_at, tweet.user.screen_name, tweet.text
                    );

                    let response = format!("@{} {}", tweet.user.screen_name, tweet.text);
                    rest.statuses()
                        .update(response)
                        .in_reply_to_status_id(tweet.id)
                        .execute()
                        .map_err(Error::custom)?;
                }
            }

            Ok(())
        })
        .map_err(|e| println!("error: {}", e));

    rt::run(bot);
}