use super::*;

#[test]
fn test_bst_additions() {
	testing_env!(get_context(carol()));
	
	let mut contract = Markets::default();
	contract.claim_fdai();
	contract.create_market(3, "Hi!".to_string(), 100010101001010);
	contract.place_order(0, 0, 100000, 500);
	contract.place_order(0, 0, 100000, 300);
	contract.place_order(0, 0, 100000, 600);
	contract.place_order(0, 0, 100000, 700);
	contract.place_order(0, 0, 100000, 550);
	contract.place_order(0, 0, 100000, 350);
	contract.place_order(0, 0, 100000, 500);


	let open_orders = &contract.get_market(0).orderbooks.get(&0).as_ref().unwrap().open_orders;
	let order_0 = open_orders.get(&0).unwrap();
	let order_1 = open_orders.get(&1).unwrap();
	let order_2 = open_orders.get(&2).unwrap();
	let order_3 = open_orders.get(&3).unwrap();
	let order_4 = open_orders.get(&4).unwrap();
	let order_5 = open_orders.get(&5).unwrap();

	// [0]
	assert_eq!(order_0.parent, None);
	assert_eq!(order_0.better_order_id, Some(2));
	assert_eq!(order_0.worse_order_id, Some(1));

	//   [0]
	//  /
	// [1] <-
	assert_eq!(order_1.parent, Some(order_0.id));
	assert_eq!(order_1.better_order_id, Some(5));
	assert_eq!(order_1.worse_order_id, None);

	//   [0]
	//  /  \
	// [1]  [2] <-
	assert_eq!(order_2.parent, Some(order_0.id));
	assert_eq!(order_2.better_order_id, Some(3));
	assert_eq!(order_2.worse_order_id, Some(4));

	//   [0]
	//  /  \
	// [1]  [2] 
	//         \
	//		   [3] <-
	assert_eq!(order_3.parent, Some(2));
	assert_eq!(order_3.better_order_id, None);
	assert_eq!(order_3.worse_order_id, None);

	//   [0]
	//  /  \
	// [1]  [2] 
	//     /   \
	//-> [4]   [3] 
	assert_eq!(order_4.parent, Some(2));
	assert_eq!(order_4.better_order_id, None);
	assert_eq!(order_4.worse_order_id, Some(6));

	//   [0]
	//  /     \
	// [1]       [2] 
	//    \     /   \
	//  ->[5] [4]   [3] 
	assert_eq!(order_5.parent, Some(1));
	assert_eq!(order_5.better_order_id, None);
	assert_eq!(order_5.worse_order_id, None);
}	

#[test]
fn test_bst_removal() {
	testing_env!(get_context(carol()));
	
	let mut contract = Markets::default();
	contract.claim_fdai();
	contract.create_market(3, "Hi!".to_string(), 100010101001010);
	contract.place_order(0, 0, 100000, 500);
	contract.place_order(0, 0, 100000, 300);
	contract.place_order(0, 0, 100000, 600);
	contract.place_order(0, 0, 100000, 700);
	contract.place_order(0, 0, 100000, 550);
	contract.place_order(0, 0, 100000, 350);
	contract.cancel_order(0, 0, 0);
	contract.cancel_order(0, 0, 1);

	let open_orders = &contract.get_market(0).orderbooks.get(&0).as_ref().unwrap().open_orders;
	let order_2 = open_orders.get(&2).unwrap();
	let order_3 = open_orders.get(&3).unwrap();
	let order_4 = open_orders.get(&4).unwrap();
	let order_5 = open_orders.get(&5).unwrap();

	assert_eq!(order_2.parent, None);
	assert_eq!(order_2.better_order_id, Some(3));
	assert_eq!(order_2.worse_order_id, Some(4));

	assert_eq!(order_3.parent, Some(2));
	assert_eq!(order_3.better_order_id, None);
	assert_eq!(order_3.worse_order_id, None);
	
	assert_eq!(order_4.parent, Some(2));
	assert_eq!(order_4.better_order_id, None);
	assert_eq!(order_4.worse_order_id, Some(5));

	assert_eq!(order_5.parent, Some(4));
	assert_eq!(order_5.better_order_id, None);
	assert_eq!(order_5.worse_order_id, None);
}