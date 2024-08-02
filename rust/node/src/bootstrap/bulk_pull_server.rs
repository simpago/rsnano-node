use crate::{
    transport::{ResponseServerExt, ResponseServerImpl, SocketExtensions, TrafficType},
    utils::{AsyncRuntime, ThreadPool},
};
use rsnano_core::{utils::MemoryStream, Account, BlockEnum, BlockHash, BlockType};
use rsnano_ledger::Ledger;
use rsnano_messages::BulkPull;
use std::sync::{Arc, Mutex, Weak};
use tracing::{debug, trace};

/**
 * Handle a request for the pull of all blocks associated with an account
 * The account is supplied as the "start" member, and the final block to
 * send is the "end" member.  The "start" member may also be a block
 * hash, in which case the that hash is used as the start of a chain
 * to send.  To determine if "start" is interpretted as an account or
 * hash, the ledger is checked to see if the block specified exists,
 * if not then it is interpretted as an account.
 *
 * Additionally, if "start" is specified as a block hash the range
 * is inclusive of that block hash, that is the range will be:
 * [start, end); In the case that a block hash is not specified the
 * range will be exclusive of the frontier for that account with
 * a range of (frontier, end)
 */
pub struct BulkPullServer {
    server_impl: Arc<Mutex<BulkPullServerImpl>>,
}

impl BulkPullServer {
    pub fn new(
        request: BulkPull,
        connection: Arc<ResponseServerImpl>,
        ledger: Arc<Ledger>,
        thread_pool: Arc<dyn ThreadPool>,
        runtime: Arc<AsyncRuntime>,
    ) -> Self {
        let mut server_impl = BulkPullServerImpl {
            include_start: false,
            sent_count: 0,
            max_count: 0,
            current: BlockHash::zero(),
            request,
            connection,
            ledger,
            thread_pool: Arc::downgrade(&thread_pool),
            runtime,
        };

        server_impl.set_current_end();
        Self {
            server_impl: Arc::new(Mutex::new(server_impl)),
        }
    }

    pub fn request(&self) -> BulkPull {
        self.server_impl.lock().unwrap().request.clone()
    }

    pub fn current(&self) -> BlockHash {
        self.server_impl.lock().unwrap().current
    }

    pub fn set_current(&self, value: BlockHash) {
        self.server_impl.lock().unwrap().current = value;
    }

    pub fn sent_count(&self) -> u32 {
        self.server_impl.lock().unwrap().sent_count
    }

    pub fn set_sent_count(&self, value: u32) {
        self.server_impl.lock().unwrap().sent_count = value;
    }

    pub fn max_count(&self) -> u32 {
        self.server_impl.lock().unwrap().max_count
    }

    pub fn set_max_count(&self, value: u32) {
        self.server_impl.lock().unwrap().max_count = value;
    }

    pub fn include_start(&self) -> bool {
        self.server_impl.lock().unwrap().include_start
    }

    pub fn set_include_start(&self, value: bool) {
        self.server_impl.lock().unwrap().include_start = value
    }

    pub fn get_next(&self) -> Option<BlockEnum> {
        self.server_impl.lock().unwrap().get_next()
    }

    pub fn send_next(&mut self) {
        let impl_clone = self.server_impl.clone();
        self.server_impl.lock().unwrap().send_next(impl_clone);
    }
}

struct BulkPullServerImpl {
    ledger: Arc<Ledger>,
    connection: Arc<ResponseServerImpl>,
    runtime: Arc<AsyncRuntime>,
    thread_pool: Weak<dyn ThreadPool>,
    include_start: bool,
    sent_count: u32,
    max_count: u32,
    current: BlockHash,
    request: BulkPull,
}

impl BulkPullServerImpl {
    fn ascending(&self) -> bool {
        self.request.ascending
    }

