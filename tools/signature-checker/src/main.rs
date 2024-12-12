use rsnano_core::{Epochs, PublicKey, SavedBlock, Signature};
use rsnano_ledger::LedgerConstants;
use rsnano_store_lmdb::LmdbStore;
use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::{
        mpsc::{self, SyncSender},
        Arc, Mutex,
    },
    thread::{self, available_parallelism},
};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        println!("usage: signature-checker LEDGER_FILE_PATH");
        return;
    }

    let ledger_file = PathBuf::from(&args[1]);
    check_ledger_file(ledger_file);
}

fn check_ledger_file(ledger_file: impl AsRef<Path>) {
    let store = Arc::new(LmdbStore::open(ledger_file.as_ref()).build().unwrap());
    let tx = store.tx_begin_read();
    let total_blocks = store.block.count(&tx);
    let mut checked: u64 = 0;
    let problematic = Mutex::new(Vec::new());
    let epochs = LedgerConstants::live().epochs;
    let cpus = available_parallelism().unwrap();

    thread::scope(|s| {
        let mut queues: Vec<SyncSender<SavedBlock>> = Vec::new();

        for _ in 0..cpus.into() {
            let (tx, rx) = mpsc::sync_channel(1024);
            queues.push(tx);

            let probl = &problematic;
            let ep = &epochs;
            s.spawn(move || {
                while let Ok(block) = rx.recv() {
                    if is_problematic(&block, ep) {
                        probl.lock().unwrap().push(block)
                    }
                }
            });
        }

        println!("Checking signatures...");
        for block in store.block.iter(&tx) {
            if checked % 100_000 == 0 {
                print!(
                    "\r{}% done - {} found",
                    (checked * 100) / total_blocks,
                    problematic.lock().unwrap().len()
                );
                std::io::stdout().flush().unwrap();
            }

            queues[(checked % queues.len() as u64) as usize]
                .send(block.clone().into())
                .unwrap();
            checked += 1;
        }
    });

    println!();
    println!("These blocks are problematic:");
    for block in problematic.lock().unwrap().iter() {
        println!("{} {}", block.hash(), block.account().encode_account());
    }
}

fn is_problematic(block: &SavedBlock, epochs: &Epochs) -> bool {
    let signer = get_signer(block, epochs);
    validate_message(&signer, block.hash().as_bytes(), block.signature()).is_err()
}

fn get_signer(block: &SavedBlock, epochs: &Epochs) -> PublicKey {
    if block.is_epoch() {
        epochs
            .epoch_signer(&block.link_field().unwrap())
            .unwrap()
            .into()
    } else {
        block.account().into()
    }
}

pub fn validate_message(
    public_key: &PublicKey,
    message: &[u8],
    signature: &Signature,
) -> Result<(), ()> {
    let public =
        ed25519_dalek_blake2b::PublicKey::from_bytes(public_key.as_bytes()).map_err(|_| ())?;
    let sig = ed25519_dalek_blake2b::Signature::from_bytes(signature.as_bytes()).map_err(|_| ())?;
    public.verify_strict(message, &sig).map_err(|_| ())
}
