// src/solver.rs
use crate::{config, math, provider};
use alloy::primitives::{Address, U256};
use eyre::Result;
use std::sync::Arc;
use tokio::task::JoinSet;
use alloy::providers::RootProvider;
use alloy::transports::http::{Client, Http};

#[derive(Debug)]
pub struct ChunkRoute {
    pub chunk_index: u64,
    pub best_pool_name: String,
    pub amount_in: U256,     // В raw units (USDC с 6 decimals)
    pub amount_out: U256,    // В raw units (WETH с 18 decimals)
    pub amount_in_decimal: f64,   // Человекочитаемое значение USDC
    pub amount_out_decimal: f64,  // Человекочитаемое значение WETH
}

#[derive(Debug)]
pub struct SolverResult {
    pub total_weth_out: U256,        // Общий выход в raw units
    pub total_weth_out_decimal: f64, // Общий выход в человекочитаемом виде
    pub chunk_routes: Vec<ChunkRoute>,
}

pub async fn find_best_routes(
    provider: Arc<RootProvider<Http<Client>>>,
    mut pools: Vec<crate::pool::Pool>
) -> Result<SolverResult> {
    let mut chunk_routes = Vec::with_capacity(config::NUM_CHUNKS as usize);
    let mut total_weth_out = U256::ZERO;

    println!("Начинаем поиск лучших маршрутов для {} чанков", config::NUM_CHUNKS);
    let chunk_amount_raw = config::get_chunk_usdc_amount();
    println!("Размер чанка: {} USDC (raw: {})", 
        config::CHUNK_USDC_DECIMAL, 
        chunk_amount_raw);

    for i in 0..config::NUM_CHUNKS {
        let mut best_output = U256::ZERO;
        let mut best_pool_name = String::new();
        let mut best_pool_index = 0;
        let mut best_input_is_token0 = false;

        println!("\nОбрабатываем чанк #{}", i + 1);

        // Проверяем каждый пул для текущего чанка
        for (pool_index, pool) in pools.iter().enumerate() {
            // Проверяем, какой тип USDC содержит пул
            let usdc_is_token0 = pool.token0_address == config::USDC_ADDRESS;
            let usdc_is_token1 = pool.token1_address == config::USDC_ADDRESS;
            let usdc_e_is_token0 = pool.token0_address == config::USDC_E_ADDRESS;
            let usdc_e_is_token1 = pool.token1_address == config::USDC_E_ADDRESS;
            
            // Определяем, содержит ли пул любой тип USDC
            let has_usdc = usdc_is_token0 || usdc_is_token1;
            let has_usdc_e = usdc_e_is_token0 || usdc_e_is_token1;
            
            // Пропускаем пулы, которые не содержат ни USDC, ни USDC.e
            if !has_usdc && !has_usdc_e {
                println!("Пул {:?}: {} -> Пропущен (не содержит USDC или USDC.e)", 
                    pool.pool_address, pool.name);
                continue;
            }
            
            // Определяем, является ли входной токен token0
            let input_is_token0 = usdc_is_token0 || usdc_e_is_token0;
            
            // Рассчитываем output без обновления резервов для сравнения пулов
            let output = pool.get_amount_out(chunk_amount_raw, input_is_token0);
            
            // Определяем тип токена для вывода
            let token_type = if has_usdc { "USDC" } else { "USDC.e" };

            println!("Пул {:?}: {} -> WETH выход = {:.6} (raw: {}) [входной токен: {}]", 
                pool.pool_address,
                pool.name,
                config::weth_to_decimal(output), 
                output,
                token_type);

            if output > best_output {
                best_output = output;
                best_pool_name = pool.name.clone();
                best_pool_index = pool_index;
                best_input_is_token0 = input_is_token0;
            }
        }
        
        // Применяем реальный swap только к лучшему пулу (обновляем резервы)
        if best_output > U256::ZERO {
            let actual_output = pools[best_pool_index].mock_swap(chunk_amount_raw, best_input_is_token0);
            println!("Применен mock_swap к пулу {}: обновлены резервы, фактический выход = {:.6} WETH", 
                best_pool_name, config::weth_to_decimal(actual_output));
            
            // Используем фактический выход вместо расчетного (должны совпадать, но проверяем)
            if actual_output != best_output {
                println!("Предупреждение: расчетный выход ({}) != фактический выход ({})", 
                    best_output, actual_output);
            }
            best_output = actual_output;
        }
        
        total_weth_out += best_output;
        
        // Создаем запись маршрута с человекочитаемыми значениями
        chunk_routes.push(ChunkRoute {
            chunk_index: i + 1,
            best_pool_name: best_pool_name.clone(),
            amount_in: chunk_amount_raw,
            amount_out: best_output,
            amount_in_decimal: config::CHUNK_USDC_DECIMAL,
            amount_out_decimal: config::weth_to_decimal(best_output),
        });

        println!("Лучший пул для чанка #{}: {} -> {:.6} WETH", 
            i + 1, best_pool_name, config::weth_to_decimal(best_output));
    }

    let total_weth_decimal = config::weth_to_decimal(total_weth_out);
    println!("\nИтого WETH получено: {:.6} (raw: {})", total_weth_decimal, total_weth_out);

    Ok(SolverResult { 
        total_weth_out, 
        total_weth_out_decimal: total_weth_decimal,
        chunk_routes 
    })
} 