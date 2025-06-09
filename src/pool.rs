// src/pool.rs
use alloy::primitives::{Address, U256};
use alloy::providers::RootProvider;
use alloy::transports::http::{Client, Http};
use eyre::Result;
use std::sync::Arc;
use crate::provider::get_pool_reserves;
use crate::math::get_amount_out;

/// Структура для представления пула ликвидности
#[derive(Debug, Clone)]
pub struct Pool {
    pub pool_address: Address,
    pub provider: Arc<RootProvider<Http<Client>>>,
    pub token0_address: Address,
    pub token1_address: Address,
    pub reserve_token0: U256,
    pub reserve_token1: U256,
    pub name: String,
}

impl Pool {
    /// Создает новый экземпляр пула с нулевыми резервами
    /// Для получения актуальных резервов используйте refresh_reserves()
    /// 
    /// # Arguments
    /// * `pool_address` - Адрес контракта пула
    /// * `token0_address` - Адрес первого токена (должен быть < token1_address)
    /// * `token1_address` - Адрес второго токена
    /// * `provider` - Провайдер для подключения к блокчейну
    /// * `name` - Имя пула для идентификации (например, "Uniswap V2 USDC/WETH")
    /// 
    /// # Returns
    /// Новый экземпляр Pool с нулевыми резервами
    pub fn new(
        pool_address: Address,
        token0_address: Address,
        token1_address: Address,
        provider: Arc<RootProvider<Http<Client>>>,
        name: String,
    ) -> Self {
        // Убеждаемся, что token0 < token1 (стандарт Uniswap V2)
        let (token0, token1) = if token0_address < token1_address {
            (token0_address, token1_address)
        } else {
            (token1_address, token0_address)
        };
        
        Pool {
            pool_address,
            provider,
            token0_address: token0,
            token1_address: token1,
            reserve_token0: U256::ZERO,
            reserve_token1: U256::ZERO,
            name,
        }
    }
    
    /// Создает Pool и сразу получает актуальные резервы из блокчейна
    /// 
    /// # Arguments
    /// * `pool_address` - Адрес контракта пула
    /// * `token0_address` - Адрес первого токена
    /// * `token1_address` - Адрес второго токена
    /// * `provider` - Провайдер для подключения к блокчейну
    /// * `name` - Имя пула для идентификации
    /// 
    /// # Returns
    /// Pool с актуальными резервами или ошибка
    pub async fn with_reserves(
        pool_address: Address,
        token0_address: Address,
        token1_address: Address,
        provider: Arc<RootProvider<Http<Client>>>,
        name: String,
    ) -> Result<Self> {
        let mut pool = Self::new(pool_address, token0_address, token1_address, provider, name);
        pool.refresh_reserves().await?;
        Ok(pool)
    }
    
    /// Вычисляет количество выходных токенов для заданного количества входных токенов
    /// Использует формулу Uniswap V2 constant product
    /// 
    /// # Arguments
    /// * `amount_in` - Количество входных токенов
    /// * `input_is_token0` - true если входной токен это token0, false если token1
    /// 
    /// # Returns
    /// Количество выходных токенов
    pub fn get_amount_out(&self, amount_in: U256, input_is_token0: bool) -> U256 {
        if input_is_token0 {
            // Обмениваем token0 на token1
            get_amount_out(amount_in, self.reserve_token0, self.reserve_token1)
        } else {
            // Обмениваем token1 на token0
            get_amount_out(amount_in, self.reserve_token1, self.reserve_token0)
        }
    }
    
    /// Симулирует свап и обновляет резервы без обращения к блокчейну
    /// 
    /// # Arguments
    /// * `amount_in` - Количество входных токенов
    /// * `input_is_token0` - true если входной токен это token0, false если token1
    /// 
    /// # Returns
    /// Количество выходных токенов
    pub fn mock_swap(&mut self, amount_in: U256, input_is_token0: bool) -> U256 {
        let amount_out = self.get_amount_out(amount_in, input_is_token0);
        
        if input_is_token0 {
            // Обмениваем token0 на token1
            // Увеличиваем резерв token0, уменьшаем резерв token1
            self.reserve_token0 += amount_in;
            self.reserve_token1 -= amount_out;
        } else {
            // Обмениваем token1 на token0
            // Увеличиваем резерв token1, уменьшаем резерв token0
            self.reserve_token1 += amount_in;
            self.reserve_token0 -= amount_out;
        }
        
        amount_out
    }
    
    /// Обновляет резервы пула из блокчейна
    pub async fn refresh_reserves(&mut self) -> Result<()> {
        let (reserve0, reserve1) = get_pool_reserves(
            self.provider.clone(), 
            self.pool_address
        ).await?;
        
        self.reserve_token0 = reserve0;
        self.reserve_token1 = reserve1;
        
        Ok(())
    }
}
