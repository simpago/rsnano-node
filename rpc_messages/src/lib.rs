mod common;
mod ledger;
mod node;
mod utils;
mod wallets;

pub use common::*;
pub use ledger::*;
pub use node::*;
pub use utils::*;
pub use wallets::*;

use serde::{Deserialize, Serialize};

#[derive(PartialEq, Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum RpcCommand {
    AccountInfo(AccountInfoArgs),
    Keepalive(HostWithPortArgs),
    Stop,
    KeyCreate,
    Receive(ReceiveArgs),
    Send(SendArgs),
    WalletAdd(WalletAddArgs),
    WorkPeers,
    WorkPeerAdd(AddressWithPortArgs),
    Telemetry(TelemetryArgs),
    AccountCreate(AccountCreateArgs),
    AccountBalance(AccountBalanceArgs),
    AccountsCreate(AccountsCreateArgs),
    AccountRemove(WalletWithAccountArgs),
    AccountMove(AccountMoveArgs),
    AccountList(WalletRpcMessage),
    AccountRepresentativeSet(()), // TODO
    ActiveDifficulty,
    WalletCreate(WalletCreateArgs),
    WalletContains(WalletWithAccountArgs),
    WalletDestroy(WalletRpcMessage),
    WalletLock(WalletRpcMessage),
    WalletLocked(WalletRpcMessage),
    AccountBlockCount(AccountArg),
    AccountKey(AccountArg),
    AccountGet(KeyArg),
    AccountRepresentative(AccountArg),
    AccountWeight(AccountWeightArgs),
    AvailableSupply,
    BlockAccount(HashRpcMessage),
    BlockConfirm(HashRpcMessage),
    DatabaseTxnTracker(()), // TODO
    ConfirmationHistory(ConfirmationHistoryArgs),
    BlockCount,
    Uptime,
    FrontierCount,
    ValidateAccountNumber(AccountCandidateArg),
    NanoToRaw(AmountRpcMessage),
    RawToNano(AmountRpcMessage),
    WalletAddWatch(WalletAddWatchArgs),
    WalletRepresentative(WalletRpcMessage),
    WorkSet(WorkSetArgs),
    WorkGet(WalletWithAccountArgs),
    WalletWorkGet(WalletRpcMessage),
    AccountsFrontiers(AccountsRpcMessage),
    WalletFrontiers(WalletRpcMessage),
    Frontiers(FrontiersArgs),
    WalletInfo(WalletRpcMessage),
    WalletExport(WalletRpcMessage),
    PasswordChange(WalletWithPasswordArgs),
    PasswordEnter(WalletWithPasswordArgs),
    PasswordValid(WalletRpcMessage),
    DeterministicKey(DeterministicKeyArgs),
    KeyExpand(KeyExpandArgs),
    Peers(PeersArgs),
    PopulateBacklog,
    Representatives(RepresentativesArgs),
    AccountsRepresentatives(AccountsRpcMessage),
    StatsClear,
    UncheckedClear,
    Unopened(UnopenedArgs),
    NodeId,
    SearchReceivableAll,
    ReceiveMinimum,
    ReceiveMinimumSet(()), // TODO
    Stats(StatsArgs),
    WalletChangeSeed(WalletChangeSeedArgs),
    Delegators(DelegatorsArgs),
    DelegatorsCount(AccountArg),
    BlockHash(BlockHashArgs),
    AccountsBalances(AccountsBalancesArgs),
    BlockInfo(HashRpcMessage),
    Blocks(HashesArgs),
    BlocksInfo(BlocksInfoArgs),
    Chain(ChainArgs),
    Successors(ChainArgs),
    ConfirmationActive(ConfirmationActiveArgs),
    ConfirmationQuorum(ConfirmationQuorumArgs),
    WorkValidate(WorkValidateArgs),
    AccountHistory(AccountHistoryArgs),
    Sign(SignArgs),
    Process(ProcessArgs),
    WorkCancel(HashRpcMessage),
    Bootstrap(BootstrapArgs),
    BootstrapAny(BootstrapAnyArgs),
    BootstrapLazy(BootstrapLazyArgs),
    WalletReceivable(WalletReceivableArgs),
    WalletRepresentativeSet(WalletRepresentativeSetArgs),
    SearchReceivable(WalletRpcMessage),
    WalletRepublish(WalletWithCountArgs),
    WalletBalances(WalletBalancesArgs),
    WalletHistory(WalletHistoryArgs),
    WalletLedger(WalletLedgerArgs),
    AccountsReceivable(AccountsReceivableArgs),
    Receivable(ReceivableArgs),
    ReceivableExists(ReceivableExistsArgs),
    RepresentativesOnline(RepresentativesOnlineArgs),
    Unchecked(CountArgs),
    UncheckedGet(HashRpcMessage),
    UncheckedKeys(UncheckedKeysArgs),
    ConfirmationInfo(ConfirmationInfoArgs),
    Ledger(LedgerArgs),
    WorkGenerate(WorkGenerateArgs),
    Republish(RepublishArgs),
    BlockCreate(BlockCreateArgs),
    WorkPeersClear,
    Version,
}

pub fn check_error(value: &serde_json::Value) -> Result<(), String> {
    if let Some(serde_json::Value::String(error)) = value.get("error") {
        Err(error.clone())
    } else {
        Ok(())
    }
}
