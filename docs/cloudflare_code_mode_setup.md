# Cloudflare Code Mode Setup Guide

Cloudflare's Code Mode lets you run Workers AI models with structured tool calls from the Cloudflare dashboard or via the REST API/`wrangler` CLI. The steps below summarize how to get into the beta, obtain the credentials you need, and run the first request locally.

## 1. Request Code Mode access
1. Sign in to the [Cloudflare Dashboard](https://dash.cloudflare.com/).
2. Navigate to **Workers & Pages → AI**.
3. Locate **Code Mode** in the sidebar. If the feature is still in beta, click **Join the beta** and submit the short access form. Cloudflare sends an email once the feature is enabled for your account.

> **Tip:** You must have a verified Cloudflare account (including email and phone verification) and be on a Free, Pro, or higher plan. Enterprise customers should ask their account team to enable Workers AI with Code Mode.

## 2. Create a Workers AI API token
Code Mode calls Workers AI under the hood. Create a token with the minimum scope:

1. In the dashboard, click your avatar → **My Profile** → **API Tokens**.
2. Click **Create Token**.
3. Choose the **Workers AI** template (or create a custom token with the `Workers AI · Read` and `Workers AI · Write` permissions).
4. Restrict the token to the specific account that will run Code Mode jobs.
5. Copy the generated token. Store it securely—you cannot view it again after leaving the page.

You also need your **Account ID**. You can find it on **Workers & Pages → Overview** or by running `wrangler whoami` after logging in with the CLI.

## 3. Configure local credentials
Set the following environment variables before calling the API or running `wrangler` commands:

```bash
export CLOUDFLARE_ACCOUNT_ID="<your account id>"
export CLOUDFLARE_API_TOKEN="<token from step 2>"
```

If you prefer not to use environment variables, create (or update) `~/.config/.wrangler/config/default.toml` with the same values. `wrangler` prompts for these the first time you run `wrangler login` as well.

## 4. Verify access with `wrangler`
1. Install the CLI: `npm install -g wrangler` (or use `brew install cloudflare-wrangler`).
2. Authenticate: `wrangler login` (browser flow) or `wrangler config --api-token $CLOUDFLARE_API_TOKEN` for headless environments.
3. Run `wrangler ai models list` to confirm that Code Mode models (such as `@cf/code-claude-3-7-sonnet` or `@cf/code-llama-70b-instruct`) appear.

If the models list is empty or you receive a permissions error, double-check the token scopes and that Code Mode access has been granted to the account.

## 5. Run your first Code Mode call
With credentials in place, you can exercise the API directly:

```bash
curl -X POST \
  "https://api.cloudflare.com/client/v4/accounts/$CLOUDFLARE_ACCOUNT_ID/ai/run/@cf/code-claude-3-7-sonnet" \
  -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" \
  -H "Content-Type: application/json" \
  --data '{
    "messages": [
      {
        "role": "user",
        "content": [
          { "type": "text", "text": "Generate a Cloudflare Worker that echos request headers." }
        ]
      }
    ],
    "metadata": {
      "mode": "code"
    }
  }'
```

Successful requests return a JSON payload containing the generated code and tool calls. You can supply additional `tools` definitions in the payload to enable deployment or testing actions described in the blog post.

## 6. Keep your tokens safe
- Treat the API token as a secret. Rotate it from the **API Tokens** page if it leaks.
- Prefer Cloudflare's **AI Gateway** when exposing Code Mode from production apps; it adds rate limiting and observability on top of Workers AI.
- For team use, issue separate tokens per developer and restrict them to the minimum necessary account(s).

## Troubleshooting
| Symptom | Likely Cause | Fix |
| --- | --- | --- |
| `403 Unauthorized` when calling the API | Missing or incorrect token scope | Re-create the token with Workers AI read/write permissions. |
| Code Mode section missing from dashboard | Access not yet granted | Recheck the beta sign-up email or contact Cloudflare support. |
| `wrangler ai models list` returns only text models | Feature flag not enabled for your account | Wait for the enablement confirmation or ask support to confirm. |

Refer back to the [blog announcement](https://blog.cloudflare.com/code-mode/) for examples of tool definitions and workflow ideas once you have the credentials in place.

## 7. Integrate Code Mode with One Engine

Once Code Mode is working from the CLI you can wire it into One Engine's UTIR pipeline to orchestrate Workers AI runs from conversations or goals:

1. Allow outbound calls to Cloudflare by adding `api.cloudflare.com` to `ENGINE_ALLOWED_DOMAINS` before starting the engine (for example `ENGINE_ALLOWED_DOMAINS="localhost,127.0.0.1,api.cloudflare.com" ./run_dev.sh`).
2. Define a UTIR document that posts to the Code Mode endpoint using the new `http.post` operation:

   ```yaml
   task_id: "code-mode-scan"
   description: "Run Workers AI Code Mode from One Engine"
   operations:
     - type: "http.post"
       url: "https://api.cloudflare.com/client/v4/accounts/${CLOUDFLARE_ACCOUNT_ID}/ai/run/@cf/code-claude-3-7-sonnet"
       headers:
         Authorization: "Bearer ${CLOUDFLARE_API_TOKEN}"
         Content-Type: "application/json"
       body: |
         {"messages":[{"role":"user","content":[{"type":"text","text":"Generate a Worker that proxies One Engine"}]}],"metadata":{"mode":"code"}}
   ```

3. Submit the UTIR through `POST /compile_and_run` or embed it in a conversation branch so the generated Worker artifacts get captured alongside the usual execution receipts.

This pattern lets One Engine broker secure Workers AI calls (using the same Bits telemetry and ledgering as other operations) while Cloudflare handles code synthesis and deployment tooling.
