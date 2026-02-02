# todoist-bot

A discord bot that automatically creates [Todoist](https://www.todoist.com/) tasks from Discord messages.

## Configuration

The bot is configured through environment variables

- `BOT_TOKEN` - The bot token
- `INTERACTION_KEY` - The interactions public key
- `TODOIST_API_TOKEN` - Your Todoist API token
- `CLAUDE_API_TOKEN` - An Anthropic/Claude API token
- `TZ_OVERRIDE` - An optional timezone to override the local timezone
- `CLAUDE_SYSTEM_PROMPT_PATH` - A path to a file where the system prompt is stored. Uses a [built-in](./src/llm/claude/system_prompt.txt) prompt if unspecified.

The bot also supports OpenAI (or any OpenAI compatible provider). To enable OpenAI support, set `LLM_PROVIDER` to `openai`.

The following variables are supported:

- `OPENAI_API_TOKEN` - Your API token
- `OPENAI_MODEL` - The model to use (Defaults to `gpt-5-nano`)
- `OPENAI_API_ENDPOINT` - The OpenAI API endpoint to use (Defaults to `https://api.openai.com/v1/`)
- `OPENAI_SYSTEM_PROMPT_PATH` - A path to a file where the system prompt is stored. Defaults to the built-in Claude prompt if unspecified.

## System Prompts

There is a [Built-In](./src/llm/claude/system_prompt.txt) prompt that has been tuned for Claude Haiku 4.5. A custom system prompt can be used by specifying either `OPENAI_SYSTEM_PROMPT_PATH` or `CLAUDE_SYSTEM_PROMPT_PATH`.

When loading a system prompt, the following strings will be interpolated:

- `{{CURRENT_TIME}}` -- the current system time as an RFC3339 string,
- `{{TIMEZONE}}` -- The configured timezone.

The LLM must return JSON output with the following fields:

- `title` -- The title of the reminder that will be created.
- `due` -- A RFC3339 timestamp when the reminder is due, or `null` if there is no due date
- `links` -- An array of strings representing any relevant links to include alongisde the reminder.
