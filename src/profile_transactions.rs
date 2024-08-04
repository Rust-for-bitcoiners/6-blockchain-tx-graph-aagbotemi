use bitcoin::hash_types::Txid;
use bitcoin::Block;
use bitcoincore_rpc::RpcApi;
use bitcoincore_rpc::{Auth, Client};
use std::env;

use super::graph::Graph;

lazy_static! {
    static ref RPC_CLIENT: Client = {
        dotenv::dotenv().ok();
        let rpc_url: String = env::var("BITCOIN_RPC_URL").expect("BITCOIN_RPC_URL must be set");
        let rpc_user: String = env::var("BITCOIN_RPC_USER").expect("BITCOIN_RPC_USER must be set");
        let rpc_password: String =
            env::var("BITCOIN_RPC_PASSWORD").expect("BITCOIN_RPC_PASSWORD must be set");
        Client::new(&rpc_url, Auth::UserPass(rpc_user, rpc_password)).unwrap()
    };
}

pub fn build_transaction_graph(start_height: u64, end_height: u64) -> Graph<Txid> {
    // Every Transaction has a set of Inputs and outputs
    // Each Input refers to an output of some earlier transaction
    // We say a Transaction A funds Transaction B if an ouput of A is an input of B
    // Build a graph where nodes represents Txid and an edge (t1, t2) is in the graph
    // if the transaction t1 funds transaction t2
    let mut graph = Graph::new();

    for height in start_height..=end_height {
        let block_hash = RPC_CLIENT.get_block_hash(height).unwrap();
        let block: Block = RPC_CLIENT.get_block(&block_hash).unwrap();

        for tx in block.txdata {
            let txid = tx.compute_txid();
            graph.insert_vertex(txid);

            for input in tx.input {
                if !input.previous_output.is_null() {
                    let prev_txid = input.previous_output.txid;
                    graph.insert_edge(prev_txid, txid);
                }
            }
        }
    }

    graph
}

#[cfg(test)]
mod tests {

    use super::*;
    use bitcoin::hash_types::Txid;
    use bitcoind::bitcoincore_rpc::{bitcoin::Amount, RpcApi};
    use bitcoind::BitcoinD;
    use std::env;

    fn setup_bitcoind() -> BitcoinD {
        dotenv::dotenv().ok();
        let bitcoind_path = env::var("BITCOIND_PATH").expect("BITCOIND_PATH must be set");
        BitcoinD::new(bitcoind_path).unwrap()
    }

    #[test]
    fn test_build_transaction_graph() {
        let bitcoind = setup_bitcoind();
        let client = &bitcoind.client;
        
        let alice = bitcoind.create_wallet("alice").unwrap();
        assert_eq!(
            Amount::from_btc(50.0).unwrap(),
            alice.get_balances().unwrap().mine.trusted
        );
        let address = alice.get_new_address(None, None).unwrap();
        let amount = Amount::from_btc(10.0).unwrap();

        client.generate_to_address(101, &address).unwrap(); // Generate 101 blocks to get some mature coins
        client.generate_to_address(1, &address).unwrap();

        let txid = alice
            .send_to_address(&address, amount, None, None, None, None, None, None)
            .unwrap();


        let graph = build_transaction_graph(0, 102);
        let check = graph.contains_vertex(&txid);

        assert!(check, "Txn not found in graph");
    }
}
