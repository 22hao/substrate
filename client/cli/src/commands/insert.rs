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

//! Implementation of the `insert` subcommand

use crate::{error, HashFor, VersionInfo, with_crypto_scheme, pair_from_suri};
use super::{SharedParams, get_password, read_uri, RuntimeAdapter};
use structopt::StructOpt;
use sp_core::{crypto::KeyTypeId, Bytes};
use std::convert::TryFrom;
use futures01::Future;
use hyper::rt;
use sc_rpc::author::AuthorClient;
use jsonrpc_core_client::transports::http;
use serde::{de::DeserializeOwned, Serialize};
use sc_service::{Configuration, ChainSpec};

/// The `insert` command
#[derive(Debug, StructOpt, Clone)]
#[structopt(
	name = "insert",
	about = "Insert a key to the keystore of a node."
)]
pub struct InsertCmd {
	/// The secret key URI.
	/// If the value is a file, the file content is used as URI.
	/// If not given, you will be prompted for the URI.
	#[structopt(long)]
	suri: Option<String>,

	/// Key type, examples: "gran", or "imon"
	#[structopt(long)]
	key_type: String,

	/// Node JSON-RPC endpoint, default "http://localhost:9933"
	#[structopt(long)]
	node_url: Option<String>,

	#[allow(missing_docs)]
	#[structopt(flatten)]
	pub shared_params: SharedParams,
}

impl InsertCmd {
	/// Run the command
	pub fn run<RA: RuntimeAdapter>(self) -> error::Result<()>
		where
			HashFor<RA>: DeserializeOwned + Serialize + Send + Sync,
	{
		let suri = read_uri(self.suri)?;
		let password = get_password(&self.shared_params)?;

		let public = with_crypto_scheme!(
			self.shared_params.scheme,
			to_vec(&suri, &password)
		);

		let node_url = self.node_url.unwrap_or("http://localhost:9933".into());
		let key_type = self.key_type;

		// Just checking
		let _key_type_id = KeyTypeId::try_from(key_type.as_str())
			.map_err(|_| {
				error::Error::Other("Cannot convert argument to keytype: argument should be 4-character string".into())
			})?;


		insert_key::<HashFor<RA>>(
			&node_url,
			key_type.to_string(),
			suri,
			sp_core::Bytes(public),
		);

		Ok(())
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

fn to_vec<P: sp_core::Pair>(uri: &str, pass: &str) -> Vec<u8> {
	let p = pair_from_suri::<P>(uri, pass);
	p.public().as_ref().to_vec()
}

fn insert_key<H>(url: &str, key_type: String, suri: String, public: Bytes)
	where
		H: DeserializeOwned + Serialize + Send + Sync + 'static,
{
	rt::run(
		http::connect(&url)
			.and_then(|client: AuthorClient<H, H>| {
				client.insert_key(key_type, suri, public).map(|_| ())
			})
			.map_err(|e| {
				println!("Error inserting key: {:?}", e);
			})
	);
}
