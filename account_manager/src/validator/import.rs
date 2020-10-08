use crate::wallet::create::STDIN_INPUTS_FLAG;
use account_utils::{
    eth2_keystore::Keystore,
    read_password_from_user,
    validator_definitions::{
        recursively_find_voting_keystores, ValidatorDefinition, ValidatorDefinitions,
        CONFIG_FILENAME,
    },
    ZeroizeString,
};
use clap::{App, Arg, ArgMatches};
use slashing_protection::{SlashingDatabase, SLASHING_PROTECTION_FILENAME};
use std::fs;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

pub const CMD: &str = "import";
pub const KEYSTORE_FLAG: &str = "keystore";
pub const DIR_FLAG: &str = "directory";
pub const REUSE_PASSWORD_FLAG: &str = "reuse-password";

pub const PASSWORD_PROMPT: &str = "Enter the keystore password, or press enter to omit it:";
pub const KEYSTORE_REUSE_WARNING: &str = "DO NOT USE THE ORIGINAL KEYSTORES TO VALIDATE WITH \
                                          ANOTHER CLIENT, OR YOU WILL GET SLASHED.";

pub fn cli_app<'a, 'b>() -> App<'a, 'b> {
    App::new(CMD)
        .about(
            "Imports one or more EIP-2335 passwords into a Lighthouse VC directory, \
            requesting passwords interactively. The directory flag provides a convenient \
            method for importing a directory of keys generated by the eth2-deposit-cli \
            Python utility.",
        )
        .arg(
            Arg::with_name(KEYSTORE_FLAG)
                .long(KEYSTORE_FLAG)
                .value_name("KEYSTORE_PATH")
                .help("Path to a single keystore to be imported.")
                .conflicts_with(DIR_FLAG)
                .required_unless(DIR_FLAG)
                .takes_value(true),
        )
        .arg(
            Arg::with_name(DIR_FLAG)
                .long(DIR_FLAG)
                .value_name("KEYSTORES_DIRECTORY")
                .help(
                    "Path to a directory which contains zero or more keystores \
                    for import. This directory and all sub-directories will be \
                    searched and any file name which contains 'keystore' and \
                    has the '.json' extension will be attempted to be imported.",
                )
                .conflicts_with(KEYSTORE_FLAG)
                .required_unless(KEYSTORE_FLAG)
                .takes_value(true),
        )
        .arg(
            Arg::with_name(STDIN_INPUTS_FLAG)
                .long(STDIN_INPUTS_FLAG)
                .help("If present, read all user inputs from stdin instead of tty."),
        )
        .arg(
            Arg::with_name(REUSE_PASSWORD_FLAG)
                .long(REUSE_PASSWORD_FLAG)
                .help("If present, the same password will be used for all imported keystores."),
        )
}

