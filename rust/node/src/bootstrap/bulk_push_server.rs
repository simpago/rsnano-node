use super::BootstrapInitiator;
use crate::{
    block_processing::{BlockProcessor, BlockSource},
    stats::{DetailType, Direction, StatType, Stats},
    transport::{AsyncBufferReader, ResponseServer, ResponseServerExt},
    utils::{AsyncRuntime, ErrorCode, ThreadPool},
};
use num_traits::FromPrimitive;
use rsnano_core::{
    utils::BufferReader, work::WorkThresholds, BlockEnum, BlockType, ChangeBlock, OpenBlock,
    ReceiveBlock, SendBlock, StateBlock,
};
use std::{
    sync::{Arc, Mutex, Weak},
    time::Duration,
};
use tokio::task::spawn_blocking;
use tracing::debug;

/// Server side of a bulk_push request. Receives blocks and puts them in the block processor to be processed.
pub struct BulkPushServer {
    server_impl: Arc<Mutex<BulkPushServerImpl>>,
}

const BUFFER_SIZE: usize = 256;

impl BulkPushServer {
    pub fn new(
        async_rt: Arc<AsyncRuntime>,
        connection: Arc<ResponseServer>,
        thread_pool: Arc<dyn ThreadPool>,
        block_processor: Arc<BlockProcessor>,
        bootstrap_initiator: Arc<BootstrapInitiator>,
        stats: Arc<Stats>,
        work_thresholds: WorkThresholds,
    ) -> Self {
        let server_impl = BulkPushServerImpl {
            async_rt,
            connection,
            thread_pool: Arc::downgrade(&thread_pool),
            block_processor: Arc::downgrade(&block_processor),
            receive_buffer: Arc::new(Mutex::new(vec![0; BUFFER_SIZE])),
            bootstrap_initiator: Arc::downgrade(&bootstrap_initiator),
            stats,
            work_thresholds,
        };

        Self {
            server_impl: Arc::new(Mutex::new(server_impl)),
        }
    }

    pub fn throttled_receive(&self) {
        let server_impl2 = Arc::clone(&self.server_impl);
        self.server_impl
            .lock()
            .unwrap()
            .throttled_receive(server_impl2);
    }
}

struct BulkPushServerImpl {
    async_rt: Arc<AsyncRuntime>,
    connection: Arc<ResponseServer>,
    thread_pool: Weak<dyn ThreadPool>,
    block_processor: Weak<BlockProcessor>,
    receive_buffer: Arc<Mutex<Vec<u8>>>,
    bootstrap_initiator: Weak<BootstrapInitiator>,
    stats: Arc<Stats>,
    work_thresholds: WorkThresholds,
}

impl BulkPushServerImpl {
    fn throttled_receive(&self, server_impl: Arc<Mutex<Self>>) {
        let Some(thread_pool) = self.thread_pool.upgrade() else {
            return;
        };
        let Some(block_processor) = self.block_processor.upgrade() else {
            return;
        };
        if block_processor.queue_len(BlockSource::BootstrapLegacy) < 1024 {
            self.receive(server_impl);
        } else {
            thread_pool.add_delayed_task(
                Duration::from_secs(1),
                Box::new(move || {
                    let server_impl2 = Arc::clone(&server_impl);
                    let guard = server_impl.lock().unwrap();
                    if !guard.connection.is_stopped() {
                        guard.throttled_receive(server_impl2);
                    }
                }),
            );
        }
    }

    fn receive(&self, server_impl: Arc<Mutex<Self>>) {
        let Some(bootstrap_initiator) = self.bootstrap_initiator.upgrade() else {
            return;
        };
        if bootstrap_initiator.in_progress() {
            debug!("Aborting bulk_push because a bootstrap attempt is in progress");
        } else {
            let channel = Arc::clone(&self.connection.channel());
            let buffer = Arc::clone(&self.receive_buffer);
            let server_impl2 = Arc::clone(&server_impl);
            self.async_rt.tokio.spawn(async move {
                let mut buf = [0; BUFFER_SIZE];
                let result = channel.read(&mut buf, 1).await;
                buffer.lock().unwrap().copy_from_slice(&buf);
                spawn_blocking(Box::new(move || {
                    let guard = server_impl.lock().unwrap();
                    match result {
                        Ok(()) => {
                            guard.received_type(server_impl2);
                        }
                        Err(e) => {
                            debug!("Error receiving block type: {:?}", e);
                        }
                    }
                }));
            });
        }
    }

