use serde::{Deserialize, Serialize};

/// 第一层：生理状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysiologicalState {
    /// 能量 0-100，活动时消耗，睡眠时恢复
    pub energy: f32,
    /// 饥饿 0-100，随时间增加
    pub hunger: f32,
    /// 睡意 0-100，清醒时增加，睡觉时归零
    pub sleepiness: f32,
}

impl PhysiologicalState {
    pub fn new() -> Self {
        Self {
            energy: 80.0,
            hunger: 20.0,
            sleepiness: 10.0,
        }
    }

    /// 每分钟更新一次生理状态
    pub fn tick(&mut self, is_sleeping: bool) {
        if is_sleeping {
            self.energy = (self.energy + 2.0).min(100.0);
            self.sleepiness = (self.sleepiness - 3.0).max(0.0);
        } else {
            self.energy = (self.energy - 0.5).max(0.0);
            self.sleepiness = (self.sleepiness + 0.2).min(100.0);
        }
        self.hunger = (self.hunger + 0.3).min(100.0);
    }

    /// 喂食
    pub fn feed(&mut self) {
        self.hunger = (self.hunger - 30.0).max(0.0);
    }

    /// 是否需要休息
    pub fn needs_rest(&self) -> bool {
        self.energy < 30.0 || self.sleepiness > 80.0
    }

    /// 是否饿了
    pub fn is_hungry(&self) -> bool {
        self.hunger > 70.0
    }
}
