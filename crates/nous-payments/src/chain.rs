use serde::{Deserialize, Serialize};

use nous_core::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Chain {
    Ethereum,
    Solana,
    Bitcoin,
    Polygon,
    Arbitrum,
    Optimism,
    Base,
    Local,
}

impl Chain {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Ethereum => "Ethereum",
            Self::Solana => "Solana",
            Self::Bitcoin => "Bitcoin",
            Self::Polygon => "Polygon",
            Self::Arbitrum => "Arbitrum",
            Self::Optimism => "Optimism",
            Self::Base => "Base",
            Self::Local => "Local",
        }
    }

    pub fn chain_id(&self) -> u64 {
        match self {
            Self::Ethereum => 1,
            Self::Polygon => 137,
            Self::Arbitrum => 42161,
            Self::Optimism => 10,
            Self::Base => 8453,
            Self::Solana => 0,
            Self::Bitcoin => 0,
            Self::Local => 31337,
        }
    }

    pub fn native_token(&self) -> &'static str {
        match self {
            Self::Ethereum | Self::Arbitrum | Self::Optimism | Self::Base | Self::Local => "ETH",
            Self::Polygon => "MATIC",
            Self::Solana => "SOL",
            Self::Bitcoin => "BTC",
        }
    }

    pub fn decimals(&self) -> u8 {
        match self {
            Self::Bitcoin => 8,
            Self::Solana => 9,
            _ => 18,
        }
    }

    pub fn is_evm(&self) -> bool {
        matches!(
            self,
            Self::Ethereum
                | Self::Polygon
                | Self::Arbitrum
                | Self::Optimism
                | Self::Base
                | Self::Local
        )
    }

    pub fn from_chain_id(id: u64) -> Result<Self> {
        match id {
            1 => Ok(Self::Ethereum),
            137 => Ok(Self::Polygon),
            42161 => Ok(Self::Arbitrum),
            10 => Ok(Self::Optimism),
            8453 => Ok(Self::Base),
            31337 => Ok(Self::Local),
            _ => Err(Error::InvalidInput(format!("unknown chain id: {id}"))),
        }
    }

    pub fn block_time_ms(&self) -> u64 {
        match self {
            Self::Ethereum => 12000,
            Self::Polygon => 2000,
            Self::Arbitrum => 250,
            Self::Optimism => 2000,
            Self::Base => 2000,
            Self::Solana => 400,
            Self::Bitcoin => 600000,
            Self::Local => 1000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainAddress {
    pub chain: Chain,
    pub address: String,
}

impl ChainAddress {
    pub fn new(chain: Chain, address: impl Into<String>) -> Result<Self> {
        let address = address.into();
        Self::validate_format(chain, &address)?;
        Ok(Self { chain, address })
    }

    fn validate_format(chain: Chain, address: &str) -> Result<()> {
        if address.is_empty() {
            return Err(Error::InvalidInput("address cannot be empty".into()));
        }

        if chain.is_evm() {
            if !address.starts_with("0x") || address.len() != 42 {
                return Err(Error::InvalidInput(
                    "EVM address must be 0x-prefixed and 42 characters".into(),
                ));
            }
            if !address[2..].chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(Error::InvalidInput(
                    "EVM address must contain only hex characters after 0x".into(),
                ));
            }
        }

        Ok(())
    }

    pub fn display_short(&self) -> String {
        if self.address.len() > 10 {
            format!(
                "{}...{}",
                &self.address[..6],
                &self.address[self.address.len() - 4..]
            )
        } else {
            self.address.clone()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub symbol: String,
    pub name: String,
    pub chain: Chain,
    pub decimals: u8,
    pub contract_address: Option<String>,
}

impl Token {
    pub fn native(chain: Chain) -> Self {
        Self {
            symbol: chain.native_token().into(),
            name: format!("{} Native Token", chain.name()),
            chain,
            decimals: chain.decimals(),
            contract_address: None,
        }
    }

    pub fn erc20(chain: Chain, symbol: &str, name: &str, contract: &str, decimals: u8) -> Self {
        Self {
            symbol: symbol.into(),
            name: name.into(),
            chain,
            decimals,
            contract_address: Some(contract.into()),
        }
    }

    pub fn is_native(&self) -> bool {
        self.contract_address.is_none()
    }

    pub fn format_amount(&self, raw: u128) -> String {
        let divisor = 10u128.pow(self.decimals as u32);
        let whole = raw / divisor;
        let frac = raw % divisor;
        if frac == 0 {
            format!("{whole} {}", self.symbol)
        } else {
            let frac_str = format!("{:0>width$}", frac, width = self.decimals as usize);
            let trimmed = frac_str.trim_end_matches('0');
            format!("{whole}.{trimmed} {}", self.symbol)
        }
    }

    pub fn parse_amount(&self, display: &str) -> Result<u128> {
        let s = display.trim().trim_end_matches(&format!(" {}", self.symbol));
        let s = s.trim();

        let divisor = 10u128.pow(self.decimals as u32);

        if let Some((whole, frac)) = s.split_once('.') {
            let whole: u128 = whole
                .parse()
                .map_err(|_| Error::InvalidInput("invalid whole part".into()))?;

            let frac_len = frac.len();
            if frac_len > self.decimals as usize {
                return Err(Error::InvalidInput(format!(
                    "too many decimal places (max {})",
                    self.decimals
                )));
            }

            let padded = format!("{:0<width$}", frac, width = self.decimals as usize);
            let frac_val: u128 = padded
                .parse()
                .map_err(|_| Error::InvalidInput("invalid fractional part".into()))?;

            Ok(whole * divisor + frac_val)
        } else {
            let whole: u128 = s
                .parse()
                .map_err(|_| Error::InvalidInput("invalid amount".into()))?;
            Ok(whole * divisor)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasEstimate {
    pub chain: Chain,
    pub gas_limit: u64,
    pub gas_price_gwei: f64,
    pub estimated_cost_native: u128,
}

impl GasEstimate {
    pub fn new(chain: Chain, gas_limit: u64, gas_price_gwei: f64) -> Self {
        let cost = (gas_limit as f64 * gas_price_gwei * 1e9) as u128;
        Self {
            chain,
            gas_limit,
            gas_price_gwei,
            estimated_cost_native: cost,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_name() {
        assert_eq!(Chain::Ethereum.name(), "Ethereum");
        assert_eq!(Chain::Solana.name(), "Solana");
    }

    #[test]
    fn chain_id() {
        assert_eq!(Chain::Ethereum.chain_id(), 1);
        assert_eq!(Chain::Polygon.chain_id(), 137);
        assert_eq!(Chain::Arbitrum.chain_id(), 42161);
    }

    #[test]
    fn chain_from_id() {
        assert_eq!(Chain::from_chain_id(1).unwrap(), Chain::Ethereum);
        assert_eq!(Chain::from_chain_id(137).unwrap(), Chain::Polygon);
        assert!(Chain::from_chain_id(99999).is_err());
    }

    #[test]
    fn chain_native_token() {
        assert_eq!(Chain::Ethereum.native_token(), "ETH");
        assert_eq!(Chain::Polygon.native_token(), "MATIC");
        assert_eq!(Chain::Bitcoin.native_token(), "BTC");
    }

    #[test]
    fn chain_is_evm() {
        assert!(Chain::Ethereum.is_evm());
        assert!(Chain::Polygon.is_evm());
        assert!(Chain::Arbitrum.is_evm());
        assert!(!Chain::Solana.is_evm());
        assert!(!Chain::Bitcoin.is_evm());
    }

    #[test]
    fn chain_decimals() {
        assert_eq!(Chain::Ethereum.decimals(), 18);
        assert_eq!(Chain::Bitcoin.decimals(), 8);
        assert_eq!(Chain::Solana.decimals(), 9);
    }

    #[test]
    fn chain_block_time() {
        assert_eq!(Chain::Ethereum.block_time_ms(), 12000);
        assert_eq!(Chain::Arbitrum.block_time_ms(), 250);
    }

    #[test]
    fn valid_evm_address() {
        let addr =
            ChainAddress::new(Chain::Ethereum, "0x1234567890abcdef1234567890abcdef12345678")
                .unwrap();
        assert_eq!(addr.chain, Chain::Ethereum);
    }

    #[test]
    fn invalid_evm_address_no_prefix() {
        assert!(
            ChainAddress::new(Chain::Ethereum, "1234567890abcdef1234567890abcdef12345678")
                .is_err()
        );
    }

    #[test]
    fn invalid_evm_address_wrong_length() {
        assert!(ChainAddress::new(Chain::Ethereum, "0x1234").is_err());
    }

    #[test]
    fn invalid_evm_address_non_hex() {
        assert!(
            ChainAddress::new(Chain::Ethereum, "0xGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG")
                .is_err()
        );
    }

    #[test]
    fn empty_address_rejected() {
        assert!(ChainAddress::new(Chain::Solana, "").is_err());
    }

    #[test]
    fn display_short_address() {
        let addr =
            ChainAddress::new(Chain::Ethereum, "0x1234567890abcdef1234567890abcdef12345678")
                .unwrap();
        assert_eq!(addr.display_short(), "0x1234...5678");
    }

    #[test]
    fn native_token() {
        let eth = Token::native(Chain::Ethereum);
        assert_eq!(eth.symbol, "ETH");
        assert!(eth.is_native());
        assert_eq!(eth.decimals, 18);
    }

    #[test]
    fn erc20_token() {
        let usdc = Token::erc20(
            Chain::Ethereum,
            "USDC",
            "USD Coin",
            "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
            6,
        );
        assert_eq!(usdc.symbol, "USDC");
        assert!(!usdc.is_native());
        assert_eq!(usdc.decimals, 6);
    }

    #[test]
    fn format_amount_whole() {
        let eth = Token::native(Chain::Ethereum);
        assert_eq!(eth.format_amount(1_000_000_000_000_000_000), "1 ETH");
    }

    #[test]
    fn format_amount_fractional() {
        let eth = Token::native(Chain::Ethereum);
        let result = eth.format_amount(1_500_000_000_000_000_000);
        assert_eq!(result, "1.5 ETH");
    }

    #[test]
    fn format_amount_small() {
        let usdc = Token::erc20(Chain::Ethereum, "USDC", "USD Coin", "0x...", 6);
        assert_eq!(usdc.format_amount(1_500_000), "1.5 USDC");
    }

    #[test]
    fn parse_amount_whole() {
        let eth = Token::native(Chain::Ethereum);
        assert_eq!(eth.parse_amount("1").unwrap(), 1_000_000_000_000_000_000);
    }

    #[test]
    fn parse_amount_fractional() {
        let eth = Token::native(Chain::Ethereum);
        assert_eq!(
            eth.parse_amount("1.5").unwrap(),
            1_500_000_000_000_000_000
        );
    }

    #[test]
    fn parse_amount_with_symbol() {
        let eth = Token::native(Chain::Ethereum);
        assert_eq!(
            eth.parse_amount("2 ETH").unwrap(),
            2_000_000_000_000_000_000
        );
    }

    #[test]
    fn parse_amount_too_many_decimals() {
        let usdc = Token::erc20(Chain::Ethereum, "USDC", "USD Coin", "0x...", 6);
        assert!(usdc.parse_amount("1.1234567").is_err());
    }

    #[test]
    fn parse_format_roundtrip() {
        let eth = Token::native(Chain::Ethereum);
        let amount = 2_750_000_000_000_000_000u128;
        let display = eth.format_amount(amount);
        let parsed = eth.parse_amount(&display).unwrap();
        assert_eq!(parsed, amount);
    }

    #[test]
    fn gas_estimate() {
        let est = GasEstimate::new(Chain::Ethereum, 21000, 30.0);
        assert!(est.estimated_cost_native > 0);
        assert_eq!(est.gas_limit, 21000);
    }

    #[test]
    fn chain_serializes() {
        let chain = Chain::Ethereum;
        let json = serde_json::to_string(&chain).unwrap();
        let restored: Chain = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, Chain::Ethereum);
    }

    #[test]
    fn token_serializes() {
        let token = Token::native(Chain::Polygon);
        let json = serde_json::to_string(&token).unwrap();
        let restored: Token = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.symbol, "MATIC");
    }

    #[test]
    fn chain_address_serializes() {
        let addr =
            ChainAddress::new(Chain::Ethereum, "0x1234567890abcdef1234567890abcdef12345678")
                .unwrap();
        let json = serde_json::to_string(&addr).unwrap();
        let restored: ChainAddress = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.chain, Chain::Ethereum);
    }
}
