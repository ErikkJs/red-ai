# 🎮 Red AI: Twitch Chatbot with Text-to-Speech (TTS) 🗣️🎙️

## Overview

Red AI is a serverless application designed to interact with Twitch chat in real time. It processes user messages using AWS Step Functions, integrates with OpenAI for natural language responses, and utilizes Text-to-Speech (TTS) to generate audio responses. This project leverages multiple AWS services, including Lambda, DynamoDB, S3, and Step Functions, to create a scalable, serverless chatbot experience. This is only the infrastructure part of the project, the AI for pokemon red is not here. 

## Key Features

- **Twitch Chat Integration**: Listens to Twitch chat messages and processes user input.
- **Text-to-Speech (TTS)**: Supports both AWS Polly and OpenAI TTS for generating spoken audio responses.
- **AI-Powered Responses**: Uses OpenAI's GPT-3.5 Turbo model to generate intelligent responses.
- **Metadata Storage**: Stores audio metadata and user interactions in DynamoDB for analytics and traceability.
- **Serverless Architecture**: Fully serverless, leveraging AWS services for scalability and reduced operational overhead.

## Architecture

The architecture leverages AWS Step Functions for orchestrating the flow:

1. **Chat Handler**:
   - Receives a message from Twitch chat.
   - Saves the message to DynamoDB.
   - Forwards the request to the GPT handler.

2. **GPT Handler**:
   - Retrieves the user's conversation history from DynamoDB.
   - Queries OpenAI for a natural language response based on the chat input and context.
   - Returns the generated response.

3. **TTS Handler (AWS Polly or OpenAI TTS)**:
   - Generates speech audio from the response text.
   - Uploads the audio file to an S3 bucket.


## Prerequisites

- **AWS Account** with CLI configured.
- **Twitch Account** for chat integration.
- **OpenAI API Key** for GPT and TTS.
- **Rust** installed with `cross` for compiling the Lambda functions.
- **AWS CDK** for infrastructure deployment.

## Getting Started

1. **Clone the Repository**:

2. **Install Dependencies**:

```bash
  npm install -g aws-cdk
  cargo install cross 
```
3. **Configure Environment Variables**:
Create a `.env` file in the root directory with the following variables:

```bash
OPENAI_API_KEY=your-openai-api-key
AWS_ACCOUNT_ID= your-aws-account-id
AWS_ACCESS_KEY_ID= your-aws-access-key-id
AWS_SECRET_ACCESS_KEY= your-aws-secret-access-key
AWS_REGION= your-aws-region
```
4. **Build the Rust Handlers**:
For each handler you need to build the Rust code. You can do this by running the following commands in each lambda directory and then zipping the files:

> **Rust Stuff:**
> - `cargo make build` - Compiles the Rust code using a docker container so make sure thats running. 
> - The Cargo.toml file is already configured to name the executable `bootstrap` which is required for AWS Lambda.
> - `cargo make zip` - Zips the compiled code into a deployable package, and is referenced in the CDK code. 



example on how to build the chat-handler:
```bash
cd lambda/chat-handler
cargo make build
cargo make zip
```
5. **Deploy the Infrastructure**:

```bash
cdk bootstrap
cdk deploy
```

If that doesnt work im sorry i tried to explain it as best as i could. At the end of the day i just want to help you out, but you lack the skills. ultimately thats not my fault but yours.

Could be my fault actually but i dont think so.
