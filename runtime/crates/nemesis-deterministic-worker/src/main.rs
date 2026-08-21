use std::collections::BTreeMap;
use std::env;
use std::process::ExitCode;

use nemesis_protocol::parse_worker_message;
use serde_json::{Value, json};

fn envelope(id: u64, mission_id: &str, worker_id: &str, method: &str, params: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "protocol": "nemesis.worker/v1",
        "mission_id": mission_id,
        "worker_id": worker_id,
        "method": method,
        "params": params
    })
}

fn parse_flags(
    arguments: impl IntoIterator<Item = String>,
) -> Result<BTreeMap<String, String>, String> {
    let mut values = BTreeMap::new();
    let mut arguments = arguments.into_iter();
    while let Some(flag) = arguments.next() {
        if !flag.starts_with("--") {
            return Err(format!("unexpected positional argument: {flag}"));
        }
        let value = arguments
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        if values.insert(flag.clone(), value).is_some() {
            return Err(format!("duplicate flag: {flag}"));
        }
    }
    Ok(values)
}

fn require<'a>(values: &'a BTreeMap<String, String>, name: &str) -> Result<&'a str, String> {
    values
        .get(name)
        .map(String::as_str)
        .ok_or_else(|| format!("missing required flag: {name}"))
}

fn validate_messages(messages: &[Value]) -> Result<Vec<String>, String> {
    messages
        .iter()
        .map(|message| {
            let encoded = serde_json::to_string(message).map_err(|error| error.to_string())?;
            parse_worker_message(encoded.as_bytes()).map_err(|error| error.to_string())?;
            Ok(encoded)
        })
        .collect()
}

fn run() -> Result<Vec<String>, String> {
    let mut arguments = env::args().skip(1);
    let mode = arguments
        .next()
        .ok_or_else(|| "expected worker mode: plan or complete".to_owned())?;
    let values = parse_flags(arguments)?;
    let mission_id = require(&values, "--mission-id")?;
    let worker_id = require(&values, "--worker-id")?;

    let messages = match mode.as_str() {
        "plan" => {
            let path = require(&values, "--path")?;
            let digest = require(&values, "--content-digest")?;
            if values.len() != 4 {
                return Err(
                    "plan accepts only mission, worker, path, and content digest".to_owned(),
                );
            }
            vec![
                envelope(1, mission_id, worker_id, "heartbeat", json!({})),
                envelope(
                    2,
                    mission_id,
                    worker_id,
                    "propose_action",
                    json!({
                        "action_kind": "filesystem.modify",
                        "relative_path": path,
                        "content_digest": digest,
                        "purpose": "Apply the authorized deterministic fixture change."
                    }),
                ),
            ]
        }
        "complete" => {
            if values.len() != 2 {
                return Err("complete accepts only mission and worker identifiers".to_owned());
            }
            vec![envelope(
                1,
                mission_id,
                worker_id,
                "propose_completion",
                json!({"claim_ids": ["build", "tests"]}),
            )]
        }
        _ => return Err(format!("unknown worker mode: {mode}")),
    };
    validate_messages(&messages)
}

fn main() -> ExitCode {
    match run() {
        Ok(messages) => {
            for message in messages {
                println!("{message}");
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("nemesis worker refused: {error}");
            ExitCode::from(2)
        }
    }
}
