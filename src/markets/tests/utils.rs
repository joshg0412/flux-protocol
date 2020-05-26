#[macro_use]
use near_crypto::{InMemorySigner, KeyType, Signer};
use near_runtime_standalone::{init_runtime_and_signer, RuntimeStandalone};
use near_primitives::{
    account::{AccessKey, Account},
    errors::{RuntimeError, TxExecutionError},
    hash::CryptoHash,
    transaction::{ExecutionOutcome, ExecutionStatus, Transaction},
    types::{AccountId, Balance},
};

use std::collections::{HashMap};
use super::markets;
type Order = markets::market::orderbook::order::Order;

use serde_json::json;

type TxResult = Result<ExecutionOutcome, ExecutionOutcome>;

lazy_static::lazy_static! {
    static ref MARKETS_BYTES: &'static [u8] = include_bytes!("../res/flux_protocol.wasm").as_ref();
    static ref FUNGIBLE_TOKEN_BYTES: &'static [u8] = include_bytes!("../../res/fungible_token.wasm").as_ref();
}

pub fn ntoy(near_amount: Balance) -> Balance {
    near_amount * 10u128.pow(24)
}

fn outcome_into_result(outcome: ExecutionOutcome) -> TxResult {
    match outcome.status {
        ExecutionStatus::SuccessValue(_) => Ok(outcome),
        ExecutionStatus::Failure(_) => Err(outcome),
        ExecutionStatus::SuccessReceiptId(_) => panic!("Unresolved ExecutionOutcome run runtime.resolve(tx) to resolve the final outcome of tx"),
        ExecutionStatus::Unknown => unreachable!()
    }
}

pub struct ExternalUser {
    account_id: AccountId,
    signer: InMemorySigner,
}

impl ExternalUser {
    pub fn new(account_id: AccountId, signer: InMemorySigner) -> Self {
        Self { account_id, signer }
    }

    pub fn markets_init_new(&self, runtime: &mut RuntimeStandalone) -> TxResult {
            let args = json!({}).to_string().as_bytes().to_vec();

            // TODO: REPLACE POOL_ACCOUNT_ID with the correct destination address for contract
            let tx = self
                .new_tx(runtime, "flux-tests".to_string())
                .create_account()
                .deploy_contract(MARKETS_BYTES.to_vec())
                .function_call("default".into(), args, 10000000000000000, 0)
                .sign(&self.signer);
            let res = runtime.resolve_tx(tx).unwrap();
            runtime.process_all().unwrap();
            outcome_into_result(res)
        }

    fn new_tx(&self, runtime: &RuntimeStandalone, receiver_id: AccountId) -> Transaction {
        let nonce = runtime
            .view_access_key(&self.account_id, &self.signer.public_key())
            .unwrap()
            .nonce
            + 1;
        Transaction::new(
            self.account_id.clone(),
            self.signer.public_key(),
            receiver_id,
            nonce,
            CryptoHash::default(),
        )
    }

    pub fn claim_fdai(&self, runtime: &mut RuntimeStandalone) -> TxResult {
        let args = json!({})
            .to_string()
            .as_bytes()
            .to_vec();

        let tx = self
            .new_tx(runtime, "flux-tests".to_string())
            .function_call("claim_fdai".into(), args, 10000000000000000, 0)
            .sign(&self.signer);
        let res = runtime.resolve_tx(tx).unwrap();
        runtime.process_all().unwrap();
        outcome_into_result(res)
    }

    pub fn create_market(
        &self,
        runtime: &mut RuntimeStandalone,
        description: String,
        extra_info: String,
        outcomes: u64,
        outcome_tags: Vec<String>,
        categories: Vec<String>,
        end_time: u64,
        fee_percentage: u128,
        cost_percentage: u128,
        api_source: String,
    ) -> TxResult {
        let args = json!({
            "description": description,
            "extra_info": extra_info,
            "outcomes": outcomes,
            "categories": categories,
            "end_time": end_time,
            "fee_percentage": fee_percentage,
            "cost_percentage": cost_percentage,
            "api_source": api_source,
        })
            .to_string()
            .as_bytes()
            .to_vec();
        // TODO: DECIDE WHERE TO SEND TRANSACTION
        let tx = self
            .new_tx(runtime, "flux-tests".to_string())
            .function_call("create_market".into(), args, 10000000000000000, 0)
            .sign(&self.signer);
        let res = runtime.resolve_tx(tx).unwrap();
        runtime.process_all().unwrap();
        outcome_into_result(res)
    }

    pub fn place_order(
        &self,
        runtime: &mut RuntimeStandalone,
        market_id: u64,
        outcome: u64,
        spend: u128,
        price: u128,
    ) -> TxResult {
        let args = json!({
            "market_id": market_id,
            "outcome": outcome,
            "spend": spend,
            "price": price,
        })
            .to_string()
            .as_bytes()
            .to_vec();
        // TODO: UPDATE WHERE TO SEND TX TO
        let tx = self
            .new_tx(runtime, "flux-tests".to_string())
            .function_call("place_order".into(), args, 10000000000000000, 0)
            .sign(&self.signer);
        let res = runtime.resolve_tx(tx).unwrap();
        runtime.process_all().unwrap();
        outcome_into_result(res)
    }

    pub fn get_open_orders(&self, runtime: &RuntimeStandalone, market_id: u64, outcome: u64) -> &HashMap<u128, Order> {
        // TODO: SPECIFY WHAT ACCOUNT TO CALL TO
        let open_orders = runtime
            .view_method_call(
                &("flux-tests".to_string()),
                "get_open_orders",
                json!({"market_id": market_id, "outcome": outcome})
                    .to_string()
                    .as_bytes(),
            )
            .unwrap()
            .0;
        //TODO: UPDATE THIS CASTING
        &HashMap<u128, Order>::from(serde_json::from_slice::HashMap<u128, Order>(open_orders.as_slice()).unwrap())
    }

    pub fn get_filled_orders(&self, runtime: &RuntimeStandalone, market_id: u64, outcome: u64) -> &HashMap<u128, Order> {
        let filled_orders = runtime
            .view_method_call(
                &("flux-tests".to_string()),
                "get_filled_orders",
                json!({"market_id": market_id, "outcome": outcome})
                    .to_string()
                    .as_bytes(),
            )
            .unwrap()
            .0;
        //TODO: UPDATE THIS CASTING
        &HashMap<u128, Order>::from(serde_json::from_slice::HashMap<u128, Order>(open_orders.as_slice()).unwrap())
    }

}

pub fn init_markets_contract() -> (RuntimeStandalone, ExternalUser) {
    let (mut runtime, signer) = init_runtime_and_signer(&"root".into());
    let root = ExternalUser::new("root".into(), signer);

    root.markets_init_new(&mut runtime).unwrap();
    return (runtime, root);
}