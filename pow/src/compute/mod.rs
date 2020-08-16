// Copyright 2019-2020 Wei Tang.
// This file is part of Kulupu.

// Kulupu is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Kulupu is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Kulupu.  If not, see <http://www.gnu.org/licenses/>.

mod v1;
mod v2;

pub use self::v1::{ComputeV1, SealV1};
pub use self::v2::{ComputeV2, SealV2};

use log::info;
use codec::{Encode, Decode};
use std::sync::{Arc, Mutex};
use std::cell::RefCell;
use sp_core::H256;
use lazy_static::lazy_static;
use lru_cache::LruCache;
use kulupu_randomx as randomx;
use kulupu_primitives::Difficulty;

lazy_static! {
	static ref SHARED_CACHES: Arc<Mutex<LruCache<H256, Arc<randomx::FullCache>>>> =
		Arc::new(Mutex::new(LruCache::new(2)));
}
thread_local!(static MACHINES: RefCell<Option<(H256, randomx::FullVM)>> = RefCell::new(None));

#[derive(Clone, PartialEq, Eq, Encode, Decode, Debug)]
pub struct Calculation {
	pub pre_hash: H256,
	pub difficulty: Difficulty,
	pub nonce: H256,
}

fn compute_raw(key_hash: &H256, input: &[u8]) -> H256 {
	MACHINES.with(|m| {
		let mut ms = m.borrow_mut();

		let need_new_vm = ms.as_ref().map(|(mkey_hash, _)| {
			mkey_hash != key_hash
		}).unwrap_or(true);

		if need_new_vm {
			let mut shared_caches = SHARED_CACHES.lock().expect("Mutex poisioned");

			if let Some(cache) = shared_caches.get_mut(key_hash) {
				*ms = Some((*key_hash, randomx::FullVM::new(cache.clone())));
			} else {
				info!("At block boundary, generating new RandomX cache with key hash {} ...",
					  key_hash);
				let cache = Arc::new(randomx::FullCache::new(&key_hash[..]));
				shared_caches.insert(*key_hash, cache.clone());
				*ms = Some((*key_hash, randomx::FullVM::new(cache)));
			}
		}

		let work = ms.as_mut()
			.map(|(mkey_hash, vm)| {
				assert_eq!(mkey_hash, key_hash,
						   "Condition failed checking cached key_hash. This is a bug");
				vm.calculate(input)
			})
			.expect("Local MACHINES always set to Some above; qed");

		H256::from(work)
	})
}
