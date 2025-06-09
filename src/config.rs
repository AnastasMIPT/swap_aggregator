// src/config.rs
use alloy::primitives::{address, Address, U256};

// Адреса токенов в сети Polygon
pub const USDC_ADDRESS: Address = address!("3c499c542cEF5E3811e1192ce70d8cC03d5c3359"); // USDC (USD Coin)
pub const USDC_E_ADDRESS: Address = address!("2791bca1f2de4661ed88a30c99a7a9449aa84174"); // USDC.e (Bridged USDC)
pub const WETH_ADDRESS: Address = address!("7ceB23fD6bC0adD59E62ac25578270cFf1b9f619"); // WETH (Wrapped ETH)

// Decimals для токенов (количество знаков после запятой)
pub const USDC_DECIMALS: u8 = 6;  // 1 USDC = 1,000,000 units
pub const WETH_DECIMALS: u8 = 18; // 1 WETH = 1,000,000,000,000,000,000 units

// Степени 10 для конвертации decimals
pub const USDC_SCALE: U256 = U256::from_limbs([1_000_000, 0, 0, 0]); // 10^6
pub const WETH_SCALE: U256 = U256::from_limbs([1_000_000_000_000_000_000, 0, 0, 0]); // 10^18

// Параметры свапа (decimal значения для удобства)
pub const TOTAL_USDC_DECIMAL: f64 = 1000000.0;      // 1.0 USDC для обмена
pub const NUM_CHUNKS: u64 = 100;                // Разделить на 100 частей
pub const CHUNK_USDC_DECIMAL: f64 = TOTAL_USDC_DECIMAL / NUM_CHUNKS as f64;

// Функция для получения CHUNK_USDC_AMOUNT в raw units
pub fn get_chunk_usdc_amount() -> U256 {
    usdc_from_decimal(CHUNK_USDC_DECIMAL)
}

// Factory адреса для получения точных адресов пулов (для сети Polygon)
pub const QUICKSWAP_V2_FACTORY: Address = address!("5757371414417b8C6CAad45bAeF941aBc7d3Ab32");
pub const SUSHISWAP_V2_FACTORY: Address = address!("c35DADB65012eC5796536bD9864eD8773aBc74C4"); // Правильный адрес для Polygon

// Статические адреса пулов
pub const UNISWAP_V2_POOL_ADDRESS: Address = address!("67473ebdBFD1e6Fc4367462d55eD1eE56e1963FA"); // Uniswap V2 USDC/WETH

/// Конвертирует USDC из raw units в человекочитаемое значение
pub fn usdc_to_decimal(raw_amount: U256) -> f64 {
    raw_amount.to::<u64>() as f64 / USDC_SCALE.to::<u64>() as f64
}

/// Конвертирует WETH из raw units в человекочитаемое значение
pub fn weth_to_decimal(raw_amount: U256) -> f64 {
    raw_amount.to::<u128>() as f64 / WETH_SCALE.to::<u128>() as f64
}

/// Конвертирует USDC из человекочитаемого значения в raw units
pub fn usdc_from_decimal(decimal_amount: f64) -> U256 {
    U256::from((decimal_amount * USDC_SCALE.to::<u64>() as f64) as u64)
} 