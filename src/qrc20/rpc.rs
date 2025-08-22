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
            "allowance": allowance.to_string(),
            "decimals": token.decimals,
            "symbol": token.symbol,
            "formatted": format_balance(allowance, token.decimals)
        }))
    }

    /// Get QRC-20 token information
    pub fn qrc20_token_info(
        blockchain: &crate::QoraNet,
        params: Value,
    ) -> Result<Value, String> {
        let contract = parse_address(&params["contract"])?;

        let token = blockchain.qrc20_registry.get_token(contract)
            .ok_or("Token not found")?;

        Ok(json!({
            "contractAddress": format!("0x{:x}", contract),
            "name": token.name,
            "symbol": token.symbol,
            "decimals": token.decimals,
            "totalSupply": token.total_supply.to_string(),
            "maxSupply": token.max_supply.map(|s| s.to_string()),
            "mintable": token.mintable,
            "burnable": token.burnable,
            "owner": format!("0x{:x}", token.owner),
            "createdAt": token.created_at,
            "formattedTotalSupply": format_balance(token.total_supply, token.decimals)
        }))
    }

    /// Get all QRC-20 tokens
    pub fn qrc20_list_tokens(
        blockchain: &crate::QoraNet,
        params: Value,
    ) -> Result<Value, String> {
        let limit = params.get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(50) as usize;
        
        let offset = params.get("offset")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        let tokens = blockchain.qrc20_registry.get_all_tokens(limit, offset);
        let total_count = blockchain.qrc20_registry.total_tokens();

        let token_list: Vec<Value> = tokens.into_iter().map(|(address, token)| {
            json!({
                "contractAddress": format!("0x{:x}", address),
                "name": token.name,
                "symbol": token.symbol,
                "decimals": token.decimals,
                "totalSupply": token.total_supply.to_string(),
                "formattedTotalSupply": format_balance(token.total_supply, token.decimals),
                "owner": format!("0x{:x}", token.owner),
                "createdAt": token.created_at
            })
        }).collect();

        Ok(json!({
            "tokens": token_list,
            "totalCount": total_count,
            "limit": limit,
            "offset": offset,
            "hasMore": offset + limit < total_count
        }))
    }

    /// Get transaction history for a token
    pub fn qrc20_transaction_history(
        blockchain: &crate::QoraNet,
        params: Value,
    ) -> Result<Value, String> {
        let contract = parse_address(&params["contract"])?;
        let account = if let Some(acc) = params.get("account") {
            Some(parse_address(acc)?)
        } else {
            None
        };
        
        let limit = params.get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(50) as usize;
        
        let offset = params.get("offset")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        let history = blockchain.qrc20_registry.get_transaction_history(
            contract, 
            account, 
            limit, 
            offset
        );

        let transactions: Vec<Value> = history.into_iter().map(|tx| {
            json!({
                "hash": format!("0x{:x}", tx.hash),
                "blockNumber": tx.block_number,
                "timestamp": tx.timestamp,
                "from": format!("0x{:x}", tx.from),
                "to": tx.to.map(|addr| format!("0x{:x}", addr)),
                "amount": tx.amount.to_string(),
                "type": tx.transaction_type,
                "gasUsed": tx.gas_used,
                "status": tx.status
            })
        }).collect();

        Ok(json!({
            "transactions": transactions,
            "contractAddress": format!("0x{:x}", contract),
            "account": account.map(|addr| format!("0x{:x}", addr)),
            "limit": limit,
            "offset": offset
        }))
    }

    /// Get total supply of a token
    pub fn qrc20_total_supply(
        blockchain: &crate::QoraNet,
        params: Value,
    ) -> Result<Value, String> {
        let contract = parse_address(&params["contract"])?;

        let token = blockchain.qrc20_registry.get_token(contract)
            .ok_or("Token not found")?;

        Ok(json!({
            "totalSupply": token.total_supply.to_string(),
            "decimals": token.decimals,
            "symbol": token.symbol,
            "formatted": format_balance(token.total_supply, token.decimals)
        }))
    }

    /// Batch balance query for multiple accounts
    pub fn qrc20_batch_balance(
        blockchain: &crate::QoraNet,
        params: Value,
    ) -> Result<Value, String> {
        let contract = parse_address(&params["contract"])?;
        let accounts = params["accounts"]
            .as_array()
            .ok_or("Missing 'accounts' array")?;

        let token = blockchain.qrc20_registry.get_token(contract)
            .ok_or("Token not found")?;

        let mut balances = Vec::new();
        for account_val in accounts {
            let account = parse_address(account_val)?;
            let balance = token.balance_of(account);
            
            balances.push(json!({
                "account": format!("0x{:x}", account),
                "balance": balance.to_string(),
                "formatted": format_balance(balance, token.decimals)
            }));
        }

        Ok(json!({
            "contractAddress": format!("0x{:x}", contract),
            "symbol": token.symbol,
            "decimals": token.decimals,
            "balances": balances
        }))
    }

    /// Get contract events (logs)
    pub fn qrc20_get_events(
        blockchain: &crate::QoraNet,
        params: Value,
    ) -> Result<Value, String> {
        let contract = parse_address(&params["contract"])?;
        let from_block = params.get("fromBlock")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let to_block = params.get("toBlock")
            .and_then(|v| v.as_u64())
            .unwrap_or(u64::MAX);
        
        let event_types = if let Some(types) = params.get("eventTypes") {
            types.as_array()
                .ok_or("eventTypes must be an array")?
                .iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect()
        } else {
            vec!["Transfer".to_string(), "Approval".to_string(), "Mint".to_string(), "Burn".to_string()]
        };

        let events = blockchain.qrc20_registry.get_contract_events(
            contract, 
            from_block, 
            to_block, 
            &event_types
        );

        let event_list: Vec<Value> = events.into_iter().map(|event| {
            json!({
                "blockNumber": event.block_number,
                "transactionHash": format!("0x{:x}", event.transaction_hash),
                "eventType": event.event_type,
                "data": event.data,
                "timestamp": event.timestamp
            })
        }).collect();

        Ok(json!({
            "contractAddress": format!("0x{:x}", contract),
            "fromBlock": from_block,
            "toBlock": to_block,
            "events": event_list,
            "count": event_list.len()
        }))
    }
}

