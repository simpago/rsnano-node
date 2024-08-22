use crate::wallets::Wallets;
use rsnano_core::{utils::PropertyTree, Account};
use serde::Deserialize;
use serde_json::Value;
use std::{collections::HashSet, sync::Arc};
use tracing::warn;

#[derive(Clone)]
pub struct ConfirmationOptions {
    pub include_election_info: bool,
    pub include_election_info_with_votes: bool,
    pub include_sideband_info: bool,
    pub include_block: bool,
    pub has_account_filtering_options: bool,
    pub all_local_accounts: bool,
    pub confirmation_types: u8,
    pub accounts: HashSet<String>,
    wallets: Arc<Wallets>,
}

#[derive(Deserialize, Default)]
pub struct ConfirmationJsonOptions {
    pub include_block: Option<bool>,
    pub include_election_info: Option<bool>,
    pub include_election_info_with_votes: Option<bool>,
    pub include_sideband_info: Option<bool>,
    pub confirmation_type: Option<String>,
    pub all_local_accounts: Option<bool>,
    pub accounts: Option<Vec<String>>,
}

impl ConfirmationOptions {
    const TYPE_ACTIVE_QUORUM: u8 = 1;
    const TYPE_ACTIVE_CONFIRMATION_HEIGHT: u8 = 2;
    const TYPE_INACTIVE: u8 = 4;
    const TYPE_ALL_ACTIVE: u8 = Self::TYPE_ACTIVE_QUORUM | Self::TYPE_ACTIVE_CONFIRMATION_HEIGHT;
    const TYPE_ALL: u8 = Self::TYPE_ALL_ACTIVE | Self::TYPE_INACTIVE;

    pub fn new(wallets: Arc<Wallets>, options_a: ConfirmationJsonOptions) -> Self {
        let mut result = Self {
            include_election_info: false,
            include_election_info_with_votes: false,
            include_sideband_info: false,
            include_block: true,
            has_account_filtering_options: false,
            all_local_accounts: false,
            confirmation_types: Self::TYPE_ALL,
            accounts: HashSet::new(),
            wallets,
        };
        // Non-account filtering options
        result.include_block = options_a.include_block.unwrap_or(true);
        result.include_election_info = options_a.include_election_info.unwrap_or(false);
        result.include_election_info_with_votes =
            options_a.include_election_info_with_votes.unwrap_or(false);
        result.include_sideband_info = options_a.include_sideband_info.unwrap_or(false);

        let type_l = options_a
            .confirmation_type
            .unwrap_or_else(|| "all".to_string());

        if type_l.eq_ignore_ascii_case("active") {
            result.confirmation_types = Self::TYPE_ALL_ACTIVE;
        } else if type_l.eq_ignore_ascii_case("active_quorum") {
            result.confirmation_types = Self::TYPE_ACTIVE_QUORUM;
        } else if type_l.eq_ignore_ascii_case("active_confirmation_height") {
            result.confirmation_types = Self::TYPE_ACTIVE_CONFIRMATION_HEIGHT;
        } else if type_l.eq_ignore_ascii_case("inactive") {
            result.confirmation_types = Self::TYPE_INACTIVE;
        } else {
            result.confirmation_types = Self::TYPE_ALL;
        }

        // Account filtering options
        let all_local_accounts_l = options_a.all_local_accounts.unwrap_or(false);
        if all_local_accounts_l {
            result.all_local_accounts = true;
            result.has_account_filtering_options = true;
            if !result.include_block {
                warn!("Websocket: Filtering option \"all_local_accounts\" requires that \"include_block\" is set to true to be effective");
            }
        }
        if let Some(accounts_l) = options_a.accounts {
            result.has_account_filtering_options = true;
            for account_l in accounts_l {
                match Account::decode_account(&account_l) {
                    Ok(result_l) => {
                        // Do not insert the given raw data to keep old prefix support
                        result.accounts.insert(result_l.encode_account());
                    }
                    Err(_) => {
                        warn!(
                            "Invalid account provided for filtering blocks: {}",
                            account_l
                        );
                    }
                }
            }

            if !result.include_block {
                warn!("Filtering option \"accounts\" requires that \"include_block\" is set to true to be effective");
            }
        }
        result.check_filter_empty();

        result
    }

    /**
     * Checks if a message should be filtered for given block confirmation options.
     * @param message_a the message to be checked
     * @return false if the message should be broadcasted, true if it should be filtered
     */
    pub fn should_filter(&self, message_content: &Value) -> bool {
        let mut should_filter_conf_type = true;

        if let Some(serde_json::Value::String(type_text)) = message_content.get("confirmation_type")
        {
            let confirmation_types = self.confirmation_types;
            if type_text == "active_quorum" && (confirmation_types & Self::TYPE_ACTIVE_QUORUM) > 0 {
                should_filter_conf_type = false;
            } else if type_text == "active_confirmation_height"
                && (confirmation_types & Self::TYPE_ACTIVE_CONFIRMATION_HEIGHT) > 0
            {
                should_filter_conf_type = false;
            } else if type_text == "inactive" && (confirmation_types & Self::TYPE_INACTIVE) > 0 {
                should_filter_conf_type = false;
            }
        }

        let mut should_filter_account = self.has_account_filtering_options;
        if let Some(serde_json::Value::Object(block)) = message_content.get("block") {
            if let Some(serde_json::Value::String(destination_text)) = block.get("link_as_account")
            {
                let source_text = match message_content.get("account") {
                    Some(serde_json::Value::String(s)) => s.as_str(),
                    _ => "",
                };
                if self.all_local_accounts {
                    let source = Account::decode_account(source_text).unwrap_or_default();
                    let destination =
                        Account::decode_account(&destination_text).unwrap_or_default();
                    if self.wallets.exists(&source.into())
                        || self.wallets.exists(&destination.into())
                    {
                        should_filter_account = false;
                    }
                }
                if self.accounts.contains(source_text) || self.accounts.contains(destination_text) {
                    should_filter_account = false;
                }
            }
        }

        should_filter_conf_type || should_filter_account
    }

    /**
     * Update some existing options
     * Filtering options:
     * - "accounts_add" (array of std::strings) - additional accounts for which blocks should not be filtered
     * - "accounts_del" (array of std::strings) - accounts for which blocks should be filtered
     */
    pub fn update(&mut self, options: &dyn PropertyTree) {
        let mut update_accounts = |accounts_text: &dyn PropertyTree, insert: bool| {
            self.has_account_filtering_options = true;
            for account in accounts_text.get_children() {
                match Account::decode_account(account.1.data()) {
                    Ok(result) => {
                        // Re-encode to keep old prefix support
                        let encoded = result.encode_account();
                        if insert {
                            self.accounts.insert(encoded);
                        } else {
                            self.accounts.remove(&encoded);
                        }
                    }
                    Err(_) => {
                        warn!(
                            "Invalid account provided for filtering blocks: {}",
                            account.1.data()
                        );
                    }
                }
            }
        };

        // Adding accounts as filter exceptions
        if let Some(accounts_add) = options.get_child("accounts_add") {
            update_accounts(&*accounts_add, true);
        }

        // Removing accounts as filter exceptions
        if let Some(accounts_del) = options.get_child("accounts_del") {
            update_accounts(&*accounts_del, false);
        }

        self.check_filter_empty();
    }

    pub fn check_filter_empty(&self) {
        // Warn the user if the options resulted in an empty filter
        if self.has_account_filtering_options
            && !self.all_local_accounts
            && self.accounts.is_empty()
        {
            warn!("Provided options resulted in an empty account confirmation filter");
        }
    }
}
