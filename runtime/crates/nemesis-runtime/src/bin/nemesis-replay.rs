use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use nemesis_runtime::AdaReplay;
use serde_json::json;

const MAX_LEDGER_BYTES: u64 = 64 * 1024 * 1024;

fn replay() -> Result<serde_json::Value, String> {
    let mut arguments = env::args_os().skip(1);
    let flag = arguments
        .next()
        .ok_or_else(|| "missing --ledger".to_owned())?;
    let path = arguments
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "missing ledger path".to_owned())?;
    if flag != "--ledger" || arguments.next().is_some() {
        return Err("usage: nemesis-replay --ledger <path>".to_owned());
    }
    let metadata = fs::metadata(&path).map_err(|error| error.to_string())?;
    if metadata.len() == 0 || metadata.len() > MAX_LEDGER_BYTES {
        return Err("ledger is empty or oversized".to_owned());
    }
    let replay = AdaReplay::parse(&fs::read(path).map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())?;
    let events: Vec<_> = replay
        .events
        .iter()
        .map(|event| {
            json!({
                "sequence": event.sequence,
                "kind_code": event.kind_code,
                "state_code": event.state_code,
                "source_digest": hex::encode(event.source_digest),
                "payload_digest": hex::encode(event.payload_digest),
                "previous_hash": hex::encode(event.previous_hash),
                "event_hash": hex::encode(event.event_hash)
            })
        })
        .collect();
    Ok(json!({
        "schema": "nemesis.replay/v1",
        "verdict": "VERIFIED",
        "events": events,
        "final_state_code": replay.final_state_code,
        "head": hex::encode(replay.head),
        "exact_state_reconstruction": replay.exact_state_reconstruction,
        "exact_model_reexecution": replay.exact_model_reexecution
    }))
}

fn main() -> ExitCode {
    match replay() {
        Ok(value) => {
            println!(
                "{}",
                serde_json::to_string(&value).expect("replay JSON serializes")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            println!(
                "{}",
                serde_json::to_string(&json!({"verdict": "REJECTED", "reason": error}))
                    .expect("replay rejection JSON serializes")
            );
            ExitCode::from(1)
        }
    }
}
