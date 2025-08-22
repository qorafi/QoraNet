/// JSON-RPC methods for QRC-20 integration
use serde_json::{Value, json};
use primitive_types::{H160, H256, U256};
use super::{QRC20Transaction, QRC20Error};

/// QRC-20 RPC handler
pub struct QRC20RpcHandler;

impl QRC20RpcHandler {
    /// Deploy QRC-20 token
    pub fn deploy_qrc20(
        blockchain: &mut crate::QoraNet,
        params: Value,
    ) -> Result<Value, String> {
        let caller = parse_address(&params["from"])?;
        let name = params["name"].as_str().ok_or("Missing 'name' field")?.to_string();
        let symbol = params["symbol"].as_str().ok_or("Missing 'symbol' field")?.to_string();
        let decimals = params["decimals"].as_u64().ok_or("Missing 'decimals' field")? as u8;
        let total_supply = parse_u256(&params["totalSupply"])?;

        // Optional parameters
        let max_supply = if let Some(max_val) = params.get("maxSupply") {
            Some(parse_u256(max_val)?)
        } else {
            None
        };

        let mintable = params.get("mintable").and_then(|v| v.as_bool());
        let burnable = params.get("burnable").and_then(|v| v.as_bool());

        let transaction = QRC20Transaction::Deploy {
            name: name.clone(),
            symbol: symbol.clone(),
            decimals,
            total_supply,
            max_supply,
            mintable,
            burnable,
        };

        let gas_limit = params.get("gasLimit")
            .and_then(|v| v.as_u64())
            .unwrap_or(500_000);

        let event = blockchain.process_qrc20_transaction(caller, transaction, gas_limit)?;

        let contract_address = match event {
            crate::QRC20Event::Deploy { contract, .. } => contract,
            _ => return Err("Unexpected event type".to_string()),
        };

        Ok(json!({
            "contractAddress": format!("0x{:x}", contract_address),
            "transactionHash": format!("0x{:x}", H256::random()),
            "status": "success",
            "gasUsed": gas_limit,
            "tokenInfo": {
                "name": name,
                "symbol": symbol,
                "decimals": decimals,
                "totalSupply": total_supply.to_string()
            }
        }))
    }

    /// Transfer QRC-20 tokens
    pub fn qrc20_transfer(
        blockchain: &mut crate::QoraNet,
        params: Value,
    ) -> Result<Value, String> {
        let caller = parse_address(&params["from"])?;
        let contract = parse_address(&params["contract"])?;
        let to = parse_address(&params["to"])?;
        let amount = parse_u256(&params["amount"])?;

        let transaction = QRC20Transaction::Transfer { contract, to, amount };
        let gas_limit = params.get("gasLimit")
            .and_then(|v| v.as_u64())
            .unwrap_or(50_000);

        let event = blockchain.process_qrc20_transaction(caller, transaction, gas_limit)?;

        match event {
            crate::QRC20Event::Transfer { from, to, amount, .. } => {
                Ok(json!({
                    "transactionHash": format!("0x{:x}", H256::random()),
                    "status": "success",
                    "gasUsed": gas_limit,
                    "from": format!("0x{:x}", from),
                    "to": format!("0x{:x}", to),
                    "amount": amount.to_string()
                }))
            }
            _ => Err("Unexpected event type".to_string()),
        }
    }

    /// Approve QRC-20 spending
    pub fn qrc20_approve(
        blockchain: &mut crate::QoraNet,
        params: Value,
    ) -> Result<Value, String> {
        let caller = parse_address(&params["from"])?;
        let contract = parse_address(&params["contract"])?;
        let spender = parse_address(&params["spender"])?;
        let amount = parse_u256(&params["amount"])?;

        let transaction = QRC20Transaction::Approve { contract, spender, amount };
        let gas_limit = params.get("gasLimit")
            .and_then(|v| v.as_u64())
            .unwrap_or(45_000);

        let event = blockchain.process_qrc20_transaction(caller, transaction, gas_limit)?;

        match event {
            crate::QRC20Event::Approval { owner, spender, amount, .. } => {
                Ok(json!({
                    "transactionHash": format!("0x{:x}", H256::random()),
                    "status": "success",
                    "gasUsed": gas_limit,
                    "owner": format!("0x{:x}", owner),
                    "spender": format!("0x{:x}", spender),
                    "amount": amount.to_string()
                }))
            }
            _ => Err("Unexpected event type".to_string()),
        }
    }

