mod config;
mod math;
mod pool;
mod provider;
mod solver;

use std::env;
use config::{USDC_ADDRESS, WETH_ADDRESS, TOTAL_USDC_DECIMAL, weth_to_decimal};
use provider::{create_provider, get_all_pool_addresses};
use solver::find_best_routes;
use eyre::Result;



#[tokio::main]
async fn main() -> Result<()> {
    println!("Добро пожаловать в Swap Aggregator для USDC/WETH на Polygon!");
    
    // Загружаем переменные окружения из .env файла
    dotenv::dotenv().ok();
    
    // Получаем RPC URL из переменной окружения
    let rpc_url = env::var("INFURA_POLYGON_URL")
        .unwrap_or_else(|_| "https://polygon-mainnet.infura.io/v3/YOUR_PROJECT_ID".to_string());
    
    println!("Подключаемся к сети Polygon через RPC: {}", rpc_url);
    
    // Создаем провайдер
    let provider = create_provider(&rpc_url).await
        .expect("Не удалось создать провайдер для подключения к Polygon");
    println!("Провайдер создан успешно");
    

    
    // Получаем Pool объекты через Factory контракты
    println!("\n=== Получение Pool объектов через Factory контракты ===");
    let pools = get_all_pool_addresses(provider.clone(), USDC_ADDRESS, WETH_ADDRESS).await?;
    
    if pools.is_empty() {
        println!("\nНе найдено ни одного пула через Factory контракты!");
        println!("Возможные причины:");
        println!("  - Factory контракты не содержат пулы USDC/WETH");
        println!("  - Неправильные адреса Factory контрактов"); 
        println!("  - Проблемы с подключением к сети");
        return Ok(());
    }
    
    println!("✓ Найдено {} Pool объектов через Factory контракты", pools.len());
    for pool in &pools {
        println!("  Pool: {} - {:?} (tokens: {:?}/{:?})", 
            pool.name, pool.pool_address, pool.token0_address, pool.token1_address);
    }

    // Запускаем полный анализ свапа
    println!("\n=== Запуск полного анализа свапа ===");
    let result = find_best_routes(provider.clone(), pools).await?;
    
    let total_weth_decimal = weth_to_decimal(result.total_weth_out);
   
    println!("Solver завершил работу успешно!");
    println!("Результаты:");
    println!("  Обработано частей: {}", result.chunk_routes.len());
    println!("  Общий выход WETH: {:.6} WETH (raw: {})", total_weth_decimal, result.total_weth_out);
    println!("  Входная сумма USDC: {} USDC", TOTAL_USDC_DECIMAL);
    
    // Показываем первые 5 результатов
    println!("\nПервые 5 результатов:");
    for (i, route) in result.chunk_routes.iter().take(5).enumerate() {
        println!("  {}. Часть {}: {} -> {:.6} WETH", 
            i + 1, route.chunk_index, route.best_pool_name, route.amount_out_decimal);
    }
    
    // Подсчитываем и показываем статистику использования пулов
    let mut pool_usage = std::collections::HashMap::new();
    for route in &result.chunk_routes {
        *pool_usage.entry(route.best_pool_name.clone()).or_insert(0) += 1;
    }
    
    println!("\nСтатистика использования пулов:");
    for (pool_name, count) in pool_usage {
        let percentage = (count as f64 / result.chunk_routes.len() as f64) * 100.0;
        println!("  {}: {} раз ({:.1}%)", pool_name, count, percentage);
    }
    
    Ok(())
}
