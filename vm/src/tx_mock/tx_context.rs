use crate::{
    tx_execution::BlockchainVMRef,
    types::VMAddress,
    world_mock::{AccountData, AccountEsdt, BlockchainState, FailingExecutor},
};
use core::cell::RefCell;
use num_bigint::BigUint;
use num_traits::Zero;
use std::{
    cell::{Ref, RefMut},
    collections::HashMap,
    rc::Rc,
};

use super::{BlockchainRng, BlockchainUpdate, TxCache, TxInput, TxManagedTypes, TxResult};

pub struct TxContext {
    pub vm_ref: BlockchainVMRef,
    pub tx_input_box: Box<TxInput>,
    pub tx_cache: Rc<TxCache>,
    pub managed_types: RefCell<TxManagedTypes>,
    pub tx_result_cell: RefCell<TxResult>,
    pub b_rng: RefCell<BlockchainRng>,
}

impl TxContext {
    pub fn new(vm_ref: BlockchainVMRef, tx_input: TxInput, tx_cache: TxCache) -> Self {
        let b_rng = RefCell::new(BlockchainRng::new(&tx_input, &tx_cache));
        TxContext {
            vm_ref,
            tx_input_box: Box::new(tx_input),
            tx_cache: Rc::new(tx_cache),
            managed_types: RefCell::new(TxManagedTypes::new()),
            tx_result_cell: RefCell::new(TxResult::empty()),
            b_rng,
        }
    }

    pub fn dummy() -> Self {
        let tx_cache = TxCache::new(Rc::new(BlockchainState::default()));
        let contract_address = VMAddress::from([b'c'; 32]);
        tx_cache.insert_account(AccountData {
            address: contract_address.clone(),
            nonce: 0,
            egld_balance: BigUint::zero(),
            storage: HashMap::new(),
            esdt: AccountEsdt::default(),
            username: Vec::new(),
            contract_path: None,
            contract_owner: None,
            developer_rewards: BigUint::zero(),
        });

        let tx_input = TxInput {
            from: contract_address.clone(),
            to: contract_address,
            tx_hash: b"dummy...........................".into(),
            ..Default::default()
        };

        let b_rng = RefCell::new(BlockchainRng::new(&tx_input, &tx_cache));
        TxContext {
            vm_ref: BlockchainVMRef::new(Box::new(FailingExecutor)),
            tx_input_box: Box::new(tx_input),
            tx_cache: Rc::new(tx_cache),
            managed_types: RefCell::new(TxManagedTypes::new()),
            tx_result_cell: RefCell::new(TxResult::empty()),
            b_rng,
        }
    }

    pub fn input_ref(&self) -> &TxInput {
        self.tx_input_box.as_ref()
    }

    pub fn blockchain_cache(&self) -> &TxCache {
        &self.tx_cache
    }

    pub fn blockchain_cache_rc(&self) -> Rc<TxCache> {
        self.tx_cache.clone()
    }

    pub fn blockchain_ref(&self) -> &BlockchainState {
        self.tx_cache.blockchain_ref()
    }

    pub fn with_account<R, F>(&self, address: &VMAddress, f: F) -> R
    where
        F: FnOnce(&AccountData) -> R,
    {
        self.tx_cache.with_account(address, f)
    }

    pub fn with_contract_account<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&AccountData) -> R,
    {
        self.with_account(&self.tx_input_box.to, f)
    }

    pub fn with_account_mut<R, F>(&self, address: &VMAddress, f: F) -> R
    where
        F: FnOnce(&mut AccountData) -> R,
    {
        self.tx_cache.with_account_mut(address, f)
    }

    pub fn with_contract_account_mut<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&mut AccountData) -> R,
    {
        self.with_account_mut(&self.tx_input_box.to, f)
    }

    pub fn m_types_borrow(&self) -> Ref<TxManagedTypes> {
        self.managed_types.borrow()
    }

    pub fn m_types_borrow_mut(&self) -> RefMut<TxManagedTypes> {
        self.managed_types.borrow_mut()
    }

    pub fn result_borrow_mut(&self) -> RefMut<TxResult> {
        self.tx_result_cell.borrow_mut()
    }

    pub fn extract_result(&self) -> TxResult {
        self.tx_result_cell.replace(TxResult::empty())
    }

    pub fn rng_borrow_mut(&self) -> RefMut<BlockchainRng> {
        self.b_rng.borrow_mut()
    }

    pub fn create_new_contract(
        &self,
        new_address: &VMAddress,
        contract_path: Vec<u8>,
        contract_owner: VMAddress,
    ) {
        assert!(
            !self.tx_cache.blockchain_ref().account_exists(new_address),
            "Account already exists at deploy address."
        );

        self.tx_cache.insert_account(AccountData {
            address: new_address.clone(),
            nonce: 0,
            egld_balance: BigUint::zero(),
            storage: HashMap::new(),
            esdt: AccountEsdt::default(),
            username: Vec::new(),
            contract_path: Some(contract_path),
            contract_owner: Some(contract_owner),
            developer_rewards: BigUint::zero(),
        });
    }

    pub fn into_blockchain_updates(self) -> BlockchainUpdate {
        let tx_cache = Rc::try_unwrap(self.tx_cache).unwrap();
        tx_cache.into_blockchain_updates()
    }

    pub fn into_results(self) -> (TxResult, BlockchainUpdate) {
        let tx_cache = Rc::try_unwrap(self.tx_cache).unwrap();
        let tx_result = RefCell::into_inner(self.tx_result_cell);
        let blockchain_updates = tx_cache.into_blockchain_updates();
        (tx_result, blockchain_updates)
    }
}

impl std::fmt::Debug for TxContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TxContext")
            .field("tx_input_box", &self.tx_input_box)
            .field("tx_cache", &self.tx_cache)
            .field("managed_types", &self.managed_types)
            .field("tx_result_cell", &self.tx_result_cell)
            .field("b_rng", &self.b_rng)
            .finish()
    }
}
