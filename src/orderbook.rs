use near_sdk::{
	env,
	collections::{
		UnorderedMap,
		TreeMap,
		Vector
	},
	json_types::{U128, U64},
	borsh::{
		self, 
		BorshDeserialize, 
		BorshSerialize
	}
};
use std::{
	cmp,
	convert::TryInto,
	collections::HashMap
};
use serde_json::json;

use crate::order;
use crate::logger;

pub type Order = order::Order;

#[derive(BorshDeserialize, BorshSerialize)]
pub struct PriceData {
	pub share_liquidity: u128,
	pub orders: TreeMap<u128, Order>
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct AccountData {
	pub balance: u128,
	pub spent: u128,
	pub to_spend: u128,
	pub open_orders: TreeMap<u128, u128> // Check if we need order id or can just keep track of balance of open orders - for now open order id mapped to price
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct Orderbook {
	pub market_id: u64,
	pub outcome_id: u64,
	pub price_data: TreeMap<u128, PriceData>,
	pub user_data: UnorderedMap<String, AccountData>,
	pub nonce: u128,
}

impl Orderbook {

	fn new_account(&self, account_id: String) -> AccountData {
		AccountData {
			balance: 0,
			spent: 0,
			to_spend: 0,
			open_orders: TreeMap::new(format!("{}:open_orders:{}:{}", account_id, self.market_id, self.outcome_id).as_bytes().to_vec())
		}
	}

	fn new_price(&self, price: u128) -> PriceData {
		PriceData {
			share_liquidity: 0,
			orders: TreeMap::new(format!("price_data:{}:{}:{}", self.market_id, self.outcome_id, price).as_bytes().to_vec())
		}
	}

	pub fn new(
		market_id: u64,
		outcome: u64
	) -> Self {
		Self {
			market_id,
			price_data: TreeMap::new(format!("price_data:{}:{}", market_id, outcome).as_bytes().to_vec()),
			user_data: UnorderedMap::new(format!("user_data:{}:{}", market_id, outcome).as_bytes().to_vec()),
			nonce: 0,
			outcome_id: outcome,
		}
	}

    // Grabs latest nonce
	fn new_order_id(
		&mut self
	) -> u128 {
		let id = self.nonce;
		self.nonce = self.nonce + 1;
		return id;
	}

    // Places order in orderbook
	pub fn new_order(
		&mut self,
		market_id: u64,
		account_id: String, 
		outcome: u64, 
		spend: u128, 
		shares: u128, 
		price: u128, 
		filled: u128, 
		shares_filled: u128,
		affiliate_account_id: Option<String>
	){
		let order_id = self.new_order_id();
		let new_order = Order::new(order_id, account_id.to_string(), market_id, spend, filled, shares, shares_filled, price, affiliate_account_id.clone());
		let mut user_data = self.user_data.get(&account_id).unwrap_or(self.new_account(account_id.to_string()));
		user_data.balance += shares_filled;
		user_data.spent += filled;
		user_data.to_spend += spend;
		
		logger::log_update_user_balance(account_id.to_string(), market_id, outcome, user_data.balance, user_data.to_spend, user_data.spent);
		
		let left_to_spend = spend - filled;
		let mut fill_price = 0;
		
		if shares_filled > 0 {
			fill_price = filled / shares_filled;
		}
		
		
		// TODO: add to affiliate_earnings
		// if left_to_spend < 100 the order counts as filled
		if left_to_spend < 100 {
			self.user_data.insert(&account_id, &user_data);
			
			logger::log_order_filled_at_placement(&new_order, outcome, fill_price);
			return;
		}
		
		// TODO: expect that we don't need a reference to the order
		user_data.open_orders.insert(&order_id, &price);
		self.user_data.insert(&account_id, &user_data);

		let mut price_data = self.price_data.get(&price).unwrap_or(self.new_price(price));
		price_data.orders.insert(&order_id, &new_order);
		price_data.share_liquidity += (spend - filled) / price;
		self.price_data.insert(&price, &price_data);

		logger::log_order_placed(&new_order, outcome, fill_price);
	}


	pub fn cancel_order(&mut self, order: Order) -> u128 {
		let mut price_data = self.price_data.get(&order.price).unwrap();
		let mut user_data = self.user_data.get(&order.creator).unwrap();

		let to_return = order.spend - order.filled; 

		price_data.share_liquidity -= to_return / order.price;
		price_data.orders.remove(&order.id);

		if price_data.orders.len() == 0 {
			self.price_data.remove(&order.price);
		} else {
			self.price_data.insert(&order.price, &price_data);
		}
		
		user_data.open_orders.remove(&order.id);
		user_data.to_spend -= order.spend - order.filled;
		logger::log_update_user_balance(order.creator.to_string(), order.market_id, self.outcome_id, user_data.balance, user_data.to_spend, user_data.spent);
		self.user_data.insert(&order.creator, &user_data);

		logger::log_order_closed(&order, self.market_id, self.outcome_id);

		return to_return;
	}


	// TODO: add to affiliate_earnings
	pub fn fill_order(
		&mut self, 
		mut order: Order, 
		shares_to_fill: u128,
		close_order: bool
	) {
		let mut user_data = self.user_data.get(&order.creator).expect("order is owned by non-existent user");
		let mut price_data = self.price_data.get(&order.price).expect("no price data for this order");

		user_data.balance += shares_to_fill;
		user_data.spent += shares_to_fill * order.price;
		price_data.share_liquidity -= shares_to_fill;


		if close_order {
			user_data.open_orders.remove(&order.id);
			price_data.orders.remove(&order.id);
			logger::log_order_closed(&order, self.market_id, self.outcome_id);
		}  else {
			order.filled += shares_to_fill * order.price;
			order.shares_filled += shares_to_fill;
			price_data.orders.insert(&order.id, &order);
		}

		if price_data.orders.len() == 0 {
			self.price_data.remove(&order.price);
		} else {
			self.price_data.insert(&order.price, &price_data);
		}

		self.user_data.insert(&order.creator, &user_data);
		logger::log_order_filled(&order, shares_to_fill, self.market_id, self.outcome_id);
		logger::log_update_user_balance(order.creator, order.market_id, self.outcome_id, user_data.balance, user_data.to_spend, user_data.spent);
	}

	pub fn fill_best_orders(
		&mut self, 
		mut shares_to_fill: u128
	) -> u128 {
		let fill_price = match self.price_data.max() {
			Some(price) => price,
			None => return 0
		};

		let orders = self.price_data.get(&fill_price).expect("this price shouldn't exist if there are no orders to be filled").orders.to_vec();

		let mut shares_filled = 0;
		for (_, order) in orders.iter() {
			env::log(b"get here6");
			if shares_to_fill < 1 { break;} 
			let shares_fillable_for_order = (order.spend - order.filled) / order.price;

			if shares_fillable_for_order == 0 {panic!("should never be 0")}			
			let filling = cmp::min(shares_fillable_for_order, shares_to_fill); 
			shares_filled += filling;
			if shares_to_fill < shares_fillable_for_order {
				self.fill_order(order.clone(), filling, false);
				break;
			} else if shares_to_fill > shares_fillable_for_order {
				self.fill_order(order.clone(), filling, true);
			} else {
				self.fill_order(order.clone(), filling, true);
				break;
			}
			env::log(b"get here7");

			shares_to_fill -= filling;
		}
		env::log(b"get here8");

		return shares_filled;
	}

	pub fn get_depth_up_to_price(&self, max_shares: u128, min_price: u128) -> (u128, u128) {
		let mut best_price = self.price_data.max().unwrap_or(0);

		let mut depth = 0;
		let mut depth_price_prod_sum = 0;
		while best_price > min_price && max_shares > depth {
			let shares_left_to_fill = max_shares - depth;
			let price_data = self.price_data.get(&best_price).expect("Expected there to be a value at this key");
			let liquidity = cmp::min(shares_left_to_fill, price_data.share_liquidity);
			depth_price_prod_sum += liquidity * best_price;

			depth += liquidity;
			best_price = self.price_data.lower(&best_price).unwrap_or(0);

		}

		if depth == 0 {return (0, 0);}

		return (cmp::min(max_shares, depth), depth_price_prod_sum / depth);
	}
}
