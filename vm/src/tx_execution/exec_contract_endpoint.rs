use std::rc::Rc;

use alloc::boxed::Box;
use multiversx_sc::err_msg;

use crate::{
    display_util::address_hex,
    tx_mock::{
        StaticVarStack, TxContext, TxContextRef, TxContextStack, TxFunctionName, TxPanic, TxResult,
    },
    world_mock::ContractContainer,
};

use super::catch_tx_panic;

/// Runs contract code using the auto-generated function selector.
/// The endpoint name is taken from the tx context.
/// Catches and wraps any panics thrown in the contract.
pub fn execute_tx_context(tx_context: TxContext) -> (TxContext, TxResult) {
    let tx_context_rc = Rc::new(tx_context);
    let (tx_context_rc, tx_result) = execute_tx_context_rc(tx_context_rc);
    let tx_context = Rc::try_unwrap(tx_context_rc).unwrap();
    (tx_context, tx_result)
}

/// The actual core of the execution.
/// The argument is returned and can be unwrapped,
/// since the lifetimes of all other references created from it cannot outlive this function.
fn execute_tx_context_rc(tx_context_rc: Rc<TxContext>) -> (Rc<TxContext>, TxResult) {
    let tx_context_ref = TxContextRef::new(tx_context_rc.clone());

    let func_name = &tx_context_ref.tx_input_box.func_name;
    let contract_identifier = get_contract_identifier(&tx_context_ref);
    let contract_map = &tx_context_rc.blockchain_ref().contract_map;

    let contract_container = contract_map.get_contract(contract_identifier.as_slice());

    TxContextStack::static_push(tx_context_rc.clone());
    StaticVarStack::static_push();
    let tx_result = execute_contract_instance_endpoint(contract_container, func_name);

    let tx_context_rc = TxContextStack::static_pop();
    StaticVarStack::static_pop();
    (tx_context_rc, tx_result)
}

fn get_contract_identifier(tx_context: &TxContext) -> Vec<u8> {
    tx_context
        .tx_cache
        .with_account(&tx_context.tx_input_box.to, |account| {
            account.contract_path.clone().unwrap_or_else(|| {
                panic!(
                    "Recipient account is not a smart contract {}",
                    address_hex(&tx_context.tx_input_box.to)
                )
            })
        })
}

/// The actual execution and the extraction/wrapping of results.
fn execute_contract_instance_endpoint(
    contract_container: &ContractContainer,
    endpoint_name: &TxFunctionName,
) -> TxResult {
    let result = catch_tx_panic(contract_container.panic_message, || {
        let call_successful = contract_container.call(endpoint_name);
        if call_successful {
            Ok(())
        } else {
            Err(TxPanic::new(1, "invalid function (not found)"))
        }
    });

    if let Err(tx_panic) = result {
        TxContextRef::new_from_static().replace_tx_result_with_error(tx_panic);
    }

    TxContextRef::new_from_static().into_tx_result()
}

/// Interprets a panic thrown during execution as a tx failure.
/// Note: specific tx outcomes from the debugger are signalled via specific panic objects.
pub fn interpret_panic_as_tx_result(
    panic_any: Box<dyn std::any::Any + std::marker::Send>,
    panic_message_flag: bool,
) -> TxPanic {
    if let Some(panic_obj) = panic_any.downcast_ref::<TxPanic>() {
        return panic_obj.clone();
    }

    if let Some(panic_string) = panic_any.downcast_ref::<String>() {
        return interpret_panic_str_as_tx_result(panic_string.as_str(), panic_message_flag);
    }

    if let Some(panic_string) = panic_any.downcast_ref::<&str>() {
        return interpret_panic_str_as_tx_result(panic_string, panic_message_flag);
    }

    TxPanic::user_error("unknown panic object")
}

pub fn interpret_panic_str_as_tx_result(panic_str: &str, panic_message_flag: bool) -> TxPanic {
    if panic_message_flag {
        TxPanic::user_error(&format!("panic occurred: {panic_str}"))
    } else {
        TxPanic::user_error(err_msg::PANIC_OCCURRED)
    }
}
