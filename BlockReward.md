# QoraNet Per-Block Reward Distribution

## Block Reward Pool Distribution

**Every block produces rewards that are split among ALL eligible participants:**

```
Total Block Reward = 10 QOR + Transaction Fees
```

## Distribution Process Per Block

### Step 1: Calculate Total Reward Pool
```
Block_Reward_Pool = {
  LP_Pool: 40% of total block reward
  Performance_Pool: 50% of total block reward  
  Network_Fund: 10% of total block reward
}
```

**Rationale for 50% Performance Pool:**
- Users provide **real computational resources** (servers, bandwidth, storage)
- Apps strengthen network infrastructure and utility  
- Higher technical barrier than just providing liquidity
- Ongoing operational costs (electricity, hardware maintenance)
- Critical for network growth and adoption

### Step 2: Distribute to ALL Eligible Users

**Every block, EVERY eligible user receives rewards based on their share:**

## LP Pool Distribution (60% of block reward)

```rust
// Pseudo-code for LP reward distribution
for each_user in active_lp_providers {
    user_lp_share = user_lp_value / total_network_lp_value
    user_lp_reward = LP_Pool * user_lp_share
    
    // Add to user's pending rewards
    pending_rewards[user_address] += user_lp_reward
}
```

**Example with 3 users:**
- Block Reward: 10 QOR
- LP Pool: 4 QOR (40%)
- Total Network LP: $50,000

| User | LP Value | LP Share | LP Reward |
|------|----------|----------|-----------|
| Alice | $20,000 | 40% | 1.6 QOR |
| Bob | $15,000 | 30% | 1.2 QOR |
| Carol | $15,000 | 30% | 1.2 QOR |

## Performance Pool Distribution (30% of block reward)

```rust
// Performance rewards based on actual work done
for each_user in active_app_hosters {
    // Calculate performance score
    performance_score = calculate_performance(user)
    user_performance_share = performance_score / total_network_performance
    user_performance_reward = Performance_Pool * user_performance_share
    
    pending_rewards[user_address] += user_performance_reward
}
```

**Example Performance Distribution:**
- Performance Pool: 5 QOR (50% of block)
- Total Network Performance Score: 300

| User | Performance Score | Share | Performance Reward |
|------|------------------|-------|-------------------|
| Alice | 120 | 40% | 2.0 QOR |
| Bob | 90 | 30% | 1.5 QOR |
| Carol | 90 | 30% | 1.5 QOR |

## Complete Block Distribution Example

**Block #12345 Rewards:**
- Total Block Reward: 12 QOR (10 base + 2 fees)
- LP Pool: 4.8 QOR (40%)
- Performance Pool: 6.0 QOR (50%)
- Network Fund: 1.2 QOR (10%)

| User | LP Reward | Performance Reward | Total Per Block |
|------|-----------|-------------------|-----------------|
| Alice | 1.92 QOR | 2.4 QOR | **4.32 QOR** |
| Bob | 1.44 QOR | 1.8 QOR | **3.24 QOR** |
| Carol | 1.44 QOR | 1.8 QOR | **3.24 QOR** |

## Reward Accumulation System

### Pending Rewards Contract
```rust
struct UserRewards {
    address: PublicKey,
    pending_lp_rewards: u64,
    pending_performance_rewards: u64,
    pending_bonus_rewards: u64,
    last_claim_block: u64,
    vesting_schedule: Vec<VestingEntry>,
}
```

### Block Processing Flow
```
1. Block Produced
   ↓
2. Calculate Total Reward Pool
   ↓
3. Update ALL User Pending Rewards
   ↓
4. Apply Vesting Rules
   ↓
5. Users Can Claim Available Rewards
```

## Eligibility Requirements

### For LP Rewards (Every Block):
- **Minimum LP**: $100 USD equivalent
- **Active Status**: LP tokens locked and verified
- **No Slashing**: Not currently penalized

### For Performance Rewards (Every Block):
- **Node Online**: Must be actively running
- **Minimum Uptime**: 95% in last 24 hours
- **Running Apps**: At least 1 approved network application
- **Reporting Metrics**: Successfully submitting performance data

## Real-Time Distribution Algorithm

```rust
impl BlockRewardDistributor {
    pub fn distribute_block_rewards(&self, block: &Block) -> Result<()> {
        let total_reward = self.calculate_block_reward(block);
        
        // Split reward pools
        let lp_pool = total_reward * 0.40;
        let performance_pool = total_reward * 0.50;
        let network_fund = total_reward * 0.10;
        
        // Get all eligible participants
        let lp_providers = self.get_active_lp_providers()?;
        let app_hosters = self.get_active_app_hosters()?;
        
        // Distribute LP rewards
        for user in lp_providers {
            let share = self.calculate_lp_share(&user)?;
            let reward = lp_pool * share;
            self.add_pending_reward(&user.address, reward, RewardType::LP)?;
        }
        
        // Distribute performance rewards  
        for user in app_hosters {
            let score = self.calculate_performance_score(&user)?;
            let share = score / self.total_network_performance;
            let reward = performance_pool * share;
            self.add_pending_reward(&user.address, reward, RewardType::Performance)?;
        }
        
        // Send to network fund
        self.add_to_network_fund(network_fund)?;
        
        Ok(())
    }
}
```

## Claim Mechanism

### User Claims Rewards
```rust
pub fn claim_rewards(user: &PublicKey) -> Result<u64> {
    let user_rewards = get_user_rewards(user)?;
    let claimable = calculate_claimable_rewards(&user_rewards)?;
    
    if claimable > 0 {
        transfer_qor(user, claimable)?;
        update_user_rewards(user, claimable)?;
    }
    
    Ok(claimable)
}
```

## Gas/Fee Considerations

### Who Pays for Distribution?
- **Network Fund covers distribution costs**
- **Users only pay gas when claiming**
- **Automatic distribution funded by protocol**

### Claiming Fees
- **Claim Transaction**: $0.00015 USD (standard fee)
- **Batch Claims**: Users can accumulate rewards to save on fees
- **Auto-claim**: Optional feature with higher fee ($0.0005 USD)

## Scaling Considerations

### For Large User Base (10,000+ users):
- **Merkle Tree Rewards**: Bundle rewards in Merkle trees
- **Batch Processing**: Process rewards in batches per block
- **State Compression**: Compress reward state to reduce storage
- **Lazy Claiming**: Only update state when users claim

### Example with 10,000 Users:
```
Block Processing Time: ~50ms
Reward Calculation: ~30ms  
State Updates: ~20ms
Total Per Block: ~100ms (well within block time)
```

## Economic Example: Daily Rewards

**Network Stats:**
- 1,000 active users
- 8,640 blocks per day (10-second blocks)
- 86,400 QOR distributed daily

**Average User Daily Rewards:**
- LP Provider (average): ~50 QOR/day
- App Hoster (average): ~35 QOR/day  
- Both LP + Apps: ~86 QOR/day

**Annual Rewards:**
- Dedicated user: ~31,390 QOR/year
- At $2.50/QOR: ~$78,475 annual rewards

This creates strong incentives for long-term participation!
