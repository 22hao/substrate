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

//! Implementation of the `sign-transaction` subcommand
use crate::{error, VersionInfo, with_crypto_scheme};
use super::{
	SharedParams, get_password,
	IndexFor, CallFor, pair_from_suri, decode_hex, create_extrinsic_for,
};
use structopt::StructOpt;
use parity_scale_codec::{Codec, Encode, Decode};
use std::{str::FromStr, fmt::Display};
use sc_service::{Configuration, ChainSpec};
use sp_runtime::MultiSigner;
use cli_utils::RuntimeAdapter;

type Call = Vec<u8>;

/// The `sign-transaction` command
#[derive(Debug, StructOpt, Clone)]
#[structopt(
	name = "sign-transaction",
	about = "Sign transaction from encoded Call. Returns a signed and encoded UncheckedMortalCompactExtrinsic as hex."
)]
pub struct SignTransactionCmd {
	/// The secret key URI.
	#[structopt(long)]
	suri: String,

	/// The nonce.
	#[structopt(long)]
	nonce: String,

	/// The call, hex-encoded.
	#[structopt(long, parse(try_from_str = decode_hex))]
	call: Call,

	#[allow(missing_docs)]
	#[structopt(flatten)]
	pub shared_params: SharedParams,
}

impl SignTransactionCmd {
	/// Run the command
	pub fn run<RA>(self) -> error::Result<()>
		where
			RA: RuntimeAdapter,
			<IndexFor<RA> as FromStr>::Err: Display,
			CallFor<RA>: Codec,
	{

		let nonce = IndexFor::<RA>::from_str(&self.nonce).map_err(|e| format!("{}", e))?;
		let call = CallFor::<RA>::decode(&mut &self.call[..])?;

		with_crypto_scheme!(
			self.shared_params.scheme,
			print_ext<RA>(
				&self.suri,
				&get_password(&self.shared_params)?,
				call,
				nonce
			)
		)
	}

	/// Update and prepare a `Configuration` with command line parameters
	pub fn update_config<F>(
		&self,
		mut config: &mut Configuration,
		spec_factory: F,
		version: &VersionInfo,
	) -> error::Result<()> where
		F: FnOnce(&str) -> Result<Box<dyn ChainSpec>, String>,
	{
		self.shared_params.update_config(&mut config, spec_factory, version)?;

		Ok(())
	}
}

fn print_ext<Pair, RA>(uri: &str, pass: &str, call: CallFor<RA>, nonce: IndexFor<RA>) -> error::Result<()>
	where
		Pair: sp_core::Pair,
		Pair::Public: Into<MultiSigner>,
		Pair::Signature: Encode,
		RA: RuntimeAdapter,
		CallFor<RA>: Codec,
{
	let signer = pair_from_suri::<Pair>(uri, pass);
	let extrinsic = create_extrinsic_for::<Pair, RA, CallFor<RA>>(call, nonce, signer)?;
	println!("0x{}", hex::encode(Encode::encode(&extrinsic)));
	Ok(())
}
