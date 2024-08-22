use crate::{ProgramArgs, TestNode};
use anyhow::{bail, Result};
use rsnano_core::Account;
use rsnano_rpc_messages::AccountInfoDto;
use std::{collections::HashMap, time::Duration};
use tokio::time::{sleep, Instant};

pub struct LoadTest {
    args: ProgramArgs,
    nodes: Vec<TestNode>,
}

impl LoadTest {
    pub fn new(args: ProgramArgs) -> Self {
        let nodes = Vec::with_capacity(args.node_count);
        Self { args, nodes }
    }

    pub async fn run(mut self) -> Result<()> {
        self.start_nodes().await?;
        self.wait_for_nodes_to_spin_up().await;
        self.connect_nodes().await?;
        let expected_account_info = self.create_send_and_receive_blocks().await?;
        self.wait_for_nodes_to_catch_up(&expected_account_info)
            .await?;
        self.stop_nodes().await?;
        Ok(())
    }

    fn primary_node(&self) -> &TestNode {
        &self.nodes[0]
    }

    async fn start_nodes(&mut self) -> Result<()> {
        for i in 0..self.args.node_count {
            let mut node = TestNode::new(i)?;
            println!(
                "starting node, port {}, data dir {:?}",
                node.rpc_port, node.data_path
            );
            node.start(&self.args.node_path, &self.args.rpc_path)
                .await?;
            self.nodes.push(node);
        }
        Ok(())
    }

    async fn wait_for_nodes_to_spin_up(&self) {
        println!("Waiting for nodes to spin up...");
        sleep(Duration::from_secs(7)).await;
    }

    async fn connect_nodes(&self) -> Result<()> {
        println!("Connecting nodes...");
        for node in &self.nodes {
            self.primary_node().connect(node).await?;
        }
        Ok(())
    }

    async fn create_send_and_receive_blocks(&self) -> Result<HashMap<Account, AccountInfoDto>> {
        println!("Beginning tests");
        self.primary_node()
            .create_send_and_receive_blocks(
                self.args.destination_count,
                self.args.send_count,
                self.args.simultaneous_process_calls,
            )
            .await
    }

    async fn wait_for_nodes_to_catch_up(
        &self,
        expected_account_info: &HashMap<Account, AccountInfoDto>,
    ) -> Result<()> {
        println!("Waiting for nodes to catch up...");
        let timer = Instant::now();
        for node in &self.nodes[1..] {
            for (account, info) in expected_account_info {
                loop {
                    if let Ok(other_account_info) = node.account_info(*account).await {
                        if info == &other_account_info {
                            // Found the account in this node
                            break;
                        }
                    }
                    if timer.elapsed() > Duration::from_secs(120) {
                        bail!("Timed out");
                    }

                    sleep(Duration::from_secs(1)).await;
                }
            }
        }
        println!("Done!");
        Ok(())
    }

    async fn stop_nodes(mut self) -> Result<()> {
        for node in self.nodes.iter_mut() {
            node.stop().await?;
        }
        Ok(())
    }
}
