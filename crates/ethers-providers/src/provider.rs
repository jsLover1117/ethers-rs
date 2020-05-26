use ethers_types::{
    Address, Block, BlockId, BlockNumber, Bytes, Filter, Log, Transaction, TransactionReceipt,
    TransactionRequest, TxHash, U256,
};
use ethers_utils as utils;

use crate::{http::Provider as HttpProvider, JsonRpcClient};
use serde::Deserialize;
use url::{ParseError, Url};

use std::{convert::TryFrom, fmt::Debug};

/// An abstract provider for interacting with the [Ethereum JSON RPC
/// API](https://github.com/ethereum/wiki/wiki/JSON-RPC)
#[derive(Clone, Debug)]
pub struct Provider<P>(P);

// JSON RPC bindings
impl<P: JsonRpcClient> Provider<P> {
    /// Gets the current gas price as estimated by the node
    pub async fn get_gas_price(&self) -> Result<U256, P::Error> {
        self.0.request("eth_gasPrice", None::<()>).await
    }

    /// Tries to estimate the gas for the transaction
    pub async fn estimate_gas(
        &self,
        tx: &TransactionRequest,
        block: Option<BlockNumber>,
    ) -> Result<U256, P::Error> {
        let tx = utils::serialize(tx);

        let args = match block {
            Some(block) => vec![tx, utils::serialize(&block)],
            None => vec![tx],
        };

        self.0.request("eth_estimateGas", Some(args)).await
    }

    /// Gets the logs matching a given filter
    pub async fn get_logs(&self, filter: &Filter) -> Result<Vec<Log>, P::Error> {
        self.0.request("eth_getLogs", Some(filter)).await
    }

    /// Gets the accounts on the node
    pub async fn get_accounts(&self) -> Result<Vec<Address>, P::Error> {
        self.0.request("eth_accounts", None::<()>).await
    }

    /// Gets the latest block number via the `eth_BlockNumber` API
    pub async fn get_block_number(&self) -> Result<U256, P::Error> {
        self.0.request("eth_blockNumber", None::<()>).await
    }

    pub async fn get_block(&self, id: impl Into<BlockId>) -> Result<Block<TxHash>, P::Error> {
        self.get_block_gen(id.into(), false).await
    }

    pub async fn get_block_with_txs(
        &self,
        id: impl Into<BlockId>,
    ) -> Result<Block<Transaction>, P::Error> {
        self.get_block_gen(id.into(), true).await
    }

    async fn get_block_gen<Tx: for<'a> Deserialize<'a>>(
        &self,
        id: BlockId,
        include_txs: bool,
    ) -> Result<Block<Tx>, P::Error> {
        let include_txs = utils::serialize(&include_txs);

        match id {
            BlockId::Hash(hash) => {
                let hash = utils::serialize(&hash);
                let args = vec![hash, include_txs];
                self.0.request("eth_getBlockByHash", Some(args)).await
            }
            BlockId::Number(num) => {
                let num = utils::serialize(&num);
                let args = vec![num, include_txs];
                self.0.request("eth_getBlockByNumber", Some(args)).await
            }
        }
    }

    /// Gets the transaction receipt for tx hash
    pub async fn get_transaction_receipt<T: Send + Sync + Into<TxHash>>(
        &self,
        hash: T,
    ) -> Result<TransactionReceipt, P::Error> {
        let hash = hash.into();
        self.0
            .request("eth_getTransactionReceipt", Some(hash))
            .await
    }

    /// Gets the transaction which matches the provided hash via the `eth_getTransactionByHash` API
    pub async fn get_transaction<T: Send + Sync + Into<TxHash>>(
        &self,
        hash: T,
    ) -> Result<Transaction, P::Error> {
        let hash = hash.into();
        self.0.request("eth_getTransactionByHash", Some(hash)).await
    }

    // State mutations

    /// Broadcasts the transaction request via the `eth_sendTransaction` API
    pub async fn call(
        &self,
        tx: TransactionRequest,
        block: Option<BlockNumber>,
    ) -> Result<Bytes, P::Error> {
        let tx = utils::serialize(&tx);
        let block = utils::serialize(&block.unwrap_or(BlockNumber::Latest));
        self.0.request("eth_call", Some(vec![tx, block])).await
    }

    /// Broadcasts the transaction request via the `eth_sendTransaction` API
    pub async fn send_transaction(&self, tx: TransactionRequest) -> Result<TxHash, P::Error> {
        self.0.request("eth_sendTransaction", Some(tx)).await
    }

    /// Broadcasts a raw RLP encoded transaction via the `eth_sendRawTransaction` API
    pub async fn send_raw_transaction(&self, tx: &Transaction) -> Result<TxHash, P::Error> {
        let rlp = utils::serialize(&tx.rlp());
        self.0.request("eth_sendRawTransaction", Some(rlp)).await
    }

    // Account state

    pub async fn get_transaction_count(
        &self,
        from: Address,
        block: Option<BlockNumber>,
    ) -> Result<U256, P::Error> {
        let from = utils::serialize(&from);
        let block = utils::serialize(&block.unwrap_or(BlockNumber::Latest));
        self.0
            .request("eth_getTransactionCount", Some(&[from, block]))
            .await
    }

    pub async fn get_balance(
        &self,
        from: Address,
        block: Option<BlockNumber>,
    ) -> Result<U256, P::Error> {
        let from = utils::serialize(&from);
        let block = utils::serialize(&block.unwrap_or(BlockNumber::Latest));
        self.0.request("eth_getBalance", Some(&[from, block])).await
    }
}

impl TryFrom<&str> for Provider<HttpProvider> {
    type Error = ParseError;

    fn try_from(src: &str) -> Result<Self, Self::Error> {
        Ok(Provider(HttpProvider::new(Url::parse(src)?)))
    }
}