    fn received_type(&self, server_impl: Arc<Mutex<Self>>) {
        let stats = Arc::clone(&self.stats);
        let server_impl2 = Arc::clone(&server_impl);
        let block_type = { BlockType::from_u8(self.receive_buffer.lock().unwrap()[0]) };
        let channel = Arc::clone(&self.connection.channel());
        let buffer = Arc::clone(&self.receive_buffer);

        match block_type {
            Some(BlockType::NotABlock) => {
                let connection = self.connection.clone();
                self.async_rt
                    .tokio
                    .spawn(async move { connection.run().await });
                return;
            }
            Some(BlockType::Invalid) | None => {
                debug!("Unknown type received as block type");
                return;
            }
            _ => {}
        }

        self.async_rt.tokio.spawn(async move {
            let mut buf = [0; BUFFER_SIZE];
            match block_type {
                Some(BlockType::LegacySend) => {
                    stats.inc_dir(StatType::Bootstrap, DetailType::Send, Direction::In);
                    let result = channel.read(&mut buf, SendBlock::serialized_size()).await;
                    buffer.lock().unwrap().copy_from_slice(&buf);
                    let ec;
                    let len;
                    match result {
                        Ok(()) => {
                            ec = ErrorCode::new();
                            len = SendBlock::serialized_size();
                        }
                        Err(_) => {
                            ec = ErrorCode::fault();
                            len = 0;
                        }
                    }

                    spawn_blocking(Box::new(move || {
                        server_impl.lock().unwrap().received_block(
                            server_impl2,
                            ec,
                            len,
                            BlockType::LegacySend,
                        );
                    }));
                }
                Some(BlockType::LegacyReceive) => {
                    stats.inc_dir(StatType::Bootstrap, DetailType::Receive, Direction::In);
                    let result = channel
                        .read(&mut buf, ReceiveBlock::serialized_size())
                        .await;
                    buffer.lock().unwrap().copy_from_slice(&buf);
                    let ec;
                    let len;
                    match result {
                        Ok(()) => {
                            ec = ErrorCode::new();
                            len = SendBlock::serialized_size();
                        }
                        Err(_) => {
                            ec = ErrorCode::fault();
                            len = 0;
                        }
                    }
                    spawn_blocking(Box::new(move || {
                        server_impl.lock().unwrap().received_block(
                            server_impl2,
                            ec,
                            len,
                            BlockType::LegacyReceive,
                        );
                    }));
                }
                Some(BlockType::LegacyOpen) => {
                    stats.inc_dir(StatType::Bootstrap, DetailType::Open, Direction::In);
                    let result = channel.read(&mut buf, OpenBlock::serialized_size()).await;
                    buffer.lock().unwrap().copy_from_slice(&buf);
                    let ec;
                    let len;
                    match result {
                        Ok(()) => {
                            ec = ErrorCode::new();
                            len = SendBlock::serialized_size();
                        }
                        Err(_) => {
                            ec = ErrorCode::fault();
                            len = 0;
                        }
                    }
                    spawn_blocking(Box::new(move || {
                        server_impl.lock().unwrap().received_block(
                            server_impl2,
                            ec,
                            len,
                            BlockType::LegacyOpen,
                        );
                    }));
                }
                Some(BlockType::LegacyChange) => {
                    stats.inc_dir(StatType::Bootstrap, DetailType::Change, Direction::In);
                    let result = channel.read(&mut buf, ChangeBlock::serialized_size()).await;
                    buffer.lock().unwrap().copy_from_slice(&buf);
                    let ec;
                    let len;
                    match result {
                        Ok(()) => {
                            ec = ErrorCode::new();
                            len = SendBlock::serialized_size();
                        }
                        Err(_) => {
                            ec = ErrorCode::fault();
                            len = 0;
                        }
                    }
                    spawn_blocking(Box::new(move || {
                        server_impl.lock().unwrap().received_block(
                            server_impl2,
                            ec,
                            len,
                            BlockType::LegacyChange,
                        );
                    }));
                }
                Some(BlockType::State) => {
                    stats.inc_dir(StatType::Bootstrap, DetailType::StateBlock, Direction::In);
                    let result = channel.read(&mut buf, StateBlock::serialized_size()).await;
                    buffer.lock().unwrap().copy_from_slice(&buf);
                    let ec;
                    let len;
                    match result {
                        Ok(()) => {
                            ec = ErrorCode::new();
                            len = SendBlock::serialized_size();
                        }
                        Err(_) => {
                            ec = ErrorCode::fault();
                            len = 0;
                        }
                    }
                    spawn_blocking(Box::new(move || {
                        server_impl.lock().unwrap().received_block(
                            server_impl2,
                            ec,
                            len,
                            BlockType::State,
                        );
                    }));
                }
                Some(BlockType::NotABlock) | Some(BlockType::Invalid) | None => unreachable!(),
            }
        });
    }

    fn received_block(
        &self,
        server_impl: Arc<Mutex<Self>>,
        ec: ErrorCode,
        _len: usize,
        block_type: BlockType,
    ) {
        let Some(block_processor) = self.block_processor.upgrade() else {
            return;
        };

        if ec.is_ok() {
            let guard = self.receive_buffer.lock().unwrap();
            let block =
                BlockEnum::deserialize_block_type(block_type, &mut BufferReader::new(&guard));
            drop(guard);
            match block {
                Ok(block) => {
                    if self.work_thresholds.validate_entry_block(&block) {
                        debug!("Insufficient work for bulk push block: {}", block.hash());
                        self.stats.inc_dir(
                            StatType::Error,
                            DetailType::InsufficientWork,
                            Direction::In,
                        );
                    } else {
                        block_processor.process_active(Arc::new(block));
                        self.throttled_receive(server_impl);
                    }
                }
                Err(_) => {
                    debug!("Error deserializing block received from pull request");
                }
            }
        }
    }
}
