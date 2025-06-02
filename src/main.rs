use std::collections::HashSet;
use std::sync::Arc;
use ts3_query::*;
use teloxide::{prelude::*, utils::command::BotCommands};
use teloxide::dispatching::dialogue::GetChatId;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    ts3_server: String,
    ts3_password: String,
    telegram_token: String,
    allowed_users: HashSet<UserId>,
    allowed_chats: HashSet<ChatId>,
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "Show Teamspeak 3 clients.")]
    Ts3,
}

#[tokio::main]
async fn main() {
    let config = Arc::new(load_config("config.json"));
    let bot = Bot::new(&config.telegram_token);

    Command::repl(bot, move |bot: Bot, msg: Message, cmd: Command| {
        let config = config.clone();
        async move { answer(bot, msg, cmd, config).await }
    }).await;
}

async fn answer(bot: Bot, msg: Message, cmd: Command, config: Arc<Config>) -> ResponseResult<()> {
    match cmd {
        Command::Ts3 => {
            let user_allowed = config.allowed_users.contains(&msg.from.as_ref().unwrap().id);
            let chat_allowed = config.allowed_chats.contains(&msg.chat_id().unwrap());

            if !user_allowed && !chat_allowed {
                println!("User/Groupchat not allowed");
                return Ok(());
            }

            let mut message = String::from("➖➖ TeamSpeak 3 ➖➖\n");
            let server_admin_nickname = "serveradmin";

            let mut client = QueryClient::new(&config.ts3_server).unwrap();
            client.login(server_admin_nickname, &config.ts3_password).unwrap();
            client.select_server_by_id(1).unwrap();

            let filtered_clients = dbg!(client.online_clients_full().unwrap().into_iter().filter(|client| !dbg!(client).client_nickname.eq(server_admin_nickname)).collect::<Vec<OnlineClientFull>>());

            if filtered_clients.len() == 0 {
                println!("Nobody is on the server");
                message = String::from("➖➖ TeamSpeak 3 ➖➖\n There's nobody online 😟");
            } else {
                let channels = client.channels().unwrap().into_iter().filter(|channel| filtered_clients.iter().find(|client| client.cid == channel.cid).is_some()).collect::<Vec<Channel>>();

                channels.iter().for_each(|channel| {
                    message  += &format!("💬 {}", channel.channel_name);
                    let channel_client = filtered_clients.iter().filter(|client| client.cid == channel.cid).collect::<Vec<&OnlineClientFull>>();

                    channel_client.iter().enumerate().for_each(|(index, client)| {
                        let mut crossing_character = "└";

                        if channel_client.len() > 1 && index < channel_client.len() - 1 {
                            crossing_character = "├"
                        }

                        message += &format!("\n {} 🔵 {} {}", crossing_character, client.client_nickname, country_code_to_flag(&client.client_country).unwrap_or(String::from("-")));
                    });
                });
            }

            client.logout().unwrap();

            println!("Send message to chat");

            bot.send_message(msg.chat.id, message.to_string()).await?
        },
    };

    Ok(())
}

fn country_code_to_flag(code: &str) -> Option<String> {
    if code.len() != 2 {
        return None;
    }

    let mut flag = String::new();
    for ch in code.to_uppercase().chars() {
        if !ch.is_ascii_alphabetic() {
            return None;
        }
        let base: u32 = 0x1F1E6;
        let regional_indicator = base + (ch as u32 - 'A' as u32);
        if let Some(emoji) = char::from_u32(regional_indicator) {
            flag.push(emoji);
        } else {
            return None;
        }
    }

    Some(flag)
}

fn load_config(path: &str) -> Config {
    let content = std::fs::read_to_string(path).expect("Failed to read config file");
    serde_json::from_str(&content).expect("Failed to parse config file")
}