use serde_json::json;
use tg_flows::ChatId;
use tg_flows::{listen_to_update, Telegram, Update, UpdateKind, update_handler};
use openai_flows::{
    chat::{ChatModel, ChatOptions},
    OpenAIFlows,
};
use store_flows::{get, set};
use flowsnet_platform_sdk::logger;

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
pub async fn on_deploy() {
    let telegram_token = std::env::var("telegram_token").unwrap();
    listen_to_update(telegram_token).await;
}

#[update_handler]
async fn handler(update: Update) {
    logger::init();
    let telegram_token = std::env::var("telegram_token").unwrap();
    let placeholder_text = std::env::var("placeholder").unwrap_or("Typing ...".to_string());
    let system_prompt = std::env::var("system_prompt").unwrap_or("Вы помощник в группе telegram который отвечает только на вопросы связанные с повышением эффективности при использовании GPT. Если вопрос не соответствует теме - ответьте что вы не настроены для ответа на этот вопрос и попросите пользователя сформулировать вопрос на тему Повышение эффективности при использовании GPT. Ваш ответ не должен превышать 50 слов.".to_string());
    let help_mesg = std::env::var("help_mesg").unwrap_or("Я помощник в этой теме. Могу вам помочь?".to_string());

    let tele = Telegram::new(telegram_token.to_string());

    if let UpdateKind::Message(msg) = update.kind {
        let chat_id = msg.chat.id;
        let thread_id = msg.thread_id; // Добавьте эту строку
        log::info!("Received message from {}", chat_id);

        // Указанные значения для вашей группы и темы
        let target_chat_id = ChatId(-1002129124389); // Преобразуйте значение в тип ChatId
        let target_thread_id = 11;

        // Проверка на совпадение chat_id и message_thread_id
        if chat_id != target_chat_id || thread_id != Some(target_thread_id) {
            // Действие, если условие не выполняется
            return;
        }
        
        let mut openai = OpenAIFlows::new();
        openai.set_retry_times(3);
        let mut co = ChatOptions::default();
        // co.model = ChatModel::GPT4;
        co.model = ChatModel::GPT35Turbo;
        co.restart = false;
        co.system_prompt = Some(&system_prompt);

        let text = msg.text().unwrap_or("");
        if text.eq_ignore_ascii_case("/help") {
            _ = tele.send_message(target_chat_id, &help_mesg);         

        } else if text.eq_ignore_ascii_case("/start") {
            _ = tele.send_message(target_chat_id, &help_mesg); // Измените chat_id на target_chat_id
            set(&target_chat_id.to_string(), json!(true), None);
            log::info!("Started conversation for {}", target_chat_id);

        } else if text.eq_ignore_ascii_case("/restart") {
            _ = tele.send_message(target_chat_id, "Ok, I am starting a new conversation."); // Измените chat_id на target_chat_id
            set(&target_chat_id.to_string(), json!(true), None);
            log::info!("Restarted conversation for {}", target_chat_id);

        } else {
            let placeholder = tele
                .send_message(chat_id, &placeholder_text)
                .expect("Error occurs when sending Message to Telegram");

            let restart = match get(&chat_id.to_string()) {
                Some(v) => v.as_bool().unwrap_or_default(),
                None => false,
            };
            if restart {
                log::info!("Detected restart = true");
                set(&chat_id.to_string(), json!(false), None);
                co.restart = true;
            }

            match openai.chat_completion(&chat_id.to_string(), &text, &co).await {
                Ok(r) => {
                    _ = tele.edit_message_text(chat_id, placeholder.id, r.choice);
                }
                Err(e) => {
                    _ = tele.edit_message_text(chat_id, placeholder.id, "Sorry, an error has occured. Please try again later!");
                    log::error!("OpenAI returns error: {}", e);
                }
            }
        }
    }
}