// Helper functions

/// Parse address from JSON value
fn parse_address(value: &Value) -> Result<H160, String> {
    let addr_str = value.as_str()
        .ok_or("Address must be a string")?;
    
    let addr_clean = addr_str.strip_prefix("0x").unwrap_or(addr_str);
    
    if addr_clean.len() != 40 {
        return Err("Invalid address length".to_string());
    }
    
    let bytes = hex::decode(addr_clean)
        .map_err(|_| "Invalid hex address".to_string())?;
    
    if bytes.len() != 20 {
        return Err("Address must be 20 bytes".to_string());
    }
    
    Ok(H160::from_slice(&bytes))
}

/// Parse U256 from JSON value
fn parse_u256(value: &Value) -> Result<U256, String> {
    match value {
        Value::String(s) => {
            if s.starts_with("0x") {
                U256::from_str_radix(&s[2..], 16)
                    .map_err(|_| "Invalid hex number".to_string())
            } else {
                U256::from_dec_str(s)
                    .map_err(|_| "Invalid decimal number".to_string())
            }
        }
        Value::Number(n) => {
            if let Some(val) = n.as_u64() {
                Ok(U256::from(val))
            } else {
                Err("Number too large".to_string())
            }
        }
        _ => Err("Amount must be a string or number".to_string())
    }
}

/// Format balance with proper decimals
fn format_balance(balance: U256, decimals: u8) -> String {
    let divisor = U256::from(10).pow(U256::from(decimals));
    let integer_part = balance / divisor;
    let fractional_part = balance % divisor;
    
    if fractional_part.is_zero() {
        integer_part.to_string()
    } else {
        let frac_str = format!("{:0width$}", fractional_part, width = decimals as usize);
        let trimmed = frac_str.trim_end_matches('0');
        
        if trimmed.is_empty() {
            integer_part.to_string()
        } else {
            format!("{}.{}", integer_part, trimmed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_address() {
        let addr_val = json!("0x742d35Cc6621C0532c5C3d30485e1c463E2D0E6C");
        let result = parse_address(&addr_val);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_u256() {
        let num_val = json!("1000000000000000000");
        let result = parse_u256(&num_val);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), U256::from(1_000_000_000_000_000_000_u64));
    }

    #[test]
    fn test_format_balance() {
        let balance = U256::from(1_500_000_000_000_000_000_u64); // 1.5 tokens
        let formatted = format_balance(balance, 18);
        assert_eq!(formatted, "1.5");
        
        let balance_whole = U256::from(2_000_000_000_000_000_000_u64); // 2.0 tokens
        let formatted_whole = format_balance(balance_whole, 18);
        assert_eq!(formatted_whole, "2");
    }
}
