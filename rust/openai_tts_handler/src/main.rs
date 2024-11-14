use aws_config::BehaviorVersion;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client as S3Client;
use lambda_runtime::{service_fn, Error, LambdaEvent};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use uuid::Uuid;

#[derive(Deserialize)]
struct Request {
    text: String,
}

#[derive(Serialize)]
struct Response {
    audio_url: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let func = service_fn(handle_openai_tts);
    lambda_runtime::run(func).await?;
    Ok(())
}

async fn handle_openai_tts(event: LambdaEvent<Request>) -> Result<Response, Error> {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let s3_client = S3Client::new(&config);
    let openai_api_key =
        env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable not set");
    let bucket_name = env::var("AUDIO_BUCKET").expect("AUDIO_BUCKET environment variable not set");
    let audio_key = format!("audio/{}.mp3", Uuid::new_v4());
    println!("Generated audio key: {}", audio_key);

    // Generate speech audio with OpenAI TTS
    let audio_bytes = match generate_openai_tts(&event.payload.text, &openai_api_key).await {
        Ok(bytes) => bytes,
        Err(e) => {
            println!("Error generating speech with OpenAI: {:?}", e);
            return Err(Error::from(format!("OpenAI TTS request failed: {:?}", e)));
        }
    };

    // Upload buffered audio to S3
    if let Err(e) = s3_client
        .put_object()
        .bucket(&bucket_name)
        .key(&audio_key)
        .body(ByteStream::from(audio_bytes))
        .send()
        .await
    {
        println!("Error uploading audio to S3: {:?}", e);
        return Err(Error::from(format!("S3 upload failed: {:?}", e)));
    }

    let audio_url = format!("https://{}.s3.amazonaws.com/{}", bucket_name, audio_key);
    println!("Audio URL: {}", audio_url);

    Ok(Response { audio_url })
}

async fn generate_openai_tts(text: &str, api_key: &str) -> Result<Vec<u8>, Error> {
    let client = Client::new();

    let response = client
        .post("https://api.openai.com/v1/audio/speech")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&serde_json::json!({
            "input": text,
            "model": "tts-1",
           "voice": "nova"
        }))
        .send()
        .await?;

    if !response.status().is_success() {
        let error_message = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        println!("OpenAI API error: {}", error_message);
        return Err(Error::from(format!(
            "OpenAI TTS API failed: {}",
            error_message
        )));
    }

    let audio_bytes = response.bytes().await?.to_vec();
    Ok(audio_bytes)
}
