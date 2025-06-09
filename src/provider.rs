// src/provider.rs
use alloy::primitives::{Address, U256};
use alloy::providers::{Provider, ProviderBuilder, RootProvider};
use alloy::sol;
use alloy::transports::http::{Client, Http};
use eyre::Result;
use std::sync::Arc;
use crate::config::{usdc_to_decimal, weth_to_decimal};

// Определяем ABI для функции getReserves контракта Uniswap V2 Pair
sol! {
    #[sol(rpc)]
    interface IUniswapV2Pair {
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
    }
}

// Определяем ABI для Factory контракта
sol! {
    #[sol(rpc)]
    interface IUniswapV2Factory {
        function getPair(address tokenA, address tokenB) external view returns (address pair);
    }
}

/// Создает провайдер для подключения к сети Polygon через Infura
pub async fn create_provider(rpc_url: &str) -> Result<Arc<RootProvider<Http<Client>>>> {
    let provider = ProviderBuilder::new()
        .on_http(rpc_url.parse()?);
    Ok(Arc::new(provider))
}

/// Получает резервы (reserve0, reserve1) из пула ликвидности
/// Возвращает raw значения в наименьших единицах (без учета decimals)
/// 
/// # Arguments
/// * `provider` - Провайдер для подключения к блокчейну
/// * `pool_address` - Адрес контракта пула
/// 
/// # Returns
/// Кортеж с резервами (reserve0, reserve1) в формате U256 в raw units
pub async fn get_pool_reserves(
    provider: Arc<RootProvider<Http<Client>>>,
    pool_address: Address,
) -> Result<(U256, U256)> {
    // Создаем экземпляр контракта по адресу
    let contract = IUniswapV2Pair::IUniswapV2PairInstance::new(pool_address, provider);
    
    // Вызываем функцию getReserves
    println!("Отправляем запрос к контракту по адресу: {:?}", pool_address);
    let reserves = contract.getReserves().call().await?;
    
    // Конвертируем uint112 в U256 для большей совместимости
    let reserve0 = U256::from(reserves.reserve0);
    let reserve1 = U256::from(reserves.reserve1);
    
    println!("Получены резервы: reserve0={}, reserve1={}", reserve0, reserve1);
    
    Ok((reserve0, reserve1))
}

/// Определяет порядок токенов в пуле (token0 < token1 по адресу)
/// и возвращает резервы в правильном порядке для USDC/WETH пары
/// 
/// # Arguments
/// * `provider` - Провайдер для подключения к блокчейну
/// * `pool_address` - Адрес контракта пула
/// * `usdc_address` - Адрес токена USDC
/// * `weth_address` - Адрес токена WETH
/// 
/// # Returns
/// Кортеж (usdc_reserve_raw, weth_reserve_raw) в правильном порядке в raw units
pub async fn get_usdc_weth_reserves(
    provider: Arc<RootProvider<Http<Client>>>,
    pool_address: Address,
    usdc_address: Address,
    weth_address: Address,
) -> Result<(U256, U256)> {
    let (reserve0, reserve1) = get_pool_reserves(provider, pool_address).await?;
    
    // В Uniswap V2 token0 < token1 по лексикографическому порядку адресов
    let (usdc_reserve_raw, weth_reserve_raw) = if usdc_address < weth_address {
        // USDC = token0, WETH = token1
        (reserve0, reserve1)
    } else {
        // WETH = token0, USDC = token1
        (reserve1, reserve0)
    };
    
    // Логируем человекочитаемые значения для проверки
    let usdc_decimal = usdc_to_decimal(usdc_reserve_raw);
    let weth_decimal = weth_to_decimal(weth_reserve_raw);
    println!("Резервы пула (decimal): USDC={:.2}, WETH={:.6}", usdc_decimal, weth_decimal);
    
    Ok((usdc_reserve_raw, weth_reserve_raw))
}

