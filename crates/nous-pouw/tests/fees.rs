//! Fee-market tests: tx fee is debited from sender + credited to leader.

use ed25519_dalek::SigningKey;
use nous_pouw::block::{Block, BlockBody, BlockHeader, sign_block};
use nous_pouw::state::{ChainState, WorkerId};
use nous_pouw::tx::{Transaction, TxBody};
use rand::rngs::OsRng;

fn worker_with_balance(state: &mut ChainState, balance: u64) -> (SigningKey, WorkerId) {
    let sk = SigningKey::generate(&mut OsRng);
    let id = WorkerId::from_verifying_key(&sk.verifying_key());
    state.register_worker(id, 0, 1.0);
    state.workers.get_mut(&id).unwrap().balance = balance;
    (sk, id)
}

fn block_from(leader_sk: &SigningKey, height: u64, prev: [u8; 32], txs: Vec<Transaction>) -> Block {
    let leader = WorkerId::from_verifying_key(&leader_sk.verifying_key());
    let body = BlockBody {
        certs: vec![],
        slashes: vec![],
        mints: vec![],
        transactions: txs,
    };
    let body_hash = body.hash();
    let mut header = BlockHeader {
        height,
        prev_hash: prev,
        state_root: [0; 32],
        body_hash,
        timestamp_ms: 0,
        leader,
        signature: vec![],
        parent_qc: None,
    };
    sign_block(&mut header, leader_sk);
    Block { header, body }
}

#[test]
fn transfer_fee_debits_sender_credits_leader_via_block() {
    let mut state = ChainState::new();
    let (sk_sender, sender) = worker_with_balance(&mut state, 1_000);
    let (_, recipient) = worker_with_balance(&mut state, 0);
    let (sk_leader, leader) = worker_with_balance(&mut state, 0);
    state.validators.insert(leader);

    let tx = Transaction::new_signed(
        TxBody::Transfer {
            from: sender,
            to: recipient,
            amount: 100,
        },
        1,
        25, // fee
        &sk_sender,
    );
    let block = block_from(&sk_leader, 1, state.head_hash, vec![tx]);
    state.apply_block(&block).unwrap();

    assert_eq!(state.workers[&sender].balance, 875); // 1000 - 100 - 25
    assert_eq!(state.workers[&recipient].balance, 100);
    assert_eq!(state.workers[&leader].balance, 25);
}

#[test]
fn fee_only_tx_debits_sender_credits_leader() {
    // RegisterValidator has no action_amount; the fee is the only debit.
    let mut state = ChainState::new();
    let (sk_sender, sender) = worker_with_balance(&mut state, 100);
    state.workers.get_mut(&sender).unwrap().stake = 1; // required for RegisterValidator
    let (sk_leader, leader) = worker_with_balance(&mut state, 0);
    state.validators.insert(leader);

    let tx = Transaction::new_signed(
        TxBody::RegisterValidator { worker: sender },
        1,
        7,
        &sk_sender,
    );
    let block = block_from(&sk_leader, 1, state.head_hash, vec![tx]);
    state.apply_block(&block).unwrap();
    assert_eq!(state.workers[&sender].balance, 93);
    assert_eq!(state.workers[&leader].balance, 7);
}

#[test]
fn insufficient_balance_for_amount_plus_fee_rejects_whole_tx() {
    let mut state = ChainState::new();
    let (sk_sender, sender) = worker_with_balance(&mut state, 100);
    let (_, recipient) = worker_with_balance(&mut state, 0);
    let (sk_leader, _leader) = worker_with_balance(&mut state, 0);

    // 100 - 90 = 10 left, not enough for fee=20.
    let tx = Transaction::new_signed(
        TxBody::Transfer {
            from: sender,
            to: recipient,
            amount: 90,
        },
        1,
        20,
        &sk_sender,
    );
    let block = block_from(&sk_leader, 1, state.head_hash, vec![tx]);
    let err = state.apply_block(&block).unwrap_err();
    assert!(format!("{err}").contains("insufficient balance"));
    // Sender unchanged (block rejected entirely).
    assert_eq!(state.workers[&sender].balance, 100);
}

#[test]
fn aggregated_fees_credit_leader_in_one_block() {
    let mut state = ChainState::new();
    let (sk_a, a) = worker_with_balance(&mut state, 1_000);
    let (sk_b, b) = worker_with_balance(&mut state, 1_000);
    let (_, c) = worker_with_balance(&mut state, 0);
    let (sk_leader, leader) = worker_with_balance(&mut state, 0);

    let tx1 = Transaction::new_signed(
        TxBody::Transfer {
            from: a,
            to: c,
            amount: 50,
        },
        1,
        5,
        &sk_a,
    );
    let tx2 = Transaction::new_signed(
        TxBody::Transfer {
            from: b,
            to: c,
            amount: 50,
        },
        1,
        15,
        &sk_b,
    );
    let block = block_from(&sk_leader, 1, state.head_hash, vec![tx1, tx2]);
    state.apply_block(&block).unwrap();
    assert_eq!(state.workers[&leader].balance, 20);
    assert_eq!(state.workers[&c].balance, 100);
}
