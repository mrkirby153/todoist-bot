const SYSTEM_PROMPT: &str = include_str!("system_prompt.txt");

use tracing::debug;

use crate::get_timezone_override;

pub fn get_system_prompt() -> String {
    let raw_prompt = SYSTEM_PROMPT;
    // Interpolate {{TIMEZONE}} with the configured timezone or the local timezone
    let timezone = get_timezone_override()
        .map(|tz| tz.name().to_string())
        .unwrap_or_else(|| iana_time_zone::get_timezone().unwrap_or_else(|_| "UTC".to_string()));
    let prompt = raw_prompt.replace("{{TIMEZONE}}", &timezone);
    let prompt = prompt.replace("{{CURRENT_TIME}}", &chrono::Local::now().to_rfc3339());
    debug!("Using system prompt: \n{}", prompt);
    prompt
}
