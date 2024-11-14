import {
  Stack,
  StackProps,
  RemovalPolicy,
  CfnOutput,
  Duration,
} from "aws-cdk-lib";
import { Construct } from "constructs";
import * as s3 from "aws-cdk-lib/aws-s3";
import * as dynamodb from "aws-cdk-lib/aws-dynamodb";
import * as lambda from "aws-cdk-lib/aws-lambda";
import * as iam from "aws-cdk-lib/aws-iam";
import * as stepfunctions from "aws-cdk-lib/aws-stepfunctions";
import * as stepfunctions_tasks from "aws-cdk-lib/aws-stepfunctions-tasks";

interface RedAiStackProps extends StackProps {
  openAiApiKey: string;
}

export class RedAiStack extends Stack {
  constructor(scope: Construct, id: string, props: RedAiStackProps) {
    super(scope, id, props);

    // S3 bucket for storing audio files
    const audioBucket = new s3.Bucket(this, "AudioBucket", {
      removalPolicy: RemovalPolicy.DESTROY,
      autoDeleteObjects: true,
    });

    // DynamoDB table for tracking chat interactions
    const chatTable = new dynamodb.Table(this, "ChatTable", {
      partitionKey: { name: "user_id", type: dynamodb.AttributeType.STRING },
      sortKey: { name: "timestamp", type: dynamodb.AttributeType.STRING },
      removalPolicy: RemovalPolicy.DESTROY,
    });

    // Lambda function: ChatHandler
    const chatHandler = new lambda.Function(this, "ChatHandler", {
      runtime: lambda.Runtime.PROVIDED_AL2,
      handler: "bootstrap",
      code: lambda.Code.fromAsset("rust/chat_handler/chat_handler.zip"),
      environment: {
        CHAT_TABLE: chatTable.tableName,
      },
    });

    // Lambda function: GptHandler
    const gptHandler = new lambda.Function(this, "GptHandler", {
      runtime: lambda.Runtime.PROVIDED_AL2,
      handler: "bootstrap",
      code: lambda.Code.fromAsset("rust/gpt_handler/gpt_handler.zip"),
      environment: {
        CHAT_TABLE: chatTable.tableName,
        OPENAI_API_KEY: props.openAiApiKey,
      },
      timeout: Duration.seconds(15),
    });

    // Lambda function: PollyHandler
    const pollyHandler = new lambda.Function(this, "PollyHandler", {
      runtime: lambda.Runtime.PROVIDED_AL2,
      handler: "bootstrap",
      code: lambda.Code.fromAsset("rust/polly_handler/polly_handler.zip"),
      environment: {
        AUDIO_BUCKET: audioBucket.bucketName,
      },
      timeout: Duration.seconds(15),
    });
    const openAiTtsHandler = new lambda.Function(this, "OpenAiTtsHandler", {
      runtime: lambda.Runtime.PROVIDED_AL2,
      handler: "bootstrap",
      code: lambda.Code.fromAsset(
        "rust/openai_tts_handler/openai_tts_handler.zip"
      ),
      environment: {
        AUDIO_BUCKET: audioBucket.bucketName,
        OPENAI_API_KEY: props.openAiApiKey,
      },      timeout: Duration.seconds(15),

    });

    chatTable.grantReadWriteData(chatHandler);
    chatTable.grantReadWriteData(gptHandler);
    chatTable.grantReadWriteData(openAiTtsHandler);
    audioBucket.grantPut(pollyHandler);
    audioBucket.grantPut(openAiTtsHandler);

    pollyHandler.addToRolePolicy(
      new iam.PolicyStatement({
        actions: ["polly:SynthesizeSpeech"],
        resources: ["*"],
        effect: iam.Effect.ALLOW,
      })
    );

    const chatTask = new stepfunctions_tasks.LambdaInvoke(this, "ChatTask", {
      lambdaFunction: chatHandler,
      outputPath: "$.Payload",
    });

    const gptTask = new stepfunctions_tasks.LambdaInvoke(this, "GptTask", {
      lambdaFunction: gptHandler,
      payload: stepfunctions.TaskInput.fromObject({
        user_id: stepfunctions.JsonPath.stringAt("$.user_id"),
        prompt: stepfunctions.JsonPath.stringAt("$.prompt"),
      }),
      outputPath: "$.Payload",
    });

    const pollyTask = new stepfunctions_tasks.LambdaInvoke(this, "PollyTask", {
      lambdaFunction: pollyHandler,
      payload: stepfunctions.TaskInput.fromObject({
        text: stepfunctions.JsonPath.stringAt("$.completion"),
        user_id: stepfunctions.JsonPath.stringAt("$.user_id"),
      }),
      outputPath: "$.Payload",
    });

    const openAiTtsTask = new stepfunctions_tasks.LambdaInvoke(
      this,
      "OpenAiTtsTask",
      {
        lambdaFunction: openAiTtsHandler,
        payload: stepfunctions.TaskInput.fromObject({
          text: stepfunctions.JsonPath.stringAt("$.completion"),
          user_id: stepfunctions.JsonPath.stringAt("$.user_id"),
        }),
        outputPath: "$.Payload",
      }
    );

    const workflow = new stepfunctions.StateMachine(this, "RedAiWorkflow", {
      definition: chatTask.next(gptTask).next(openAiTtsTask),
    });

    new CfnOutput(this, "WorkflowArn", {
      value: workflow.stateMachineArn,
      exportName: "RedAiWorkflowArn",
    });
  }
}