pub fn cli_run(matches: &ArgMatches, validator_dir: PathBuf) -> Result<(), String> {
    let keystore: Option<PathBuf> = clap_utils::parse_optional(matches, KEYSTORE_FLAG)?;
    let keystores_dir: Option<PathBuf> = clap_utils::parse_optional(matches, DIR_FLAG)?;
    let stdin_inputs = matches.is_present(STDIN_INPUTS_FLAG);
    let reuse_password = matches.is_present(REUSE_PASSWORD_FLAG);

    if !validator_dir.exists() {
        fs::create_dir_all(&validator_dir)
            .map_err(|e| format!("Unable to create {}: {:?}", validator_dir.display(), e))?;
    }

    let mut defs = ValidatorDefinitions::open_or_create(&validator_dir)
        .map_err(|e| format!("Unable to open {}: {:?}", CONFIG_FILENAME, e))?;

    let slashing_protection_path = validator_dir.join(SLASHING_PROTECTION_FILENAME);
    let slashing_protection =
        SlashingDatabase::open_or_create(&slashing_protection_path).map_err(|e| {
            format!(
                "Unable to open or create slashing protection database at {}: {:?}",
                slashing_protection_path.display(),
                e
            )
        })?;

    // Collect the paths for the keystores that should be imported.
    let keystore_paths = match (keystore, keystores_dir) {
        (Some(keystore), None) => vec![keystore],
        (None, Some(keystores_dir)) => {
            let mut keystores = vec![];

            recursively_find_voting_keystores(&keystores_dir, &mut keystores)
                .map_err(|e| format!("Unable to search {:?}: {:?}", keystores_dir, e))?;

            if keystores.is_empty() {
                eprintln!("No keystores found in {:?}", keystores_dir);
                return Ok(());
            }

            keystores
        }
        _ => {
            return Err(format!(
                "Must supply either --{} or --{}",
                KEYSTORE_FLAG, DIR_FLAG
            ))
        }
    };

    eprintln!("WARNING: {}", KEYSTORE_REUSE_WARNING);

    // For each keystore:
    //
    // - Obtain the keystore password, if the user desires.
    // - Copy the keystore into the `validator_dir`.
    // - Register the voting key with the slashing protection database.
    // - Add the keystore to the validator definitions file.
    //
    // Skip keystores that already exist, but exit early if any operation fails.
    // Reuses the same password for all keystores if the `REUSE_PASSWORD_FLAG` flag is set.
    let mut num_imported_keystores = 0;
    let mut previous_password: Option<ZeroizeString> = None;
    for src_keystore in &keystore_paths {
        let keystore = Keystore::from_json_file(src_keystore)
            .map_err(|e| format!("Unable to read keystore JSON {:?}: {:?}", src_keystore, e))?;

        eprintln!("");
        eprintln!("Keystore found at {:?}:", src_keystore);
        eprintln!("");
        eprintln!(" - Public key: 0x{}", keystore.pubkey());
        eprintln!(" - UUID: {}", keystore.uuid());
        eprintln!("");
        eprintln!(
            "If you enter the password it will be stored as plain-text in {} so that it is not \
             required each time the validator client starts.",
            CONFIG_FILENAME
        );

        let password_opt = loop {
            if let Some(password) = previous_password.clone() {
                eprintln!("Reuse previous password.");
                break Some(password);
            }
            eprintln!("");
            eprintln!("{}", PASSWORD_PROMPT);

            let password = read_password_from_user(stdin_inputs)?;

            if password.as_ref().is_empty() {
                eprintln!("Continuing without password.");
                sleep(Duration::from_secs(1)); // Provides nicer UX.
                break None;
            }

            match keystore.decrypt_keypair(password.as_ref()) {
                Ok(_) => {
                    eprintln!("Password is correct.");
                    eprintln!("");
                    sleep(Duration::from_secs(1)); // Provides nicer UX.
                    if reuse_password {
                        previous_password = Some(password.clone());
                    }
                    break Some(password);
                }
                Err(eth2_keystore::Error::InvalidPassword) => {
                    eprintln!("Invalid password");
                }
                Err(e) => return Err(format!("Error whilst decrypting keypair: {:?}", e)),
            }
        };

        // The keystore is placed in a directory that matches the name of the public key. This
        // provides some loose protection against adding the same keystore twice.
        let dest_dir = validator_dir.join(format!("0x{}", keystore.pubkey()));
        if dest_dir.exists() {
            eprintln!(
                "Skipping import of keystore for existing public key: {:?}",
                src_keystore
            );
            continue;
        }

        fs::create_dir_all(&dest_dir)
            .map_err(|e| format!("Unable to create import directory: {:?}", e))?;

        // Retain the keystore file name, but place it in the new directory.
        let dest_keystore = src_keystore
            .file_name()
            .and_then(|file_name| file_name.to_str())
            .map(|file_name_str| dest_dir.join(file_name_str))
            .ok_or_else(|| format!("Badly formatted file name: {:?}", src_keystore))?;

        // Copy the keystore to the new location.
        fs::copy(&src_keystore, &dest_keystore)
            .map_err(|e| format!("Unable to copy keystore: {:?}", e))?;

        // Register with slashing protection.
        let voting_pubkey = keystore
            .public_key()
            .ok_or_else(|| format!("Keystore public key is invalid: {}", keystore.pubkey()))?;
        slashing_protection
            .register_validator(&voting_pubkey)
            .map_err(|e| {
                format!(
                    "Error registering validator {}: {:?}",
                    voting_pubkey.to_hex_string(),
                    e
                )
            })?;

        eprintln!("Successfully imported keystore.");
        num_imported_keystores += 1;

        let validator_def =
            ValidatorDefinition::new_keystore_with_password(&dest_keystore, password_opt)
                .map_err(|e| format!("Unable to create new validator definition: {:?}", e))?;

        defs.push(validator_def);

        defs.save(&validator_dir)
            .map_err(|e| format!("Unable to save {}: {:?}", CONFIG_FILENAME, e))?;

        eprintln!("Successfully updated {}.", CONFIG_FILENAME);
    }

    eprintln!("");
    eprintln!(
        "Successfully imported {} validators ({} skipped).",
        num_imported_keystores,
        keystore_paths.len() - num_imported_keystores
    );
    eprintln!("");
    eprintln!("WARNING: {}", KEYSTORE_REUSE_WARNING);

    Ok(())
}
