use anyhow::anyhow;
use futures::StreamExt;
use js_sys::Promise;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt::Write;
use subxt::config::substrate::Era;
use subxt::ext::codec::{Compact, Encode};
use subxt::{self, OnlineClient, PolkadotConfig};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use yew::{AttrValue, Callback};

#[subxt::subxt(runtime_metadata_path = "artifacts/kusama_metadata.scale")]
pub mod node_runtime {}

/// subscribes to finalized blocks. When a block is received, it is formatted as a string and sent via the callback.
pub(crate) async fn subscribe_to_finalized_blocks(
    cb: Callback<AttrValue>,
) -> Result<(), subxt::Error> {
    let api = OnlineClient::<PolkadotConfig>::from_url("wss://rpc.ibp.network/kusama").await?;

    // Subscribe to all finalized blocks:
    let mut blocks_sub = api.blocks().subscribe_finalized().await?;
    while let Some(block) = blocks_sub.next().await {
        let block = block?;
        let mut output = String::new();
        writeln!(output, "Block #{}:", block.header().number).ok();
        writeln!(output, "  Hash: {}", block.hash()).ok();
        cb.emit(output.into())
    }
    Ok(())
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = getAccounts)]
    pub fn js_get_accounts() -> Promise;
    #[wasm_bindgen(js_name = signPayload)]
    pub fn js_sign_payload(payload: String, source: String, address: String) -> Promise;
}

/// DTO to communicate with JavaScript
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Account {
    /// account name
    pub name: String,
    /// name of the browser extension
    pub source: String,
    /// the signature type, e.g. "sr25519" or "ed25519"
    pub ty: String,
    /// ss58 formatted address as string. Can be converted into AccountId32 via it's FromStr implementation.
    pub address: String,
}

pub async fn get_accounts() -> Result<Vec<Account>, anyhow::Error> {
    let result = JsFuture::from(js_get_accounts())
        .await
        .map_err(|js_err| anyhow!("{js_err:?}"))?;
    let accounts_str = result
        .as_string()
        .ok_or(anyhow!("Error converting JsValue into String"))?;
    let accounts: Vec<Account> = serde_json::from_str(&accounts_str)?;
    Ok(accounts)
}

fn to_hex(bytes: impl AsRef<[u8]>) -> String {
    format!("0x{}", hex::encode(bytes.as_ref()))
}

fn encode_then_hex<E: Encode>(input: &E) -> String {
    format!("0x{}", hex::encode(input.encode()))
}

/// communicates with JavaScript to obtain a signature for the `partial_extrinsic` via a browser extension (e.g. polkadot-js or Talisman)
///
/// Some parameters are hard-coded here and not taken from the partial_extrinsic itself (mortality_checkpoint, era, tip).
pub async fn extension_signature_for_extrinsic(
    call_data: &[u8],
    api: &OnlineClient<PolkadotConfig>,
    account_nonce: u64,
    account_source: String,
    account_address: String,
) -> Result<Vec<u8>, anyhow::Error> {
    let genesis_hash = encode_then_hex(&api.genesis_hash());
    // These numbers aren't SCALE encoded; their bytes are just converted to hex:
    let spec_version = to_hex(&api.runtime_version().spec_version.to_be_bytes());
    let transaction_version = to_hex(&api.runtime_version().transaction_version.to_be_bytes());
    let nonce = to_hex(&account_nonce.to_be_bytes());
    // If you construct a mortal transaction, then this block hash needs to correspond
    // to the block number passed to `Era::mortal()`.
    let mortality_checkpoint = encode_then_hex(&api.genesis_hash());
    let era = encode_then_hex(&Era::Immortal);
    let method = to_hex(call_data);
    let signed_extensions: Vec<String> = api
        .metadata()
        .extrinsic()
        .signed_extensions()
        .iter()
        .map(|e| e.identifier().to_string())
        .collect();
    let tip = encode_then_hex(&Compact(0u128));

    let payload = json!({
        "specVersion": spec_version,
        "transactionVersion": transaction_version,
        "address": account_address,
        "blockHash": mortality_checkpoint,
        "blockNumber": "0x00000000",
        "era": era,
        "genesisHash": genesis_hash,
        "method": method,
        "nonce": nonce,
        "signedExtensions": signed_extensions,
        "tip": tip,
        "version": 4,
    });

    let payload = payload.to_string();
    let result = JsFuture::from(js_sign_payload(payload, account_source, account_address))
        .await
        .map_err(|js_err| anyhow!("{js_err:?}"))?;
    let signature = result
        .as_string()
        .ok_or(anyhow!("Error converting JsValue into String"))?;
    let signature = hex::decode(&signature[2..])?;
    Ok(signature)
}
