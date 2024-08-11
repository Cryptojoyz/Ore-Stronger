use std::{sync::Arc, sync::RwLock, time::Instant};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash as StdHash, Hasher};

use colored::*;
use drillx::{
    equix::{self},
    Hash, Solution,
};
use ore_api::{
    consts::{BUS_ADDRESSES, BUS_COUNT, EPOCH_DURATION},
    state::{Bus, Config, Proof},
};
use ore_utils::AccountDeserialize;
use rand::Rng;
use solana_program::pubkey::Pubkey;
use solana_rpc_client::spinner;
use solana_sdk::signer::Signer;

use crate::{
    args::MineArgs,
    send_and_confirm::ComputeBudget,
    utils::{
        amount_u64_to_string, get_clock, get_config, get_updated_proof_with_authority, proof_pubkey,
    },
    Miner,
};

fn hash_combine<T: StdHash>(seed: u64, value: T) -> u64 {
    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    value.hash(&mut hasher);
    hasher.finish()
}

impl Miner {
    pub async fn mine(&self, args: MineArgs) {
        // Open account, if needed.
        let signer = self.signer();
        self.open().await;

        // Check num threads
        self.check_num_cores(args.cores);

        let mut best_solution = None;
        let mut highest_difficulty = 0;
        let mut interval = 100;
        let mut start_time = Instant::now(); // 初始化 start_time
        // Start mining loop
        let mut last_hash_at = 0;
        let mut last_balance = 0;
        loop {
            // Fetch proof
            let config = get_config(&self.rpc_client).await;
            let proof =
                get_updated_proof_with_authority(&self.rpc_client, signer.pubkey(), last_hash_at)
                    .await;
            println!(
                "\n\nStake: {} ORE\n{}  Multiplier: {:12}x",
                amount_u64_to_string(proof.balance),
                if last_hash_at.gt(&0) {
                    format!(
                        "  Change: {} ORE\n",
                        amount_u64_to_string(proof.balance.saturating_sub(last_balance))
                    )
                } else {
                    "".to_string()
                },
                calculate_multiplier(proof.balance, config.top_balance)
            );
            last_hash_at = proof.last_hash_at;
            last_balance = proof.balance;

            loop {
                // Calculate cutoff time
            let cutoff_time = self.get_cutoff(proof, args.buffer_time).await;

            // Run drillx
            let (solution, difficulty) =
                Self::find_hash_par(proof, cutoff_time, args.cores, config.min_difficulty as u32, interval)
                    .await;
            
            // 更新最高难度和最佳 solution
            if difficulty >= highest_difficulty {
                    highest_difficulty = difficulty;
                    best_solution = Some(solution.clone());
            }
            
            // 检查 best_solution 是否有值，然后打印
            if let Some(ref best_solution) = best_solution {
                    let hashshow = best_solution.to_hash();
                    // println!("当前最佳难度: {}", highest_difficulty);
                    println!("  当前最好Hash: {} (难度 {})", bs58::encode(hashshow.h).into_string(), highest_difficulty);
            }

            if difficulty >= 18 {
                highest_difficulty = difficulty;
                if let Some(valid_solution) = best_solution.clone(){
                // Build instruction set
                let mut ixs = vec![ore_api::instruction::auth(proof_pubkey(signer.pubkey()))];
                let mut compute_budget = 500_000;

                if self.should_reset(config).await && rand::thread_rng().gen_range(0..100).eq(&0) {
                    compute_budget += 100_000;
                    ixs.push(ore_api::instruction::reset(signer.pubkey()));
                }
                // Build mine ix
                ixs.push(ore_api::instruction::mine(
                    signer.pubkey(),
                    signer.pubkey(),
                    self.find_bus().await,
                    valid_solution,
                ));
                println!("  难度大于等于18，已提交解决方案。方案的Hash: {} (难度 {})", bs58::encode((valid_solution.to_hash()).h).into_string(), highest_difficulty);
                // Submit transaction
                let result = self.send_and_confirm(&ixs, ComputeBudget::Fixed(compute_budget), false, Some(highest_difficulty))
                    .await
                    .ok();
                if let Some(_) = result {
                    // 如果交易成功，执行操作
                    start_time = Instant::now(); // 重置 start_time
                    interval = 100;
                }
                highest_difficulty = 0; // 重置最高难度
                best_solution = None; // 清空最佳 solution
                break; // 满足条件后退出内部循环
                }
            } 
            else {
                    if start_time.elapsed().as_secs() >= 70 {
                        if let Some(valid_solution) = best_solution.clone() {
                            // Build instruction set
                        let mut ixs = vec![ore_api::instruction::auth(proof_pubkey(signer.pubkey()))];
                        let mut compute_budget = 500_000;

                        if self.should_reset(config).await && rand::thread_rng().gen_range(0..100).eq(&0) {
                            compute_budget += 100_000;
                            ixs.push(ore_api::instruction::reset(signer.pubkey()));
                        }
                        // Build mine ix
                        ixs.push(ore_api::instruction::mine(
                            signer.pubkey(),
                            signer.pubkey(),
                            self.find_bus().await,
                            valid_solution,
                        ));
                        println!("  时间已超过70秒，提交解决方案。方案的Hash: {} (难度 {})", bs58::encode((valid_solution.to_hash()).h).into_string(), highest_difficulty);
                        // Submit transaction
                        let result = self.send_and_confirm(&ixs, ComputeBudget::Fixed(compute_budget), false, Some(highest_difficulty))
                            .await
                            .ok();
                        if let Some(_) = result {
                            // 如果交易成功，执行操作
                            start_time = Instant::now(); // 重置 start_time
                            interval = 100;
                        }
                        highest_difficulty = 0; // 重置最高难度
                        best_solution = None; // 清空最佳 solution
                        break; // 满足条件后退出内部循环

                            }
                    }
                    else {
                        interval = 5000;
                        println!(
                            "难度小于18, 间隔已重置为{}, 搏一搏，万一呢？",interval
                        );
                    }
                        
                }
            }
        }
    }