    fn set_current_end(&mut self) {
        self.include_start = false;
        let transaction = self.ledger.read_txn();
        if !self
            .ledger
            .store
            .block
            .exists(&transaction, &self.request.end)
        {
            debug!(
                "Bulk pull end block doesn't exist: {}, sending everything",
                self.request.end
            );
            self.request.end = BlockHash::zero();
        }

        let raw_block_exists = {
            let this = &self.ledger.store.block;
            let hash = &self.request.start.into();
            this.block_raw_get(&transaction, hash).is_some()
        };

        if raw_block_exists {
            debug!("Bulk pull request for block hash: {}", self.request.start);

            self.current = if self.ascending() {
                self.ledger
                    .any()
                    .block_successor(&transaction, &self.request.start.into())
                    .unwrap_or_default()
            } else {
                self.request.start.into()
            };
            self.include_start = true;
        } else if let Some(info) = self
            .ledger
            .account_info(&transaction, &self.request.start.into())
        {
            self.current = if self.ascending() {
                info.open_block
            } else {
                info.head
            };
            if !self.request.end.is_zero() {
                let account = self
                    .ledger
                    .any()
                    .block_account(&transaction, &self.request.end)
                    .unwrap_or_default();
                if account != self.request.start.into() {
                    debug!(
                        "Request for block that is not on account chain: {} not on {}",
                        self.request.end,
                        Account::from(self.request.start).encode_account()
                    );
                    self.current = self.request.end;
                }
            }
        } else {
            debug!(
                "Request for unknown account: {}",
                Account::from(self.request.start).encode_account()
            );
            self.current = self.request.end;
        }

        self.sent_count = 0;
        self.max_count = self.request.count;
    }

    pub fn get_next(&mut self) -> Option<BlockEnum> {
        let mut send_current = false;
        let mut set_current_to_end = false;

        /*
         * Determine if we should reply with a block
         *
         * If our cursor is on the final block, we should signal that we
         * are done by returning a null result.
         *
         * Unless we are including the "start" member and this is the
         * start member, then include it anyway.
         */
        if self.current != self.request.end {
            send_current = true;
        } else if self.current == self.request.end && self.include_start {
            send_current = true;

            /*
             * We also need to ensure that the next time
             * are invoked that we return a null result
             */
            set_current_to_end = true;
        }

        /*
         * Account for how many blocks we have provided.  If this
         * exceeds the requested maximum, return an empty object
         * to signal the end of results
         */
        if self.max_count != 0 && self.sent_count >= self.max_count {
            send_current = false;
        }

        let mut result = None;
        if send_current {
            {
                let txn = self.ledger.read_txn();
                result = self.ledger.any().get_block(&txn, &self.current);
            }

            if let Some(result) = &result {
                if !set_current_to_end {
                    let next = if self.ascending() {
                        result.successor().unwrap_or_default()
                    } else {
                        result.previous()
                    };
                    if !next.is_zero() {
                        self.current = next;
                    } else {
                        self.current = self.request.end;
                    }
                } else {
                    self.current = self.request.end;
                }
            } else {
                self.current = self.request.end;
            }

            self.sent_count += 1;
        }

        /*
         * Once we have processed "get_next()" once our cursor is no longer on
         * the "start" member, so this flag is not relevant is always false.
         */
        self.include_start = false;

        result
    }

    pub fn send_finished(&self, server_impl: Arc<Mutex<Self>>) {
        let send_buffer = Arc::new(vec![BlockType::NotABlock as u8]);
        debug!("Bulk sending finished");

        let conn = self.connection.clone();
        self.runtime.tokio.spawn(async move {
            match conn
                .socket
                .write(&send_buffer, TrafficType::Bootstrap)
                .await
            {
                Ok(()) => {
                    let guard = server_impl.lock().unwrap();
                    let connection = guard.connection.clone();
                    guard
                        .runtime
                        .tokio
                        .spawn(async move { connection.run().await });
                }
                Err(e) => debug!("Unable to send not-a-block ({:?})", e),
            }
        });
    }

    pub fn send_next(&mut self, server_impl: Arc<Mutex<Self>>) {
        if let Some(block) = self.get_next() {
            trace!(block = %block.hash(), remote = %self.connection.remote_endpoint(), "Sending block");
            let mut stream = MemoryStream::new();

            block.serialize(&mut stream);
            let send_buffer = Arc::new(stream.to_vec());
            let conn = self.connection.clone();
            self.runtime.tokio.spawn(async move {
                if conn
                    .socket
                    .write(&send_buffer, TrafficType::Bootstrap)
                    .await
                    .is_ok()
                {
                    let server_impl_clone = server_impl.clone();
                    server_impl.lock().unwrap().sent_action(server_impl_clone);
                } else {
                    debug!("Unable to bulk send block");
                }
            });
        } else {
            self.send_finished(server_impl);
        }
    }

    fn sent_action(&mut self, server_impl: Arc<Mutex<Self>>) {
        let Some(thread_pool) = self.thread_pool.upgrade() else {
            return;
        };
        thread_pool.push_task(Box::new(move || {
            let impl_clone = server_impl.clone();
            server_impl.lock().unwrap().send_next(impl_clone);
        }));
    }
}
