use serde::{Deserialize, Serialize};
use crate::state::SophieState;
use crate::state::emotion::Emotion;

/// Sophie 的行为——直接映射到前端动画状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Behavior {
    /// 待机（站立微动） → 前端 "idle"
    Idle,
    /// 睡觉 → 前端 "sleep"
    Sleep,
    /// 行走 → 前端 "walk"
    Walk,
    /// 警觉/注意 → 前端 "alert"
    Alert,
    /// 坐下 → 前端 "sit"
    Sit,
    /// 奔跑/疯跑 → 前端 "run"
    Run,
}

impl Behavior {
    /// 该行为是否需要翻转方向（随机朝左或朝右）
    pub fn may_change_direction(&self) -> bool {
        matches!(self, Behavior::Walk | Behavior::Run)
    }
}

/// 根据当前状态决策下一个行为
pub fn decide_behavior(state: &SophieState, hour: u32) -> Behavior {
    // 睡眠状态优先
    if state.is_sleeping {
        return Behavior::Sleep;
    }

    let phys = &state.physiological;

    // 生理需求：困了要睡
    if phys.sleepiness > 70.0 {
        return Behavior::Sleep;
    }
    // 能量太低：坐下休息
    if phys.energy < 20.0 {
        return Behavior::Sit;
    }
    // 饿了：走来走去引起注意
    if phys.hunger > 85.0 {
        return Behavior::Walk;
    }

    // 情绪驱动
    match state.emotion {
        Emotion::Bored => {
            let r = rand_f32();
            if r < 0.3 {
                Behavior::Walk // 蹭屏幕 → 走来走去
            } else if r < 0.5 {
                Behavior::Run // 疯跑
            } else {
                Behavior::Alert // 盯着用户看
            }
        }
        Emotion::Happy => {
            let r = rand_f32();
            if state.relationship.intimacy > 50.0 && r < 0.3 {
                Behavior::Walk // 靠近用户
            } else if r < 0.5 {
                Behavior::Idle // 放松待机
            } else {
                Behavior::Sit // 舒服地坐着
            }
        }
        Emotion::Irritated => Behavior::Sit, // 躲到角落坐着不理人
        Emotion::Down => Behavior::Sleep,     // 蜷缩休息
        Emotion::Curious => Behavior::Alert,  // 警觉观察
        Emotion::Playful => {
            if rand_f32() < 0.5 {
                Behavior::Run // 疯跑
            } else {
                Behavior::Walk
            }
        }
        Emotion::Calm => {
            // 晨昏活跃
            if (5..8).contains(&hour) || (17..20).contains(&hour) {
                if phys.energy > 60.0 && rand_f32() < 0.4 {
                    return Behavior::Walk;
                }
            }
            // 默认日常行为
            random_daily_behavior()
        }
    }
}

fn random_daily_behavior() -> Behavior {
    let r = rand_f32();
    if r < 0.35 {
        Behavior::Idle
    } else if r < 0.55 {
        Behavior::Sit
    } else if r < 0.7 {
        Behavior::Walk
    } else if r < 0.85 {
        Behavior::Alert
    } else {
        Behavior::Idle
    }
}

/// 简单伪随机
fn rand_f32() -> f32 {
    use std::time::SystemTime;
    let t = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    // 加入更多变化
    let mixed = t.wrapping_mul(1103515245).wrapping_add(12345);
    (mixed % 10000) as f32 / 10000.0
}
