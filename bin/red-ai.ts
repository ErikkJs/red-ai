#!/usr/bin/env node
import "source-map-support/register";
import * as cdk from "aws-cdk-lib";
import { RedAiStack } from "../lib/red-ai-stack";

// Load environment variables
const openAiApiKey = process.env.OPENAI_API_KEY || "";
const awsRegion = process.env.AWS_REGION || "us-west-2";

const app = new cdk.App();
new RedAiStack(app, "RedAiStack", {
  env: { account: process.env.CDK_DEFAULT_ACCOUNT, region: awsRegion },
  openAiApiKey: openAiApiKey,
});
