use serde_json::to_string_pretty;
use tenant_llm_spend_guard::{InfraiClient, SpendGuardError};

#[tokio::main]
async fn main() -> Result<(), SpendGuardError> {
    let args: Vec<String> = std::env::args().collect();
    let client = InfraiClient::from_env()?;
    match args.get(1).map(String::as_str) {
        Some("onboard") => {
            let cap = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(25.0);
            println!(
                "{}",
                to_string_pretty(&client.set_budget(cap, "monthly").await?).unwrap()
            );
        }
        Some("infer") => {
            let prompt = args
                .get(2)
                .map(String::as_str)
                .unwrap_or("Summarize tenant health in one sentence.");
            println!("{}", to_string_pretty(&client.chat(prompt).await?).unwrap());
        }
        Some("report") => {
            println!(
                "budget={}",
                to_string_pretty(&client.budget().await?).unwrap()
            );
            println!(
                "usage={}",
                to_string_pretty(&client.usage_timeseries().await?).unwrap()
            );
        }
        Some("key-drill") => {
            let run_id = args.get(2).map(String::as_str).unwrap_or("maintainer-run");
            client.exercise_temporary_key(run_id).await?;
            println!("temporary key rotation drill completed");
        }
        _ => {
            eprintln!("usage: spend_guard <onboard [cap]|infer [prompt]|report|key-drill [run-id]>")
        }
    }
    Ok(())
}
