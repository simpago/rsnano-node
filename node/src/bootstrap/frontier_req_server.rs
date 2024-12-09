use crate::{
    transport::{ResponseServer, ResponseServerExt},
    utils::ThreadPool,
};
use rsnano_core::{utils::seconds_since_epoch, Account, BlockHash};
use rsnano_ledger::Ledger;
use rsnano_messages::FrontierReq;
use rsnano_network::TrafficType;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, Weak},
};
use tracing::{debug, trace};

/// Server side of a frontier request. Created when a tcp_server receives a frontier_req message and exited when end-of-list is reached.
pub struct FrontierReqServer {
    server_impl: Arc<Mutex<FrontierReqServerImpl>>,
}

impl FrontierReqServer {
    pub fn new(
        connection: Arc<ResponseServer>,
        request: FrontierReq,
        thread_pool: Arc<dyn ThreadPool>,
        ledger: Arc<Ledger>,
        tokio: tokio::runtime::Handle,
    ) -> Self {
        let result = Self {
            server_impl: Arc::new(Mutex::new(FrontierReqServerImpl {
                connection,
                current: (request.start.number().overflowing_sub(1.into()).0).into(), // todo: figure out what underflow does
                frontier: BlockHash::zero(),
                request,
                count: 0,
                accounts: VecDeque::new(),
                thread_pool: Arc::downgrade(&thread_pool),
                ledger,
                tokio,
            })),
        };
        result.server_impl.lock().unwrap().next();
        result
    }

    pub fn send_next(&self) {
        let server_clone = Arc::clone(&self.server_impl);
        self.server_impl.lock().unwrap().send_next(server_clone);
    }

    pub fn current(&self) -> Account {
        self.server_impl.lock().unwrap().current
    }

    pub fn frontier(&self) -> BlockHash {
        self.server_impl.lock().unwrap().frontier
    }
}

struct FrontierReqServerImpl {
    connection: Arc<ResponseServer>,
    current: Account,
    frontier: BlockHash,
    request: FrontierReq,
    count: usize,
    accounts: VecDeque<(Account, BlockHash)>,
    thread_pool: Weak<dyn ThreadPool>,
    ledger: Arc<Ledger>,
    tokio: tokio::runtime::Handle,
}

impl FrontierReqServerImpl {
    pub fn send_confirmed(&self) -> bool {
        self.request.only_confirmed
    }

    pub fn send_next(&mut self, server: Arc<Mutex<FrontierReqServerImpl>>) {
        if !self.current.is_zero() && self.count < self.request.count as usize {
            trace!(
                account = %self.current,
                frontier = %self.frontier,
                socket = %self.connection.peer_addr(),
                "Sending frontier");

            let mut send_buffer = Vec::with_capacity(64);
            send_buffer.extend_from_slice(self.current.as_bytes());
            send_buffer.extend_from_slice(self.frontier.as_bytes());
            debug_assert!(!self.current.is_zero());
            debug_assert!(!self.frontier.is_zero());
            self.next();

            let conn = self.connection.clone();
            self.tokio.spawn(async move {
                match conn
                    .channel()
                    .send_buffer(&send_buffer, TrafficType::Generic)
                    .await
                {
                    Ok(()) => {
                        let server2 = server.clone();
                        server.lock().unwrap().sent_action(server2);
                    }
                    Err(e) => debug!("Error sending frontier pair: {:?}", e),
                }
            });
        } else {
            let connection = self.connection.clone();
            self.tokio.spawn(async move {
                Self::send_finished(connection).await;
            });
        }
    }

    async fn send_finished(connection: Arc<ResponseServer>) {
        let mut send_buffer = Vec::with_capacity(64);
        send_buffer.extend_from_slice(Account::zero().as_bytes());
        send_buffer.extend_from_slice(BlockHash::zero().as_bytes());
        debug!("Frontier sending finished");

        match connection
            .channel()
            .send_buffer(&send_buffer, TrafficType::Generic)
            .await
        {
            Ok(()) => {
                let connection = connection.clone();
                tokio::spawn(async move { connection.run().await });
            }
            Err(e) => debug!("Error sending frontier finish: {:?}", e),
        };
    }

    pub fn next(&mut self) {
        // Filling accounts deque to prevent often read transactions
        if self.accounts.is_empty() {
            let now = seconds_since_epoch();
            let disable_age_filter = self.request.age == u32::MAX;
            let max_size = 128;
            let transaction = self.ledger.read_txn();
            if !self.send_confirmed() {
                for (account, info) in self
                    .ledger
                    .any()
                    .accounts_range(&transaction, self.current.inc().unwrap_or_default()..)
                {
                    if self.accounts.len() >= max_size {
                        break;
                    }
                    if disable_age_filter || (now - info.modified) <= self.request.age as u64 {
                        self.accounts.push_back((account, info.head))
                    }
                }
            } else {
                let mut i = self.ledger.store.confirmation_height.begin_at_account(
                    &transaction,
                    &self.current.number().overflowing_add(1.into()).0.into(),
                );
                while let Some((account, info)) = i.current() {
                    if self.accounts.len() >= max_size {
                        break;
                    }

                    let confirmed_frontier = info.frontier;
                    if !confirmed_frontier.is_zero() {
                        self.accounts.push_back((*account, confirmed_frontier));
                    }

                    i.next();
                }
            }

            /* If loop breaks before max_size, then accounts_end () is reached. Add empty record to finish frontier_req_server */
            if self.accounts.len() != max_size {
                self.accounts
                    .push_back((Account::zero(), BlockHash::zero()));
            }
        }

        // Retrieving accounts from deque
        if let Some((account, frontier)) = self.accounts.pop_front() {
            self.current = account;
            self.frontier = frontier;
        }
    }

    pub fn sent_action(&mut self, server: Arc<Mutex<FrontierReqServerImpl>>) {
        let Some(thread_pool) = self.thread_pool.upgrade() else {
            return;
        };

        self.count += 1;
        thread_pool.post(Box::new(move || {
            let server_clone = Arc::clone(&server);
            server.lock().unwrap().send_next(server_clone);
        }));
    }
}
