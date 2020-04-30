// Copyright 2018-2020 Parity Technologies (UK) Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

//! subcommand utilities
use std::{io::Read, path::PathBuf};
use sp_core::{
	Pair, hexdisplay::HexDisplay,
	crypto::{Ss58Codec, Ss58AddressFormat, AccountId32},
};
use sp_runtime::{
	traits::IdentifyAccount, MultiSigner,
	generic::{UncheckedExtrinsic, SignedPayload},
};
use crate::{arg_enums::{OutputType}, error::{self, Error}, KeystoreParams};
use parity_scale_codec::Encode;
use serde_json::json;
use cli_utils::IndexFor;

/// Public key type for Runtime
pub type PublicFor<P> = <P as sp_core::Pair>::Public;
/// Seed type for Runtime
pub type SeedFor<P> = <P as sp_core::Pair>::Seed;

/// helper method to fetch password from `SharedParams` or read from stdin
pub fn get_password(params: &KeystoreParams) -> error::Result<String> {
	let (password_interactive, password) = (params.password_interactive, params.password.as_ref());

	let pass = if password_interactive {
		rpassword::read_password_from_tty(Some("Key password: "))?
	} else {
		password.map(Into::into).ok_or("Password not specified")?
	};

	Ok(pass)
}

/// helper method to fetch uri from `Option<String>` either as a file or read from stdin
pub fn read_uri(uri: Option<String>) -> error::Result<String> {
	let uri = if let Some(uri) = uri {
		let file = PathBuf::from(uri.clone());
		if file.is_file() {
			std::fs::read_to_string(uri)?
				.trim_end()
				.to_owned()
		} else {
			uri.into()
		}
	} else {
		rpassword::read_password_from_tty(Some("URI: "))?
	};

	Ok(uri)
}

/// Allows for calling $method with appropriate crypto impl.
#[macro_export]
macro_rules! with_crypto_scheme {
	($scheme:expr, $method:ident($($params:expr),*)) => {
		with_crypto_scheme!($scheme, $method<>($($params),*))
	};
    ($scheme:expr, $method:ident<$($gen:ident),*>($($params:expr),*)) => {
        match $scheme {
			$crate::arg_enums::CryptoScheme::Ecdsa => {
				$method::<sp_core::ecdsa::Pair, $($gen),*>($($params),*)
			}
			$crate::arg_enums::CryptoScheme::Sr25519 => {
				$method::<sp_core::sr25519::Pair, $($gen),*>($($params),*)
			}
			$crate::arg_enums::CryptoScheme::Ed25519 => {
				$method::<sp_core::ed25519::Pair, $($gen),*>($($params),*)
			}
		}
    };
}

/// print formatted pair from uri
pub fn print_from_uri<Pair>(
	uri: &str,
	password: Option<&str>,
	network_override: Ss58AddressFormat,
	output: OutputType,
)
	where
		Pair: sp_core::Pair,
		Pair::Public: Into<MultiSigner>,
{
	if let Ok((pair, seed)) = Pair::from_phrase(uri, password) {
		let public_key = pair.public();

		match output {
			OutputType::Json => {
				let json = json!({
						"secretPhrase": uri,
						"secretSeed": format_seed::<Pair>(seed),
						"publicKey": format_public_key::<Pair>(public_key.clone()),
						"accountId": format_account_id::<Pair>(public_key),
						"ss58Address": pair.public().into().into_account().to_ss58check(),
					});
				println!("{}", serde_json::to_string_pretty(&json).expect("Json pretty print failed"));
			},
			OutputType::Text => {
				println!("Secret phrase `{}` is account:\n  \
						Secret seed:      {}\n  \
						Public key (hex): {}\n  \
						Account ID:       {}\n  \
						SS58 Address:     {}",
				         uri,
				         format_seed::<Pair>(seed),
				         format_public_key::<Pair>(public_key.clone()),
				         format_account_id::<Pair>(public_key),
				         pair.public().into().into_account().to_ss58check(),
				);
			},
		}
	} else if let Ok((pair, seed)) = Pair::from_string_with_seed(uri, password) {
		let public_key = pair.public();

		match output {
			OutputType::Json => {
				let json = json!({
						"secretKeyUri": uri,
						"secretSeed": if let Some(seed) = seed { format_seed::<Pair>(seed) } else { "n/a".into() },
						"publicKey": format_public_key::<Pair>(public_key.clone()),
						"accountId": format_account_id::<Pair>(public_key),
						"ss58Address": pair.public().into().into_account().to_ss58check(),
					});
				println!("{}", serde_json::to_string_pretty(&json).expect("Json pretty print failed"));
			},
			OutputType::Text => {
				println!("Secret Key URI `{}` is account:\n  \
						Secret seed:      {}\n  \
						Public key (hex): {}\n  \
						Account ID:       {}\n  \
						SS58 Address:     {}",
				         uri,
				         if let Some(seed) = seed { format_seed::<Pair>(seed) } else { "n/a".into() },
				         format_public_key::<Pair>(public_key.clone()),
				         format_account_id::<Pair>(public_key),
				         pair.public().into().into_account().to_ss58check(),
				);
			},
		}
	} else if let Ok((public_key, _v)) = Pair::Public::from_string_with_version(uri) {
		let v = network_override;

		match output {
			OutputType::Json => {
				let json = json!({
						"publicKeyUri": uri,
						"networkId": String::from(v),
						"publicKey": format_public_key::<Pair>(public_key.clone()),
						"accountId": format_account_id::<Pair>(public_key.clone()),
						"ss58Address": public_key.to_ss58check_with_version(v),
					});
				println!("{}", serde_json::to_string_pretty(&json).expect("Json pretty print failed"));
			},
			OutputType::Text => {
				println!("Public Key URI `{}` is account:\n  \
						Network ID/version: {}\n  \
						Public key (hex):   {}\n  \
						Account ID:         {}\n  \
						SS58 Address:       {}",
				         uri,
				         String::from(v),
				         format_public_key::<Pair>(public_key.clone()),
				         format_account_id::<Pair>(public_key.clone()),
				         public_key.to_ss58check_with_version(v),
				);
			},
		}
	} else {
		println!("Invalid phrase/URI given");
	}
}

