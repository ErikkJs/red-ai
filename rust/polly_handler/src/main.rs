use aws_config::BehaviorVersion;
use aws_sdk_polly::{types::OutputFormat, types::VoiceId, Client as PollyClient};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client as S3Client;
use lambda_runtime::{service_fn, Error, LambdaEvent};
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
    let func = service_fn(handle_polly);
    lambda_runtime::run(func).await?;
    Ok(())
}

async fn handle_polly(event: LambdaEvent<Request>) -> Result<Response, Error> {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let polly_client = PollyClient::new(&config);
    let s3_client = S3Client::new(&config);

    let bucket_name = env::var("AUDIO_BUCKET").expect("AUDIO_BUCKET environment variable not set");
    let audio_key = format!("audio/{}.mp3", Uuid::new_v4());
    println!("Generated audio key: {}", audio_key);

    // Generate speech audio with Polly
    let response = polly_client
        .synthesize_speech()
        .output_format(OutputFormat::Mp3)
        .text(&event.payload.text)
        .voice_id(VoiceId::Joanna)
        .send()
        .await;

    let audio_stream = match response {
        Ok(resp) => resp.audio_stream,
        Err(e) => {
            println!("Error generating speech with Polly: {:?}", e);
            return Err(Error::from(format!("Polly request failed: {:?}", e)));
        }
    };

    // Collect the audio stream into a buffer
    let audio_bytes = match ByteStream::collect(audio_stream).await {
        Ok(bytes) => bytes.into_bytes(),
        Err(e) => {
            println!("Error buffering audio stream: {:?}", e);
            return Err(Error::from(format!("Audio buffering failed: {:?}", e)));
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
