use aws_config::{load_defaults, BehaviorVersion};
use aws_sdk_dynamodb::{types::AttributeValue, Client as DynamoDbClient};
use chrono::Utc;
use lambda_runtime::{service_fn, Error, LambdaEvent};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct ChatEvent {
    user_id: String,
    message: String,
}

#[derive(Serialize)]
struct GptRequest {
    user_id: String,
    prompt: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let func = service_fn(handle_chat);
    lambda_runtime::run(func).await?;
    Ok(())
}

async fn handle_chat(event: LambdaEvent<ChatEvent>) -> Result<GptRequest, Error> {
    let config = load_defaults(BehaviorVersion::latest()).await;
    let db_client = DynamoDbClient::new(&config);

    println!(
        "Received chat event: user_id={}, message={}",
        event.payload.user_id, event.payload.message
    );

    if let Err(e) = save_message_to_dynamodb(
        &db_client,
        &event.payload.user_id,
        "user",
        &event.payload.message,
    )
    .await
    {
        println!("Error saving item to DynamoDB: {:?}", e);
        return Err(Error::from(format!(
            "Failed to save message to DynamoDB: {:?}",
            e
        )));
    }

    Ok(GptRequest {
        user_id: event.payload.user_id.clone(),
        prompt: event.payload.message.clone(),
    })
}

async fn save_message_to_dynamodb(
    db_client: &DynamoDbClient,
    user_id: &str,
    role: &str,
    content: &str,
) -> Result<(), aws_sdk_dynamodb::Error> {
    let timestamp = Utc::now().to_rfc3339();

    println!(
        "Saving to DynamoDB - user_id: {}, timestamp: {}, role: {}, content: {}",
        user_id, timestamp, role, content
    );

    let request = db_client
        .put_item()
        .table_name(std::env::var("CHAT_TABLE").unwrap_or_else(|_| "ChatTable".to_string()))
        .item("user_id", AttributeValue::S(user_id.to_string()))
        .item("timestamp", AttributeValue::S(timestamp))
        .item("role", AttributeValue::S(role.to_string()))
        .item("content", AttributeValue::S(content.to_string()));

    request.send().await?;
    println!("Successfully saved item to DynamoDB");

    Ok(())
}
