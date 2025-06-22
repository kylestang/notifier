use anyhow::{Result, anyhow};
use base64::{Engine, prelude::BASE64_STANDARD};
use mail_parser::MessageParser;
use std::{
    fs::{self},
    io::Read,
    net::{TcpListener, TcpStream},
    time::Duration,
};

struct Config {
    account: String,
    key: String,
    secret: String,
    from: String,
    to: String,
    valid_emails: Vec<String>,
}

fn main() {
    let config = read_config();
    let listener = TcpListener::bind("127.0.0.1:25").expect("Couldn't bind to port");
    println!("Listening...");

    for connection in listener.incoming() {
        connection
            .map_err(|e| anyhow!(e))
            .and_then(|stream| process_email(&config, stream))
            .and_then(|msg| send_notification(&config, msg))
            .unwrap_or_else(|e| println!("error: {:?}", e))
    }
}

fn read_config() -> Config {
    let content = fs::read_to_string("Config").expect("couldn't read config");
    let mut lines = content.lines().map(|s| s.to_owned());
    Config {
        account: lines.next().unwrap(),
        key: lines.next().unwrap(),
        secret: lines.next().unwrap(),
        from: lines.next().unwrap(),
        to: lines.next().unwrap(),
        valid_emails: lines.collect(),
    }
}

fn process_email(config: &Config, stream: TcpStream) -> Result<String> {
    let mut msg = Vec::new();
    stream.set_read_timeout(Some(Duration::from_secs(20)))?;
    stream.take(100_000).read_to_end(&mut msg)?;

    let email = MessageParser::default()
        .parse(&msg)
        .ok_or(anyhow!("failed to parse email"))?;

    let sender = email.from().ok_or(anyhow!("failed to parse sender"))?;
    let mut valid = false;
    for address in &config.valid_emails {
        if sender.contains(address) {
            valid = true;
            break;
        }
    }

    if !valid {
        return Err(anyhow!("unknown sender"));
    }

    println!("Email received: {:?}\n", email.body_preview(1000));
    Ok(email
        .body_preview(1000)
        .map(|s| s.to_string())
        .unwrap_or(String::from("error: failed to parse body")))
}

fn send_notification(config: &Config, msg: String) -> Result<()> {
    let response = ureq::post(format!(
        "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json",
        config.account
    ))
    .header(
        "Authorization",
        format!(
            "Basic {}",
            BASE64_STANDARD.encode(format!("{}:{}", config.key, config.secret))
        ),
    )
    .send_form([("To", &config.to), ("From", &config.from), ("Body", &msg)])?;

    println!("twilio response: {:?}\n", response);
    Ok(())
}