/// generate a pair from suri
pub fn pair_from_suri<P: Pair>(suri: &str, password: &str) -> P {
	P::from_string(suri, Some(password)).expect("Invalid phrase")
}

/// formats seed as hex
pub fn format_seed<P: sp_core::Pair>(seed: SeedFor<P>) -> String {
	format!("0x{}", HexDisplay::from(&seed.as_ref()))
}

/// formats public key as hex
fn format_public_key<P: sp_core::Pair>(public_key: PublicFor<P>) -> String {
	format!("0x{}", HexDisplay::from(&public_key.as_ref()))
}

/// formats public key as accountId as hex
fn format_account_id<P: sp_core::Pair>(public_key: PublicFor<P>) -> String
	where
		PublicFor<P>: Into<MultiSigner>,
{
	format!("0x{}", HexDisplay::from(&public_key.into().into_account().as_ref()))
}

/// helper method for decoding hex
pub fn decode_hex<T: AsRef<[u8]>>(message: T) -> Result<Vec<u8>, Error> {
	let mut message = message.as_ref();
	if message[..2] == [b'0', b'x'] {
		message = &message[2..]
	}
	hex::decode(message)
		.map_err(|e| Error::Other(format!("Invalid hex ({})", e)))
}

/// checks if message is Some, otherwise reads message from stdin and optionally decodes hex
pub fn read_message(msg: Option<String>, should_decode: bool) -> Result<Vec<u8>, Error> {
	let mut message = vec![];
	match msg {
		Some(m) => {
			message = decode_hex(&m)?;
		},
		None => {
			std::io::stdin().lock().read_to_end(&mut message)?;
			if should_decode {
				message = decode_hex(&message)?;
			}
		}
	}
	Ok(message)
}

/// create an extrinsic for the runtime.
pub fn create_extrinsic_for<Pair, RA, Call>(
	call: Call,
	nonce:  IndexFor<RA>,
	signer: Pair,
) -> Result<UncheckedExtrinsic<AccountId32, Call, Pair::Signature, RA::Extra>, Error>
	where
		Call: Encode,
		Pair: sp_core::Pair,
		Pair::Public: Into<MultiSigner>,
		Pair::Signature: Encode,
		RA: RuntimeAdapter,
{
	let extra = RA::build_extra(nonce);
	let payload = SignedPayload::new(call, extra)
		.map_err(|_| Error::Other("Transaction validity error".into()))?;

	let signature = payload.using_encoded(|payload| signer.sign(payload));
	let signer = signer.public().into().into_account();
	let (function, extra, _) = payload.deconstruct();

	Ok(UncheckedExtrinsic::new_signed(
		function,
		signer,
		signature,
		extra,
	))
}
