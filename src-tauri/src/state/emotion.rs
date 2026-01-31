use serde::{Deserialize, Serialize};

/// 第二层：情绪状态
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Emotion {
    /// 开心 - 被陪伴、投喂、玩耍后
    Happy,
    /// 平静 - 默认状态
    Calm,
    /// 好奇 - 发现有趣的东西
    Curious,
    /// 想玩 - 能量高、无聊
    Playful,
    /// 无聊 - 长时间没互动
    Bored,
    /// 烦躁 - 被打扰太多
    Irritated,
    /// 低落 - 被长期忽视
    Down,
}

impl Emotion {
    /// 根据上下文判断情绪转移
    pub fn transition(
        &self,
        has_interaction: bool,
        minutes_since_interaction: u32,
        energy: f32,
        intimacy: f32,
    ) -> Emotion {
        match self {
            Emotion::Calm => {
                if has_interaction && energy > 50.0 {
                    Emotion::Happy
                } else if minutes_since_interaction > 120 {
                    Emotion::Bored
                } else {
                    Emotion::Calm
                }
            }
            Emotion::Happy => {
                if minutes_since_interaction > 60 {
                    Emotion::Calm
                } else {
                    Emotion::Happy
                }
            }
            Emotion::Bored => {
                if has_interaction {
                    Emotion::Happy
                } else if minutes_since_interaction > 240 {
                    if intimacy > 40.0 {
                        Emotion::Irritated
                    } else {
                        Emotion::Down
                    }
                } else {
                    Emotion::Bored
                }
            }
            Emotion::Irritated => {
                if minutes_since_interaction > 30 && !has_interaction {
                    Emotion::Calm
                } else {
                    Emotion::Irritated
                }
            }
            Emotion::Down => {
                if has_interaction && intimacy > 30.0 {
                    Emotion::Calm
                } else {
                    Emotion::Down
                }
            }
            Emotion::Curious => {
                if minutes_since_interaction > 10 {
                    Emotion::Calm
                } else {
                    Emotion::Curious
                }
            }
            Emotion::Playful => {
                if energy < 40.0 {
                    Emotion::Calm
                } else if minutes_since_interaction > 30 {
                    Emotion::Bored
                } else {
                    Emotion::Playful
                }
            }
        }
    }
}
