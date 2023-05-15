use std::{collections::HashMap, rc::Rc};

use serde::Serialize;
use soroban_env_host::{
    storage::Storage,
    xdr::{ScSpecTypeDef, ScVal},
};
use soroflare_vm::{
    soroban_cli::{self, strval::Spec},
    soroban_vm, soroflare_utils,
};
use worker::{console_debug, Request, Response, RouteContext};

use crate::{
    response::{BasicJsonResponse, JsonResponse},
    tasks::TaskRegistry,
};

pub async fn handle(
    mut req: Request,
    _ctx: RouteContext<TaskRegistry<'_>>,
) -> Result<Response, worker::Error> {
    let get_query: HashMap<String, String> = req.url()?.query_pairs().into_owned().collect();

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

    let fn_name = get_query.get("fn");

    if fn_name.is_none() {
        return BasicJsonResponse::new("Missing function name", 400).into();
    }

    let fn_name = fn_name.unwrap();

    let mut state = soroflare_utils::empty_ledger_snapshot();

    let dep_res = soroban_vm::deploy(&data, &soroflare_vm::contract_id!(0x00), &mut state);

    if dep_res.is_err() {
        return BasicJsonResponse::new(
            format!("Error deploying contract: {}", dep_res.err().unwrap()),
            500,
        )
        .into();
    }

    let snap = Rc::new(state.clone());

    let mut storage = Storage::with_recording_footprint(snap);

    let spec_entries = soroban_cli::utils::get_contract_spec_from_storage(
        &mut storage,
        soroflare_vm::contract_id!(0x00),
    );
    if spec_entries.is_err() {
        return BasicJsonResponse::new(
            format!("Failed reading contract: {}", spec_entries.err().unwrap()),
            500,
        )
        .into();
    }

    let spec_entries = spec_entries.unwrap();

    let spec = Spec(Some(spec_entries));

    let included_fns: HashMap<_, _> = spec
        .find_functions()
        .unwrap()
        .map(|spec| (spec.name.to_string_lossy(), spec))
        .collect();

    if !included_fns.contains_key(fn_name) {
        return BasicJsonResponse::new(
            format!(
                "The given function name is not present in the contract! Valid functions are: {}",
                included_fns
                    .keys()
                    .cloned()
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            400,
        )
        .into();
    }

    let fn_spec = spec.find_function(fn_name).unwrap();

    let arguments: Vec<(String, Option<String>)> = fn_spec
        .inputs
        .iter()
        .map(|i| {
            (
                i.name.to_string().unwrap(),
                get_query
                    .get(&i.name.to_string().unwrap())
                    .map(|s| s.to_owned()),
            )
        })
        .collect();

    let fn_types: HashMap<String, &ScSpecTypeDef> = fn_spec
        .inputs
        .iter()
        .map(|s| (s.name.to_string_lossy(), &s.type_))
        .collect();

    let missing_arguments: Vec<String> = arguments
        .iter()
        .filter(|(_, v)| v.is_none())
        .map(|v| v.0.clone())
        .map(|name| format!("{name} ({:?})", fn_types.get(&name).unwrap().name()))
        .collect();

    if !missing_arguments.is_empty() {
        return BasicJsonResponse::new(
            format!(
                "The given function is missing following parameters: {}",
                missing_arguments.join(", ")
            ),
            400,
        )
        .into();
    }

    let parsed_args: Vec<ScVal> = arguments
        .iter()
        .map(|(name, value)| {
            let value = value.clone().unwrap();
            let sc_type = fn_types.get(name).unwrap();
            spec.from_string(&value, sc_type).unwrap()
        })
        .collect();

    let invoke = soroban_vm::invoke(
        &soroflare_vm::contract_id!(0x00),
        fn_name,
        &parsed_args,
        &mut state,
    );

    if invoke.is_err() {
        return BasicJsonResponse::new(
            format!("Failed invoking contract: {}", invoke.err().unwrap()),
            500,
        )
        .into();
    }

    let invoke = invoke.unwrap();

    console_debug!("{:?}", (&invoke.0, &invoke.1 .1, &invoke.1 .2));

    #[derive(Serialize)]
    struct Response {
        result: ScVal,
        budget: String,
        events: String,
    }

    JsonResponse::new("success", 200)
        .with_opt(Response {
            result: invoke.0,
            budget: format!("{:?}", invoke.1 .1),
            events: format!("{:?}", invoke.1 .2),
        })
        .into()
}
