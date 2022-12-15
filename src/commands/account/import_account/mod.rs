use inquire::{CustomType, Select};
use std::{str::FromStr, vec};

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(context = crate::GlobalContext)]
pub struct Login {
    #[interactive_clap(named_arg)]
    ///Select network
    network_config: crate::network::Network,
}

impl Login {
    pub async fn process(&self, config: crate::config::Config) -> crate::CliResult {
        let network_config = self.network_config.get_network_config(config.clone());
        login(network_config, config.credentials_home_dir).await
    }
}

async fn login(
    network_config: crate::config::NetworkConfig,
    credentials_home_dir: std::path::PathBuf,
) -> crate::CliResult {
    let key_pair_properties: crate::common::KeyPairProperties =
        crate::common::generate_keypair().await?;
    let mut url: url::Url = network_config.wallet_url.join("login/")?;
    url.query_pairs_mut()
        .append_pair("title", "NEAR CLI")
        .append_pair("public_key", &key_pair_properties.public_key_str);
    // Use `success_url` once capture mode is implemented
    //.append_pair("success_url", "http://127.0.0.1:8080");
    println!(
        "If your browser doesn't automatically open, please visit this URL:\n {}\n",
        &url.as_str()
    );
    // url.open();
    open::that(url.as_ref()).ok();

    let public_key: near_crypto::PublicKey =
        near_crypto::PublicKey::from_str(&key_pair_properties.public_key_str)?;

    let account_id = loop {
        let account_id_from_cli = input_account_id()?;
        println!();
        if crate::common::verify_account_access_key(
            account_id_from_cli.clone(),
            public_key.clone(),
            network_config.clone(),
        )
        .await
        .is_err()
        {
            println!("\nIt is currently not possible to verify the account access key.\nYou may not be logged in to {} or you may have entered an incorrect account_id.\nYou have the option to reconfirm your account or save your access key information.\n", &url.as_str());
            #[derive(strum_macros::Display)]
            enum ConfirmOptions {
                #[strum(to_string = "Yes, I want to re-enter the account_id.")]
                Yes,
                #[strum(to_string = "No, I want to save the access key information.")]
                No,
            }
            let select_choose_input = Select::new(
                "Would you like to re-enter the account_id?",
                vec![ConfirmOptions::Yes, ConfirmOptions::No],
            )
            .prompt()?;
            if let ConfirmOptions::No = select_choose_input {
                break account_id_from_cli;
            }
        } else {
            break account_id_from_cli;
        }
    };
    save_access_key(
        account_id,
        key_pair_properties,
        network_config,
        credentials_home_dir,
    )?;

    Ok(())
}

fn input_account_id() -> color_eyre::eyre::Result<near_primitives::types::AccountId> {
    Ok(CustomType::new("Enter account ID").prompt()?)
}

fn save_access_key(
    account_id: near_primitives::types::AccountId,
    key_pair_properties: crate::common::KeyPairProperties,
    network_config: crate::config::NetworkConfig,
    credentials_home_dir: std::path::PathBuf,
) -> crate::CliResult {
    #[cfg(target_os = "macos")]
    {
        let macos_keychain = "Store the access key in my macOS keychain";
        let legacy_keychain =
            "Store the access key in my legacy keychain (compatible with the old near CLI)";
        let selection = Select::new(
            "Select a keychain to save the access key to:",
            vec![macos_keychain, legacy_keychain],
        )
        .prompt()?;
        if selection == macos_keychain {
            let storage_message = crate::common::save_access_key_to_macos_keychain(
                network_config,
                key_pair_properties,
                &account_id,
            )
            .map_err(|err| {
                color_eyre::Report::msg(format!(
                    "Failed to save the access key to the keychain: {}",
                    err
                ))
            })?;
            println!("{}", storage_message);
            return Ok(());
        }
    }
    let storage_message = crate::common::save_access_key_to_keychain(
        network_config,
        credentials_home_dir,
        key_pair_properties,
        &account_id,
    )
    .map_err(|err| {
        color_eyre::Report::msg(format!("Failed to save a file with access key: {}", err))
    })?;
    println!("{}", storage_message);
    Ok(())
}
