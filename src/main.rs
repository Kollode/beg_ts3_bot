use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use teloxide::dispatching::dialogue::GetChatId;
use teloxide::{prelude::*, utils::command::BotCommands};
use ts3_query::*;

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    ts3_server: String,
    ts3_password: String,
    telegram_token: String,
    allowed_users: HashSet<UserId>,
    allowed_chats: HashSet<ChatId>,
}

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum Command {
    #[command(description = "Show Teamspeak 3 clients.")]
    Ts3,
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let config = Arc::new(load_config("config.json"));
    let bot = Bot::new(&config.telegram_token);

    Command::repl(bot, move |bot: Bot, msg: Message, cmd: Command| {
        let config = config.clone();
        async move { answer(bot, msg, cmd, config).await }
    })
    .await;
}

async fn answer(bot: Bot, msg: Message, cmd: Command, config: Arc<Config>) -> ResponseResult<()> {
    match cmd {
        Command::Ts3 => {
            let user_allowed = config
                .allowed_users
                .contains(&msg.from.as_ref().unwrap().id);
            let chat_allowed = config.allowed_chats.contains(&msg.chat_id().unwrap());

            if !user_allowed && !chat_allowed {
                log::info!("User/Groupchat not allowed");
                return Ok(());
            }

            let mut message = String::from("➖➖ TeamSpeak 3 ➖➖\n");
            let server_admin_nickname = "serveradmin";

            let mut client = QueryClient::new(&config.ts3_server).unwrap();
            client
                .login(server_admin_nickname, &config.ts3_password)
                .unwrap();
            client.select_server_by_id(1).unwrap();

            let filtered_clients = client
                .online_clients()
                .unwrap()
                .into_iter()
                .filter(|client| !(client).client_nickname.eq(server_admin_nickname))
                .collect::<Vec<OnlineClient>>();

            if filtered_clients.len() == 0 {
                log::info!("Nobody is on the server");
                message = String::from("➖➖ TeamSpeak 3 ➖➖\n There's nobody online 😟");
            } else {
                let channels = client
                    .channels()
                    .unwrap()
                    .into_iter()
                    .filter(|channel| {
                        filtered_clients
                            .iter()
                            .find(|client| client.cid == channel.cid)
                            .is_some()
                    })
                    .collect::<Vec<Channel>>();

                channels.iter().for_each(|channel| {
                    message += &format!("💬 {}", channel.channel_name);
                    
                    let channel_client = filtered_clients
                        .iter()
                        .filter(|client| client.cid == channel.cid)
                        .collect::<Vec<&OnlineClient>>();

                    channel_client
                        .iter()
                        .enumerate()
                        .for_each(|(index, client)| {
                            let mut crossing_character = "└";

                            if channel_client.len() > 1 && index < channel_client.len() - 1 {
                                crossing_character = "├"
                            }

                            message += &format!(
                                "\n {} 🔵 {}",
                                crossing_character,
                                client.client_nickname
                            );
                        });
                });
            }

            client.logout().unwrap();

            log::info!("Send message to chat");

            bot.send_message(msg.chat.id, message.to_string()).await?
        }
    };

    Ok(())
}

fn load_config(path: &str) -> Config {
    let content = std::fs::read_to_string(path).expect("Failed to read config file");
    serde_json::from_str(&content).expect("Failed to parse config file")
}