    async fn find_hash_par(
        proof: Proof,
        cutoff_time: u64,
        cores: u64,
        min_difficulty: u32,
        interval: u64,
    ) -> (Solution, u32) {
        // Dispatch job to each thread
        let progress_bar = Arc::new(spinner::new_progress_bar());
        let global_best_difficulty = Arc::new(RwLock::new(0u32));
        progress_bar.set_message("Mining...");
        let core_ids = core_affinity::get_core_ids().unwrap();
        let handles: Vec<_> = core_ids
            .into_iter()
            .map(|i| {
                let global_best_difficulty = Arc::clone(&global_best_difficulty);
                std::thread::spawn({
                    let proof = proof.clone();
                    let progress_bar = progress_bar.clone();
                    let mut memory = equix::SolverMemory::new();
                    let mut rng = rand::thread_rng();
                    // 使用随机数作为变化因子
                    let random_factor: u64 = rng.gen_range(0..u64::MAX);
                    move || {
                        // Return if core should not be used
                        if (i.id as u64).ge(&cores) {
                            return (0, 0, Hash::default());
                        }

                        // Pin to core
                        let _ = core_affinity::set_for_current(i);

                        // Start hashing
                        let timer = Instant::now();
                        // let mut nonce = u64::MAX.saturating_div(cores).saturating_mul(i.id as u64);
                        // let mut best_nonce = nonce;
                        let increment = u64::MAX.saturating_div(cores);
                        // 使用哈希函数结合 random_factor 和 increment
                        let mut nonce = hash_combine(random_factor, increment);
                        let mut best_nonce = nonce.saturating_mul(i.id as u64);
                        let mut best_difficulty = 0;
                        let mut best_hash = Hash::default();
                        loop {
                            // Create hash
                            if let Ok(hx) = drillx::hash_with_memory(
                                &mut memory,
                                &proof.challenge,
                                &nonce.to_le_bytes(),
                            ) {
                                let difficulty = hx.difficulty();
                                if difficulty.gt(&best_difficulty) {
                                    best_nonce = nonce;
                                    best_difficulty = difficulty;
                                    best_hash = hx;
                                    // {{ edit_1 }}
                                    if best_difficulty.gt(&*global_best_difficulty.read().unwrap())
                                    {
                                        *global_best_difficulty.write().unwrap() = best_difficulty;
                                    }
                                    // {{ edit_1 }}
                                }
                            }

                            // Exit if time has elapsed
                            if nonce % interval == 0 {
                                let global_best_difficulty =
                                    *global_best_difficulty.read().unwrap();
                                if timer.elapsed().as_secs().ge(&cutoff_time) {
                                    if i.id == 0 {
                                        progress_bar.set_message(format!(
                                            "Mining... (difficulty {})",
                                            global_best_difficulty,
                                        ));
                                    }
                                    if global_best_difficulty.ge(&min_difficulty) {
                                        // Mine until min difficulty has been met
                                        break;
                                    }
                                } else if i.id == 0 {
                                    progress_bar.set_message(format!(
                                        "Mining... (difficulty {}, time {})",
                                        global_best_difficulty,
                                        format_duration(
                                            cutoff_time.saturating_sub(timer.elapsed().as_secs())
                                                as u32
                                        ),
                                    ));
                                }
                            }

                            // Increment nonce
                            nonce += 1;
                        }

                        // Return the best nonce
                        (best_nonce, best_difficulty, best_hash)
                    }
                })
            })
            .collect();

        // Join handles and return best nonce
        let mut best_nonce = 0;
        let mut best_difficulty = 0;
        let mut best_hash = Hash::default();
        for h in handles {
            if let Ok((nonce, difficulty, hash)) = h.join() {
                if difficulty > best_difficulty {
                    best_difficulty = difficulty;
                    best_nonce = nonce;
                    best_hash = hash;
                }
            }
        }

        // Update log
        progress_bar.finish_with_message(format!(
            "Best hash: {} (difficulty {})",
            bs58::encode(best_hash.h).into_string(),
            best_difficulty
        ));

        (Solution::new(best_hash.d, best_nonce.to_le_bytes()), best_difficulty)
    }

