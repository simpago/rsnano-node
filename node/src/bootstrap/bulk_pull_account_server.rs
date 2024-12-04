use crate::{
    transport::{ResponseServer, ResponseServerExt},
    utils::ThreadPool,
};
use rsnano_core::{Account, Amount, BlockHash, PendingInfo, PendingKey};
use rsnano_ledger::Ledger;
use rsnano_messages::{BulkPullAccount, BulkPullAccountFlags};
use rsnano_network::TrafficType;
use std::{
    collections::HashSet,
    sync::{Arc, Mutex, Weak},
};
use tracing::{debug, trace};

struct BulkPullAccountServerImpl {
    connection: Arc<ResponseServer>,
    request: BulkPullAccount,
    thread_pool: Weak<dyn ThreadPool>,
    ledger: Arc<Ledger>,
    deduplication: HashSet<Account>,
    current_key: PendingKey,
    pending_address_only: bool,
    pending_include_address: bool,
    invalid_request: bool,
    tokio: tokio::runtime::Handle,
}

impl BulkPullAccountServerImpl {
    /**
     * Bulk pull blocks related to an account
     */
    fn set_params(&mut self) {
        // Parse the flags
        self.invalid_request = false;
        self.pending_include_address = false;
        self.pending_address_only = false;
        if self.request.flags == BulkPullAccountFlags::PendingAddressOnly {
            self.pending_address_only = true;
        } else if self.request.flags == BulkPullAccountFlags::PendingHashAmountAndAddress {
            // This is the same as "pending_hash_and_amount" but with the
            // sending address appended, for UI purposes mainly.
            self.pending_include_address = true;
        } else if self.request.flags == BulkPullAccountFlags::PendingHashAndAmount {
            // The defaults are set above
        } else {
            debug!(
                "Invalid bulk_pull_account flags supplied {:?}",
                self.request.flags
            );

            self.invalid_request = true;

            return;
        }

        /*
         * Initialize the current item from the requested account
         */
        self.current_key.receiving_account = self.request.account;
        self.current_key.send_block_hash = BlockHash::zero();
    }

    fn send_frontier(&self, server: Arc<Mutex<BulkPullAccountServerImpl>>) {
        /*
         * This function is really the entry point into this class,
         * so handle the invalid_request case by terminating the
         * request without any response
         */
        if !self.invalid_request {
            let stream_transaction = self.ledger.read_txn();

            // Get account balance and frontier block hash
            let account_frontier_hash = self
                .ledger
                .any()
                .account_head(&stream_transaction, &self.request.account)
                .unwrap_or_default();
            let account_frontier_balance = self
                .ledger
                .any()
                .account_balance(&stream_transaction, &self.request.account)
                .unwrap_or_default();

            // Write the frontier block hash and balance into a buffer
            let mut send_buffer = Vec::new();
            {
                send_buffer.extend_from_slice(account_frontier_hash.as_bytes());
                send_buffer.extend_from_slice(&account_frontier_balance.to_be_bytes());
            }

            let connection_l = self.connection.clone();
            let workers = self.thread_pool.clone();
            // Send the buffer to the requestor
            self.tokio.spawn(async move {
                if connection_l
                    .channel()
                    .send_buffer(&send_buffer, TrafficType::Bootstrap)
                    .await
                    .is_ok()
                {
                    if let Some(workers) = workers.upgrade() {
                        workers.push_task(Box::new(move || {
                            let server2 = Arc::clone(&server);
                            server.lock().unwrap().send_next_block(server2);
                        }));
                    }
                } else {
                    debug!("Unable to bulk send block");
                }
            });
        }
    }

    fn send_next_block(&mut self, server: Arc<Mutex<BulkPullAccountServerImpl>>) {
        /*
         * Get the next item from the queue, it is a tuple with the key (which
         * contains the account and hash) and data (which contains the amount)
         */
        if let Some((block_info_key, block_info)) = self.get_next() {
            /*
             * If we have a new item, emit it to the socket
             */

            let mut send_buffer = Vec::new();
            if self.pending_address_only {
                trace!(pending = %block_info.source, "Sending pending");
                send_buffer.extend_from_slice(block_info.source.as_bytes());
            } else {
                trace!(block = %block_info_key.send_block_hash, "Sending block");
                send_buffer.extend_from_slice(block_info_key.send_block_hash.as_bytes());
                send_buffer.extend_from_slice(&block_info.amount.to_be_bytes());

                if self.pending_include_address {
                    // Write the source address as well, if requested
                    send_buffer.extend_from_slice(block_info.source.as_bytes());
                }
            }

            let connection = self.connection.clone();
            let workers = self.thread_pool.clone();
            self.tokio.spawn(async move {
                if connection
                    .channel()
                    .send_buffer(&send_buffer, TrafficType::Bootstrap)
                    .await
                    .is_ok()
                {
                    if let Some(workers) = workers.upgrade() {
                        workers.push_task(Box::new(move || {
                            let server2 = Arc::clone(&server);
                            server.lock().unwrap().send_next_block(server2);
                        }));
                    }
                } else {
                    debug!("Unable to bulk send block");
                }
            });
        } else {
            /*
             * Otherwise, finalize the connection
             */
            debug!("Done sending blocks");

            let conn = self.connection.clone();
            let pending_address_only = self.pending_address_only;
            let pending_include_address = self.pending_include_address;
            self.tokio.spawn(async move {
                send_finished(&conn, pending_address_only, pending_include_address).await;
            });
        }
    }