/// Создает Pool объект через Factory контракт
/// 
/// # Arguments
/// * `provider` - Провайдер для подключения к блокчейну
/// * `factory_address` - Адрес Factory контракта
/// * `token_a` - Адрес первого токена
/// * `token_b` - Адрес второго токена
/// 
/// # Returns
/// Pool объект или None если пул не существует
pub async fn create_pool_from_factory(
    provider: Arc<RootProvider<Http<Client>>>,
    factory_address: Address,
    token_a: Address,
    token_b: Address,
) -> Result<Option<crate::pool::Pool>> {
    use crate::pool::Pool;
    
    // Создаем экземпляр Factory контракта
    let factory = IUniswapV2Factory::IUniswapV2FactoryInstance::new(factory_address, provider.clone());
    
    println!("Запрашиваем пул через Factory: {:?}", factory_address);
    println!("  Токены: {:?} / {:?}", token_a, token_b);
    
    // Вызываем функцию getPair
    let pair_address = factory.getPair(token_a, token_b).call().await?;
    
    // Проверяем, что адрес не нулевой (пул существует)
    if pair_address.pair == Address::ZERO {
        println!("  Пул не найден");
        Ok(None)
    } else {
        println!("  Найден адрес пула: {:?}", pair_address.pair);
        
        // Определяем имя DEX на основе Factory адреса
        let dex_name = match factory_address {
            addr if addr == crate::config::QUICKSWAP_V2_FACTORY => "Quickswap",
            addr if addr == crate::config::SUSHISWAP_V2_FACTORY => {
                if token_a == crate::config::USDC_ADDRESS || token_b == crate::config::USDC_ADDRESS {
                    "Sushiswap"
                } else {
                    "Sushiswap USDC.e"
                }
            },
            _ => "Unknown DEX"
        };
        
        let pool_name = format!("{} USDC/WETH", dex_name);
        
        // Создаем Pool объект с резервами
        match Pool::with_reserves(
            pair_address.pair,
            token_a,
            token_b,
            provider,
            pool_name,
        ).await {
            Ok(pool) => {
                println!("  Pool объект создан успешно");
                Ok(Some(pool))
            }
            Err(e) => {
                println!("  Ошибка создания Pool объекта: {}", e);
                Err(e)
            }
        }
    }
}

/// Получает все пулы USDC/WETH через Factory контракты
/// 
/// # Arguments
/// * `provider` - Провайдер для подключения к блокчейну
/// * `usdc_address` - Адрес токена USDC
/// * `weth_address` - Адрес токена WETH
/// 
/// # Returns
/// Вектор найденных пулов Pool со всеми данными
pub async fn get_all_pool_addresses(
    provider: Arc<RootProvider<Http<Client>>>,
    usdc_address: Address,
    weth_address: Address,
) -> Result<Vec<crate::pool::Pool>> {
    use crate::config::{QUICKSWAP_V2_FACTORY, SUSHISWAP_V2_FACTORY, UNISWAP_V2_POOL_ADDRESS};
    
    let mut pools = Vec::new();
    
    // Создаем статический пул Uniswap V2
    match crate::pool::Pool::with_reserves(
        UNISWAP_V2_POOL_ADDRESS,
        usdc_address,
        weth_address,
        provider.clone(),
        "Uniswap V2 USDC/WETH".to_string(),
    ).await {
        Ok(uniswap_pool) => {
            println!("Uniswap V2 Pool создан (статический адрес)");
            pools.push(uniswap_pool);
        }
        Err(e) => {
            println!("Ошибка создания Uniswap V2 Pool: {}", e);
        }
    }
    
    // Проверяем Quickswap
    match create_pool_from_factory(
        provider.clone(),
        QUICKSWAP_V2_FACTORY,
        usdc_address,
        weth_address,
    ).await {
        Ok(Some(quickswap_pool)) => {
            println!("Quickswap Pool получен через Factory");
            pools.push(quickswap_pool);
        }
        Ok(None) => {
            println!("Quickswap: пул USDC/WETH не найден");
        }
        Err(e) => {
            println!("Ошибка получения Quickswap Pool: {}", e);
        }
    }
    
    // Проверяем Sushiswap
    match create_pool_from_factory(
        provider.clone(),
        SUSHISWAP_V2_FACTORY,
        usdc_address,
        weth_address,
    ).await {
        Ok(Some(sushiswap_pool)) => {
            println!("Sushiswap Pool получен через Factory");
            pools.push(sushiswap_pool);
        }
        Ok(None) => {
            println!("Sushiswap: пул USDC/WETH не найден");
        }
        Err(e) => {
            println!("Ошибка получения Sushiswap Pool: {}", e);
        }
    }
    
    // Также проверяем с USDC.e для Sushiswap
    let usdc_e_address = Address::from([
        0x27, 0x91, 0xBc, 0xa1, 0xf2, 0xde, 0x46, 0x61,
        0xED, 0x88, 0xA3, 0x0C, 0x99, 0xA7, 0xa9, 0x44,
        0x9A, 0xa8, 0x41, 0x74
    ]);
    match create_pool_from_factory(
        provider.clone(),
        SUSHISWAP_V2_FACTORY,
        usdc_e_address,
        weth_address,
    ).await {
        Ok(Some(sushiswap_usdc_e_pool)) => {
            println!("Sushiswap USDC.e Pool получен через Factory");
            pools.push(sushiswap_usdc_e_pool);
        }
        Ok(None) => {
            println!("Sushiswap: пул USDC.e/WETH не найден");
        }
        Err(e) => {
            println!("Ошибка получения Sushiswap USDC.e Pool: {}", e);
        }
    }
    
    println!("Создано {} Pool объектов через Factory контракты", pools.len());
    
    Ok(pools)
}
