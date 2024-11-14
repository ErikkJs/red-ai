use aws_config::BehaviorVersion;
use aws_sdk_dynamodb::{types::AttributeValue, Client as DynamoDbClient};
use lambda_runtime::{service_fn, Error, LambdaEvent};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

#[derive(Deserialize)]
struct Request {
    user_id: String,
    prompt: String,
}

#[derive(Serialize)]
struct Response {
    completion: String,
    user_id: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let func = service_fn(handle_gpt);
    lambda_runtime::run(func).await?;
    Ok(())
}

async fn handle_gpt(event: LambdaEvent<Request>) -> Result<Response, Error> {
    let api_key = match env::var("OPENAI_API_KEY") {
        Ok(key) => key,
        Err(e) => {
            println!("Failed to retrieve OPENAI_API_KEY from environment: {}", e);
            return Err(Error::from(format!("Missing API key: {}", e)));
        }
    };

    let table_name = match env::var("CHAT_TABLE") {
        Ok(name) => name,
        Err(e) => {
            println!("Failed to retrieve CHAT_TABLE from environment: {}", e);
            return Err(Error::from(format!("Missing DynamoDB Table Name: {}", e)));
        }
    };

    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let db_client = DynamoDbClient::new(&config);
    let client = Client::new();

    println!(
        "Received GPT request with user_id: {}, prompt: {}",
        event.payload.user_id, event.payload.prompt
    );

    // Fetch conversation history
    let history =
        match fetch_conversation_history(&db_client, &table_name, &event.payload.user_id).await {
            Ok(h) => h,
            Err(e) => {
                println!("Error fetching conversation history: {:?}", e);
                return Err(Error::from(format!("DynamoDB query failed: {:?}", e)));
            }
        };

    println!("Fetched conversation history: {:?}", history);

    // Generate response from OpenAI
    let completion = match generate_response(&client, &api_key, &history).await {
        Ok(response) => response,
        Err(e) => {
            println!("Error generating response from OpenAI: {:?}", e);
            return Err(Error::from(format!("OpenAI request failed: {:?}", e)));
        }
    };

    Ok(Response {
        completion,
        user_id: event.payload.user_id,
    })
}

async fn fetch_conversation_history(
    db_client: &DynamoDbClient,
    table_name: &str,
    user_id: &str,
) -> Result<Vec<HashMap<String, String>>, Error> {
    println!("Fetching conversation history for user_id: {}", user_id);

    let response = db_client
        .query()
        .table_name(table_name)
        .key_condition_expression("#uid = :user_id")
        .expression_attribute_names("#uid", "user_id")
        .expression_attribute_values(":user_id", AttributeValue::S(user_id.to_string()))
        .send()
        .await?;

    let items = response.items();
    let mut history = Vec::new();

    for item in items {
        let role = item
            .get("role")
            .and_then(|v| v.as_s().ok())
            .unwrap_or(&String::from("user"))
            .to_string();

        let content = item
            .get("content")
            .and_then(|v| v.as_s().ok())
            .unwrap_or(&String::from(""))
            .to_string();

        history.push(HashMap::from([
            ("role".to_string(), role),
            ("content".to_string(), content),
        ]));
    }

    Ok(history)
}

async fn generate_response(
    client: &Client,
    api_key: &str,
    history: &[HashMap<String, String>],
) -> Result<String, Error> {
    println!(
        "Sending request to OpenAI with conversation history: {:?}",
        history
    );

    let response = match client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&serde_json::json!({
            "model": "gpt-3.5-turbo",
            "messages": history,
            "max_tokens": 100,
            "temperature": 0.7
        }))
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            println!("HTTP request to OpenAI failed: {:?}", e);
            return Err(Error::from(format!("HTTP request failed: {:?}", e)));
        }
    };

    let json_response = match response.json::<serde_json::Value>().await {
        Ok(json) => json,
        Err(e) => {
            println!("Failed to parse JSON response from OpenAI: {:?}", e);
            return Err(Error::from(format!("JSON parsing failed: {:?}", e)));
        }
    };

    let completion = json_response["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("");

    println!("Received completion from OpenAI: {}", completion);

    Ok(completion.to_string())
}
