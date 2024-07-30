use std::path::PathBuf;

use alloy::{
    dyn_abi::DecodedEvent,
    json_abi::{Event, JsonAbi},
};
use eyre::Result;
use revm::primitives::{Log, B256};

/// Wrapper around a decoded event
#[derive(Debug, Clone)]
pub(crate) struct FullDecodedEvent {
    pub(crate) inner: DecodedEvent,
    pub(crate) event: Event,
    pub(crate) log: Log,
    pub(crate) transaction_log_index: usize,
}

/// Wrapper around a raw log
#[derive(Debug, Clone)]
pub(crate) struct FullRawEvent {
    pub(crate) log: Log,
    pub(crate) transaction_log_index: usize,
}

/// Wrapper enum for both raw and decoded events
#[derive(Debug, Clone)]
pub(crate) enum RawOrDecodedEvent {
    Raw(FullRawEvent),
    Decoded(FullDecodedEvent),
}

impl std::fmt::Display for RawOrDecodedEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RawOrDecodedEvent::Raw(log) => write!(
                f,
                r#"Transaction Log Index : {}
Address               : {}
Event Selector        : {}
Event Signature       : N/A
Topic 1               : {}
Topic 2               : {}
Topic 3               : {}
Data                  : 0x{}
"#,
                log.transaction_log_index,
                log.log.address,
                log.log.topics()[0],
                log.log
                    .topics()
                    .get(1)
                    .map(|t| t.to_string())
                    .unwrap_or_else(|| String::from("N/A")),
                log.log
                    .topics()
                    .get(2)
                    .map(|t| t.to_string())
                    .unwrap_or_else(|| String::from("N/A")),
                log.log
                    .topics()
                    .get(3)
                    .map(|t| t.to_string())
                    .unwrap_or_else(|| String::from("N/A")),
                log.log
                    .data
                    .data
                    .to_vec()
                    .chunks(32)
                    .map(hex::encode)
                    .collect::<Vec<_>>()
                    .join("\n                      :   ")
            ),
            RawOrDecodedEvent::Decoded(decoded) => {
                write!(
                    f,
                    r#"Transaction Log Index : {}
Address               : {}
Event Selector        : {}
Event Signature       : {}
Decoded               :
                      : {}
"#,
                    decoded.transaction_log_index,
                    decoded.log.address,
                    decoded.log.topics()[0],
                    decoded.event.signature(),
                    decoded
                )
            }
        }
    }
}

impl std::fmt::Display for FullDecodedEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let decoded_str = self
            .inner
            .indexed
            .iter()
            .enumerate()
            .chain(
                self.inner.body.iter().enumerate().map(|(i, v)| (i + self.inner.indexed.len(), v)),
            )
            .map(|(i, value)| {
                let indexed_len = self.inner.indexed.len();
                let name = if i < indexed_len {
                    self.event.inputs.iter().filter(|input| input.indexed).nth(i)
                } else {
                    self.event.inputs.iter().filter(|input| !input.indexed).nth(i - indexed_len)
                }
                .map(|input| input.name.as_str())
                .unwrap_or("N/A");

                format!("{} {:?}", name, value)
            })
            .collect::<Vec<_>>()
            .join("\n                      : ");

        write!(f, "{}", decoded_str)
    }
}

/// Try to get the event ABI(s) for the given event selector. Returns `None` if no event ABI is
/// found. Note: there may be multiple matching event signatures, so this function returns a Vec.
pub(crate) fn try_get_event_abi(selector: &B256, abis: &[JsonAbi]) -> Vec<Event> {
    abis.iter()
        .flat_map(|abi| abi.events.iter())
        .flat_map(|(_, events)| events.iter())
        .filter(|event| &event.selector() == selector)
        .cloned()
        .collect::<Vec<_>>()
}

pub(crate) fn get_abis(artifact_path: &PathBuf) -> Result<Vec<JsonAbi>> {
    // walk the artifact_path recursively and collect all `.hex` files
    walkdir::WalkDir::new(artifact_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().file_name().map(|f| f == "abi.json").unwrap_or(false))
        .map(|e| {
            // parse the json file
            let json = std::fs::read_to_string(e.path()).unwrap();
            let abi = serde_json::from_str::<JsonAbi>(&json)?;
            Ok::<_, eyre::Report>(abi)
        })
        .collect::<Result<_, _>>()
}
