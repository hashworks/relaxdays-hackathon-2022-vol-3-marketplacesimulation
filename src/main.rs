mod client;
mod game;

use std::env;
use std::process::exit;

#[tokio::main]
async fn main() {
    let mut args = env::args();
    let binary = args.next().unwrap();

    // WARNING: In production environments it is not safe to provide credentials using commandline arguments,
    //          since they leak into the process list and whatnot. Instead, you should use environment variables.
    //          We only use it here because they are not critical and to simplify usage.
    let user_id = args.next();
    let api_key = args.next();

    let opt_api_url = args.next();

    if user_id.is_none() || api_key.is_none() {
        eprintln!("{} <user-id> <api-key> [<api-url>]", binary);
        exit(2);
    }

    let user_id = user_id.unwrap();
    let api_key = api_key.unwrap();

    println!("Initializing…");

    let mut client = match client::Client::new(opt_api_url, user_id, api_key).await {
        Ok(client) => client,
        Err(e) => {
            eprintln!("{}", e);
            exit(1);
        }
    };

    game::play(&mut client).await;
}
