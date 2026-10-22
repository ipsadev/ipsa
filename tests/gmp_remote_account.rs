use ipsa::gmp::{account_salt, remote_account};
use soroban_sdk::{testutils::Address as _, Address, Bytes, Env, String};

#[test]
fn the_account_salt_depends_on_the_client_the_sender_and_the_salt() {
    let env = Env::default();
    let client = String::from_str(&env, "07-tendermint-0");
    let sender = String::from_str(&env, "cosmos1abc");
    let salt = Bytes::from_slice(&env, &[1]);
    let base = account_salt(&env, &client, &sender, &salt);

    assert_eq!(account_salt(&env, &client, &sender, &salt), base);
    assert_ne!(
        account_salt(
            &env,
            &String::from_str(&env, "07-tendermint-1"),
            &sender,
            &salt
        ),
        base
    );
    assert_ne!(
        account_salt(&env, &client, &String::from_str(&env, "cosmos1abd"), &salt),
        base
    );
    assert_ne!(
        account_salt(&env, &client, &sender, &Bytes::from_slice(&env, &[2])),
        base
    );
}

#[test]
fn the_length_prefix_keeps_a_boundary_shift_from_colliding() {
    let env = Env::default();
    let first = account_salt(
        &env,
        &String::from_str(&env, "07-tendermint-0"),
        &String::from_str(&env, "ab"),
        &Bytes::from_slice(&env, b"c"),
    );
    let second = account_salt(
        &env,
        &String::from_str(&env, "07-tendermint-0"),
        &String::from_str(&env, "a"),
        &Bytes::from_slice(&env, b"bc"),
    );

    assert_ne!(first, second);
}

#[test]
fn the_remote_account_is_derived_from_the_gmp_app_address() {
    let env = Env::default();
    let client = String::from_str(&env, "07-tendermint-0");
    let sender = String::from_str(&env, "cosmos1abc");
    let salt = Bytes::new(&env);
    let gmp = Address::generate(&env);
    let other_gmp = Address::generate(&env);

    assert_eq!(
        remote_account(&env, &gmp, &client, &sender, &salt),
        remote_account(&env, &gmp, &client, &sender, &salt)
    );
    assert_ne!(
        remote_account(&env, &gmp, &client, &sender, &salt),
        remote_account(&env, &other_gmp, &client, &sender, &salt)
    );
}