    fn get_next(&mut self) -> Option<(PendingKey, PendingInfo)> {
        loop {
            /*
             * For each iteration of this loop, establish and then
             * destroy a database transaction, to avoid locking the
             * database for a prolonged period.
             */
            let tx = self.ledger.read_txn();
            let mut stream = self.ledger.any().account_receivable_upper_bound(
                &tx,
                self.current_key.receiving_account,
                self.current_key.send_block_hash,
            );

            let Some((key, info)) = stream.next() else {
                break;
            };

            self.current_key = key.clone();

            /*
             * Skip entries where the amount is less than the requested
             * minimum
             */
            if info.amount < self.request.minimum_amount {
                continue;
            }

            /*
             * If the pending_address_only flag is set, de-duplicate the
             * responses.  The responses are the address of the sender,
             * so they are part of the pending table's information
             * and not key, so we have to de-duplicate them manually.
             */
            if self.pending_address_only {
                if !self.deduplication.insert(info.source) {
                    /*
                     * If the deduplication map gets too
                     * large, clear it out.  This may
                     * result in some duplicates getting
                     * sent to the client, but we do not
                     * want to commit too much memory
                     */
                    if self.deduplication.len() > 4096 {
                        self.deduplication.clear();
                    }
                    continue;
                }
            }

            return Some((key.clone(), info.clone()));
        }

        None
    }
}

async fn send_finished(
    connection: &Arc<ResponseServer>,
    pending_address_only: bool,
    pending_include_address: bool,
) {
    /*
     * The "bulk_pull_account" final sequence is a final block of all
     * zeros.  If we are sending only account public keys (with the
     * "pending_address_only" flag) then it will be 256-bits of zeros,
     * otherwise it will be either 384-bits of zeros (if the
     * "pending_include_address" flag is not set) or 640-bits of zeros
     * (if that flag is set).
     */
    let mut send_buffer = Vec::new();
    {
        send_buffer.extend_from_slice(Account::zero().as_bytes());

        if !pending_address_only {
            send_buffer.extend_from_slice(&Amount::zero().to_be_bytes());
            if pending_include_address {
                send_buffer.extend_from_slice(Account::zero().as_bytes());
            }
        }
    }

    if connection
        .channel()
        .send_buffer(&send_buffer, TrafficType::Bootstrap)
        .await
        .is_ok()
    {
        let conn = connection.clone();
        tokio::spawn(async move { conn.run().await });
    } else {
        debug!("Could not finish bulk pull");
    }
}

pub struct BulkPullAccountServer {
    server: Arc<Mutex<BulkPullAccountServerImpl>>,
}

impl BulkPullAccountServer {
    pub fn new(
        connection: Arc<ResponseServer>,
        request: BulkPullAccount,
        thread_pool: Arc<dyn ThreadPool>,
        ledger: Arc<Ledger>,
        tokio: tokio::runtime::Handle,
    ) -> Self {
        let mut server = BulkPullAccountServerImpl {
            connection,
            request,
            thread_pool: Arc::downgrade(&thread_pool),
            ledger,
            deduplication: HashSet::new(),
            current_key: PendingKey::new(Account::zero(), BlockHash::zero()),
            pending_address_only: false,
            pending_include_address: false,
            invalid_request: false,
            tokio,
        };
        /*
         * Setup the streaming response for the first call to "send_frontier" and  "send_next_block"
         */
        server.set_params();
        Self {
            server: Arc::new(Mutex::new(server)),
        }
    }

    pub fn send_frontier(&self) {
        let server2 = Arc::clone(&self.server);
        self.server.lock().unwrap().send_frontier(server2);
    }

    pub fn get_next(&self) -> Option<(PendingKey, PendingInfo)> {
        self.server.lock().unwrap().get_next()
    }

    pub fn current_key(&self) -> PendingKey {
        self.server.lock().unwrap().current_key.clone()
    }

    pub fn pending_address_only(&self) -> bool {
        self.server.lock().unwrap().pending_address_only
    }

    pub fn pending_include_address(&self) -> bool {
        self.server.lock().unwrap().pending_include_address
    }

    pub fn invalid_request(&self) -> bool {
        self.server.lock().unwrap().invalid_request
    }
}
