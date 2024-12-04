use crate::cli::CliInfrastructure;
use rsnano_core::Account;

pub(crate) fn create_key_pair(infra: &mut CliInfrastructure) {
    let keypair = infra.key_factory.create_key_pair();
    let private_key = keypair.private_key();
    let public_key = keypair.public_key();
    let account = Account::from(public_key).encode_account();

    infra.console.println(format!("Private: {}", private_key));
    infra.console.println(format!("Public: {}", public_key));
    infra.console.println(format!("Account: {}", account));
}

#[cfg(test)]
mod tests {
    use crate::cli::{Cli, CliInfrastructure};
    use clap::Parser;

    #[tokio::test]
    async fn create_key_pair() {
        let cli = Cli::try_parse_from(["nulled_node", "utils", "create-key-pair"]).unwrap();
        let mut infra = CliInfrastructure::new_null();
        let print_tracker = infra.console.track();

        cli.run(&mut infra).await.unwrap();

        let output = print_tracker.output();
        assert_eq!(
            output,
            [
                "Private: 000000000000002A000000000000002A000000000000002A000000000000002A",
                "Public: 49074D77DBE728CEB5EA2628A75DC7CE21493FDDCFCA991AAA1629F11D99FFD9",
                "Account: nano_1ka9bouxqssasttynbjanxgwhmj3b6zxumycm6fcn7jby6gsmzysauneamau",
            ]
        );
    }
}
