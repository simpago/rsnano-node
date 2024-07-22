use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use confirmation_height::ConfirmationHeightArgs;
use final_vote::FinalVoteArgs;
use online_weight::OnlineWeightArgs;
use peers::PeersArgs;

pub(crate) mod confirmation_height;
pub(crate) mod final_vote;
pub(crate) mod online_weight;
pub(crate) mod peers;

#[derive(Subcommand)]
pub(crate) enum ClearSubcommands {
    /// Either specify a single --root to clear or --all to clear all final votes (not recommended).
    FinalVote(FinalVoteArgs),
    /// Clear online weight history records.
    OnlineWeight(OnlineWeightArgs),
    /// Clear online peers database.
    Peers(PeersArgs),
    /// Clear confirmation height. Requires an <account> option that can be 'all' to clear all accounts.
    ConfirmationHeight(ConfirmationHeightArgs),
}

#[derive(Parser)]
pub(crate) struct ClearCommand {
    #[command(subcommand)]
    pub subcommand: Option<ClearSubcommands>,
}

impl ClearCommand {
    pub(crate) fn run(&self) -> Result<()> {
        match &self.subcommand {
            Some(ClearSubcommands::FinalVote(args)) => args.final_vote()?,
            Some(ClearSubcommands::ConfirmationHeight(args)) => args.confirmation_height()?,
            Some(ClearSubcommands::OnlineWeight(args)) => args.online_weight()?,
            Some(ClearSubcommands::Peers(args)) => args.peers()?,
            None => ClearCommand::command().print_long_help()?,
        }

        Ok(())
    }
}
