use std::collections::HashMap;

use soroflare_vm::{contract_id, soroflare_utils};
use worker::{Request, RouteContext, Response, console_log};

use crate::{tasks::TaskRegistry, response::{BasicJsonResponse, JsonResponse}};

pub async fn handle(
    mut req: Request,
    _ctx: RouteContext<TaskRegistry<'_>>,
) -> Result<Response, worker::Error> {

    let incoming_raw = req.bytes().await;

    if incoming_raw.is_err() {
        return BasicJsonResponse::new("Error reading submitted data in body", 400).into();
    };

    let data = incoming_raw.unwrap();

    // validate WASM magic word
    if data.len() <= 4
        || !(data[0] == 0x00 && data[1] == 0x61 && data[2] == 0x73 && data[3] == 0x6d)
    {
        return BasicJsonResponse::new("Submitted data does not contain valid WASM code", 400)
            .into();
    }


    let mut state = soroflare_utils::empty_ledger_snapshot();

    let res = soroflare_vm::soroban_vm::deploy(&data.to_vec(), &contract_id!(0), &mut state);


    if res.is_err() {
        return BasicJsonResponse::new("Error deploying contract", 500).into();
    }
    
    
    let get_query: HashMap<_, _> = req.url()?.query_pairs().into_owned().collect();

    let fn_name = get_query.get("fn");

    if fn_name.is_none() {
        return BasicJsonResponse::new("Missing `fn` to execute", 400).into();
    }

    let res = soroflare_vm::soroban_vm::invoke(&contract_id!(0), fn_name.unwrap(), &Vec::new(), &mut state);

    if let Err(e) = res {
        return JsonResponse::new("Failed to execute contract", 400)
        .with_opt(e.to_string())
        .into();
    }

    let res = res.unwrap();

    let mut  resp = JsonResponse::new("Success", 200);
    resp = resp.with_opt(res.0);
    
    resp.into()

}