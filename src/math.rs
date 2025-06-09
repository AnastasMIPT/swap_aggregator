use alloy::primitives::U256;

/// Calculates the output amount based on the Uniswap V2 formula.
/// 
/// Formula: amountOut = (amountIn * 997 * reserveOut) / (reserveIn * 1000 + amountIn * 997)
/// 
/// The 997/1000 factor accounts for the 0.3% trading fee (1000 - 3 = 997).
/// 
/// # Arguments
/// * `amount_in` - Amount of input tokens
/// * `reserve_in` - Reserve of input tokens in the pool
/// * `reserve_out` - Reserve of output tokens in the pool
/// 
/// # Returns
/// Amount of output tokens that will be received
pub fn get_amount_out(
    amount_in: U256,
    reserve_in: U256,
    reserve_out: U256,
) -> U256 {
    // Проверка на нулевые резервы - если один из резервов равен нулю,
    // то пул неликвиден и обмен невозможен
    if reserve_in == U256::ZERO || reserve_out == U256::ZERO {
        return U256::ZERO;
    }

    // amountIn * 997 (учет комиссии 0.3%)
    let amount_in_with_fee = amount_in * U256::from(997);
    
    // Числитель: amountIn * 997 * reserveOut
    let numerator = amount_in_with_fee * reserve_out;
    
    // Знаменатель: reserveIn * 1000 + amountIn * 997
    let denominator = (reserve_in * U256::from(1000)) + amount_in_with_fee;
    
    // Финальный расчет: numerator / denominator
    numerator / denominator
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_amount_out_basic() {
        // Тест с базовыми значениями
        let amount_in = U256::from(1000u64);
        let reserve_in = U256::from(100000u64);
        let reserve_out = U256::from(100000u64);
        
        let result = get_amount_out(amount_in, reserve_in, reserve_out);
        
        // При равных резервах и небольшом amount_in результат должен быть близок к amount_in * 0.997
        // 1000 * 997 * 100000 / (100000 * 1000 + 1000 * 997) = 99700000 / 100997000 ≈ 987
        let expected = U256::from(987u64);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_get_amount_out_zero_reserves() {
        let amount_in = U256::from(1000u64);
        
        // Тест с нулевыми резервами
        assert_eq!(get_amount_out(amount_in, U256::ZERO, U256::from(100000u64)), U256::ZERO);
        assert_eq!(get_amount_out(amount_in, U256::from(100000u64), U256::ZERO), U256::ZERO);
        assert_eq!(get_amount_out(amount_in, U256::ZERO, U256::ZERO), U256::ZERO);
    }

    #[test]
    fn test_get_amount_out_zero_input() {
        let reserve_in = U256::from(100000u64);
        let reserve_out = U256::from(100000u64);
        
        // При нулевом входе результат должен быть нулевым
        let result = get_amount_out(U256::ZERO, reserve_in, reserve_out);
        assert_eq!(result, U256::ZERO);
    }

    #[test]
    fn test_get_amount_out_large_numbers() {
        // Тест с большими числами (имитация реальных резервов пула)
        let amount_in = U256::from(1_000_000u64); // 1M USDC
        let reserve_in = U256::from(10_000_000_000u64); // 10B USDC
        let reserve_out = U256::from(5_000_000_000u64); // 5B WETH
        
        let result = get_amount_out(amount_in, reserve_in, reserve_out);
        
        // Результат должен быть больше нуля и меньше reserve_out
        assert!(result > U256::ZERO);
        assert!(result < reserve_out);
        
        // Примерный расчет: 1M * 997 * 5B / (10B * 1000 + 1M * 997) 
        // ≈ 4.985e+18 / 1.0009e+13 ≈ 498,551
        let expected_range = U256::from(498_000u64)..U256::from(499_000u64);
        assert!(result >= expected_range.start && result < expected_range.end);
    }
} 