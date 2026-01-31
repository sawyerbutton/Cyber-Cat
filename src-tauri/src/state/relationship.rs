use serde::{Deserialize, Serialize};

/// 第三层：关系状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipState {
    /// 信任度 0-100: 稳定陪伴、及时响应需求增加；突然消失、强行打扰下降
    pub trust: f32,
    /// 亲密度 0-100: 高质量互动增加；长期忽视下降
    pub intimacy: f32,
    /// 了解度 0-100: 对话、分享信息增加；不下降
    pub understanding: f32,
}

impl RelationshipState {
    pub fn new() -> Self {
        Self {
            trust: 10.0,
            intimacy: 5.0,
            understanding: 0.0,
        }
    }

    /// 良好互动后增加关系值
    pub fn on_positive_interaction(&mut self) {
        self.trust = (self.trust + 0.5).min(100.0);
        self.intimacy = (self.intimacy + 0.8).min(100.0);
    }

    /// 对话后增加了解度
    pub fn on_conversation(&mut self) {
        self.understanding = (self.understanding + 1.0).min(100.0);
        self.intimacy = (self.intimacy + 0.3).min(100.0);
    }

    /// 长期忽视导致关系下降
    pub fn on_neglect(&mut self) {
        self.trust = (self.trust - 0.1).max(0.0);
        self.intimacy = (self.intimacy - 0.2).max(0.0);
    }

    /// 是否愿意主动靠近（信任度 > 30）
    pub fn will_approach(&self) -> bool {
        self.trust > 30.0
    }

    /// 是否会慢眨眼（信任度 > 50）
    pub fn will_slow_blink(&self) -> bool {
        self.trust > 50.0
    }

    /// 是否会露肚皮（信任度 > 70）
    pub fn will_show_belly(&self) -> bool {
        self.trust > 70.0
    }
}