    pub fn check_num_cores(&self, cores: u64) {
        let num_cores = num_cpus::get() as u64;
        if cores.gt(&num_cores) {
            println!(
                "{} Cannot exceeds available cores ({})",
                "WARNING".bold().yellow(),
                num_cores
            );
        }
    }

    async fn should_reset(&self, config: Config) -> bool {
        let clock = get_clock(&self.rpc_client).await;
        config
            .last_reset_at
            .saturating_add(EPOCH_DURATION)
            .saturating_sub(5) // Buffer
            .le(&clock.unix_timestamp)
    }

    async fn get_cutoff(&self, proof: Proof, buffer_time: u64) -> u64 {
        let clock = get_clock(&self.rpc_client).await;
        proof
            .last_hash_at
            .saturating_add(60)
            .saturating_sub(buffer_time as i64)
            .saturating_sub(clock.unix_timestamp)
            .max(0) as u64
    }

    async fn find_bus(&self) -> Pubkey {
        // Fetch the bus with the largest balance
        if let Ok(accounts) = self.rpc_client.get_multiple_accounts(&BUS_ADDRESSES).await {
            let mut top_bus_balance: u64 = 0;
            let mut top_bus = BUS_ADDRESSES[0];
            for account in accounts {
                if let Some(account) = account {
                    if let Ok(bus) = Bus::try_from_bytes(&account.data) {
                        if bus.rewards.gt(&top_bus_balance) {
                            top_bus_balance = bus.rewards;
                            top_bus = BUS_ADDRESSES[bus.id as usize];
                        }
                    }
                }
            }
            return top_bus;
        }

        // Otherwise return a random bus
        let i = rand::thread_rng().gen_range(0..BUS_COUNT);
        BUS_ADDRESSES[i]
    }
}

fn calculate_multiplier(balance: u64, top_balance: u64) -> f64 {
    1.0 + (balance as f64 / top_balance as f64).min(1.0f64)
}

fn format_duration(seconds: u32) -> String {
    let minutes = seconds / 60;
    let remaining_seconds = seconds % 60;
    format!("{:02}:{:02}", minutes, remaining_seconds)
}
