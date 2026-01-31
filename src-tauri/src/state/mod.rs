pub mod physiological;
pub mod emotion;
pub mod relationship;

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Sophie 的完整状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SophieState {
    pub physiological: physiological::PhysiologicalState,
    pub emotion: emotion::Emotion,
    pub relationship: relationship::RelationshipState,
    /// 是否正在睡觉
    pub is_sleeping: bool,
    /// 上次互动的 Unix 时间戳（秒）
    pub last_interaction_ts: u64,
    /// 短期内的互动次数（用于判断是否过度打扰）
    pub recent_interaction_count: u32,
    /// 上次重置互动计数的时间戳
    pub interaction_count_reset_ts: u64,
}

impl SophieState {
    pub fn new() -> Self {
        let now = unix_now();
        Self {
            physiological: physiological::PhysiologicalState::new(),
            emotion: emotion::Emotion::Calm,
            relationship: relationship::RelationshipState::new(),
            is_sleeping: false,
            last_interaction_ts: now,
            recent_interaction_count: 0,
            interaction_count_reset_ts: now,
        }
    }

    /// 每分钟调用一次，更新所有状态
    pub fn tick(&mut self) {
        let now = unix_now();
        let minutes_since_interaction = ((now - self.last_interaction_ts) / 60) as u32;

        // 每 10 分钟重置短期互动计数
        if now - self.interaction_count_reset_ts > 600 {
            self.recent_interaction_count = 0;
            self.interaction_count_reset_ts = now;
        }

        // 1. 更新生理状态
        self.physiological.tick(self.is_sleeping);

        // 2. 自动入睡/醒来
        if !self.is_sleeping && self.physiological.sleepiness > 80.0 {
            self.is_sleeping = true;
        }
        if self.is_sleeping && self.physiological.sleepiness < 5.0 {
            self.is_sleeping = false;
        }

        // 3. 情绪转移
        let has_interaction = minutes_since_interaction < 2;
        self.emotion = self.emotion.transition(
            has_interaction,
            minutes_since_interaction,
            self.physiological.energy,
            self.relationship.intimacy,
        );

        // 4. 关系衰减（长期忽视）
        if minutes_since_interaction > 180 {
            self.relationship.on_neglect();
        }
    }

    /// 记录一次互动
    pub fn record_interaction(&mut self) {
        self.last_interaction_ts = unix_now();
        self.recent_interaction_count += 1;

        // 如果在睡觉被打扰
        if self.is_sleeping {
            // 短时间内打扰太多次会变烦躁
            if self.recent_interaction_count > 3 {
                self.emotion = emotion::Emotion::Irritated;
            }
            // 但还是可能醒来
            if self.recent_interaction_count > 1 {
                self.is_sleeping = false;
            }
        }
    }

    /// 距离上次互动的分钟数
    pub fn minutes_since_interaction(&self) -> u32 {
        let now = unix_now();
        ((now - self.last_interaction_ts) / 60) as u32
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