    /// Transfer tokens from one address to another (requires allowance)
    pub fn qrc20_transfer_from(
        blockchain: &mut crate::QoraNet,
        params: Value,
    ) -> Result<Value, String> {
        let caller = parse_address(&params["from"])?; // Spender
        let contract = parse_address(&params["contract"])?;
        let from = parse_address(&params["tokenOwner"])?;
        let to = parse_address(&params["to"])?;
        let amount = parse_u256(&params["amount"])?;

        let transaction = QRC20Transaction::TransferFrom { contract, from, to, amount };
        let gas_limit = params.get("gasLimit")
            .and_then(|v| v.as_u64())
            .unwrap_or(55_000);

        let event = blockchain.process_qrc20_transaction(caller, transaction, gas_limit)?;

        match event {
            crate::QRC20Event::Transfer { from, to, amount, .. } => {
                Ok(json!({
                    "transactionHash": format!("0x{:x}", H256::random()),
                    "status": "success",
                    "gasUsed": gas_limit,
                    "from": format!("0x{:x}", from),
                    "to": format!("0x{:x}", to),
                    "amount": amount.to_string()
                }))
            }
            _ => Err("Unexpected event type".to_string()),
        }
    }

    /// Mint new tokens (only token owner)
    pub fn qrc20_mint(
        blockchain: &mut crate::QoraNet,
        params: Value,
    ) -> Result<Value, String> {
        let caller = parse_address(&params["from"])?;
        let contract = parse_address(&params["contract"])?;
        let to = parse_address(&params["to"])?;
        let amount = parse_u256(&params["amount"])?;

        let transaction = QRC20Transaction::Mint { contract, to, amount };
        let gas_limit = params.get("gasLimit")
            .and_then(|v| v.as_u64())
            .unwrap_or(60_000);

        let event = blockchain.process_qrc20_transaction(caller, transaction, gas_limit)?;

        match event {
            crate::QRC20Event::Mint { to, amount, .. } => {
                Ok(json!({
                    "transactionHash": format!("0x{:x}", H256::random()),
                    "status": "success",
                    "gasUsed": gas_limit,
                    "to": format!("0x{:x}", to),
                    "amount": amount.to_string()
                }))
            }
            _ => Err("Unexpected event type".to_string()),
        }
    }

    /// Burn tokens
    pub fn qrc20_burn(
        blockchain: &mut crate::QoraNet,
        params: Value,
    ) -> Result<Value, String> {
        let caller = parse_address(&params["from"])?;
        let contract = parse_address(&params["contract"])?;
        let amount = parse_u256(&params["amount"])?;

        let transaction = QRC20Transaction::Burn { contract, amount };
        let gas_limit = params.get("gasLimit")
            .and_then(|v| v.as_u64())
            .unwrap_or(40_000);

        let event = blockchain.process_qrc20_transaction(caller, transaction, gas_limit)?;

        match event {
            crate::QRC20Event::Burn { from, amount, .. } => {
                Ok(json!({
                    "transactionHash": format!("0x{:x}", H256::random()),
                    "status": "success",
                    "gasUsed": gas_limit,
                    "from": format!("0x{:x}", from),
                    "amount": amount.to_string()
                }))
            }
            _ => Err("Unexpected event type".to_string()),
        }
    }

    /// Get QRC-20 balance
    pub fn qrc20_balance(
        blockchain: &crate::QoraNet,
        params: Value,
    ) -> Result<Value, String> {
        let contract = parse_address(&params["contract"])?;
        let account = parse_address(&params["account"])?;

        let token = blockchain.qrc20_registry.get_token(contract)
            .ok_or("Token not found")?;
        
        let balance = token.balance_of(account);

        Ok(json!({
            "balance": balance.to_string(),
            "decimals": token.decimals,
            "symbol": token.symbol,
            "formatted": format_balance(balance, token.decimals)
        }))
    }

    /// Get QRC-20 allowance
    pub fn qrc20_allowance(
        blockchain: &crate::QoraNet,
        params: Value,
    ) -> Result<Value, String> {
        let contract = parse_address(&params["contract"])?;
        let owner = parse_address(&params["owner"])?;
        let spender = parse_address(&params["spender"])?;

        let token = blockchain.qrc20_registry.get_token(contract)
            .ok_or("Token not found")?;
        
        let allowance = token.allowance(owner, spender);

        Ok(json!({
            "
