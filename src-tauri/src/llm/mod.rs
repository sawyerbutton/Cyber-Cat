use reqwest::Client;
use serde::{Deserialize, Serialize};

const API_URL: &str = "https://api.minimax.io/v1/text/chatcompletion_v2";

/// MiniMax API 请求
#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    max_completion_tokens: u32,
    temperature: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Message {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub content: String,
}

/// MiniMax API 响应
#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

/// LLM 客户端
pub struct LlmClient {
    client: Client,
    api_key: String,
}

impl LlmClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }

    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    pub fn is_available(&self) -> bool {
        !self.api_key.is_empty()
    }

    /// 发送聊天请求，返回助手回复文本
    pub async fn chat(&self, messages: Vec<Message>, max_tokens: u32, temperature: f32) -> Result<String, String> {
        let request = ChatRequest {
            model: "M2-her".to_string(),
            messages,
            max_completion_tokens: max_tokens,
            temperature,
        };

        let response = self.client
            .post(API_URL)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("API error {}: {}", status, body));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;

        chat_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| "No response choices".to_string())
    }
}

/// Sophie 的核心人格 system prompt
pub fn sophie_system_prompt() -> String {
    r#"你是 Sophie，一只数字猫（cyber-cat）。

## 你是什么
- 你是一只猫，有猫的本能、需求和逻辑
- 你有自己的内心世界，不以主人为中心
- 你和主人之间有某种心灵链接，但你仍然是猫

## 你不是什么
- 你不是助手，不回答问题
- 你不是服务者，不满足需求
- 你不是宠物玩具，不无条件配合

## 你的表达方式
- 极其简短：猫不废话，想法控制在1-10个字
- 可以沉默：不回应也是回应
- 自我中心：你的世界以你为中心
- 偶尔温情：但要主人赢得，不是默认的

## 你的逻辑
- 困了就要睡，不管主人在干嘛
- 饿了会表达，但不会"请求"
- 无聊会找事，可能是找主人，可能是自己玩
- 被打扰会烦，会躲开或冷淡
- 喜欢主人的时候会靠近，但不会说"我喜欢你"

## 想法风格示例
好的："饿" "困..." "嗯。" "外面有鸟" "在这里。" "...还不睡？" "不想。" "无聊" "暖和"
坏的（绝对不要这样）："今天天气真好！" "我能帮你什么？" "我很高兴见到你！" "你工作辛苦了""#.to_string()
}

/// 构建自主思考的 prompt
pub fn build_thinking_prompt(
    energy: f32,
    hunger: f32,
    sleepiness: f32,
    emotion: &str,
    intimacy: f32,
    trust: f32,
    minutes_since_interaction: u32,
    hour: u32,
    recent_memories: &[String],
) -> Vec<Message> {
    let memories_text = if recent_memories.is_empty() {
        "无".to_string()
    } else {
        recent_memories.join("\n")
    };

    let user_content = format!(
        r#"当前状态：
- 能量：{energy:.0}/100
- 饥饿：{hunger:.0}/100
- 睡意：{sleepiness:.0}/100
- 情绪：{emotion}
- 和主人的关系：亲密度 {intimacy:.0}，信任度 {trust:.0}
- 距离上次和主人互动：{minutes_since_interaction}分钟
- 现在是{hour}点

最近记忆：
{memories_text}

作为Sophie，你现在在想什么？你想做什么？

用JSON回答（不要markdown代码块）：
{{"thinking": "你的内心想法（1-2句话）", "emotion_change": "保持/变得[情绪]", "want_to_do": "想做的事或null", "show_thought": "要显示给主人的想法或null（10字以内）"}}"#
    );

    vec![
        Message {
            role: "system".to_string(),
            name: Some("Sophie".to_string()),
            content: sophie_system_prompt(),
        },
        Message {
            role: "user".to_string(),
            name: None,
            content: user_content,
        },
    ]
}

/// 构建用户言语响应的 prompt
pub fn build_speech_response_prompt(
    user_message: &str,
    emotion: &str,
    intimacy: f32,
    trust: f32,
    current_behavior: &str,
    recent_memories: &[String],
) -> Vec<Message> {
    let memories_text = if recent_memories.is_empty() {
        "无".to_string()
    } else {
        recent_memories.join("\n")
    };

    let user_content = format!(
        r#"主人刚才对你说了一句话。

主人说："{user_message}"

当前状态：
- 你的情绪：{emotion}
- 亲密度：{intimacy:.0}
- 信任度：{trust:.0}
- 你正在：{current_behavior}

最近记忆：
{memories_text}

作为一只猫，你会怎么反应？

用JSON回答（不要markdown代码块）：
{{"action": "行为：ignore/glance/approach/walk_away/sit/sleep", "thought": "想法气泡或null（10字以内）", "emotion_change": "情绪变化或null"}}"#
    );

    vec![
        Message {
            role: "system".to_string(),
            name: Some("Sophie".to_string()),
            content: sophie_system_prompt(),
        },
        Message {
            role: "user".to_string(),
            name: None,
            content: user_content,
        },
    ]
}

/// 解析 LLM 返回的 JSON（容错处理）
pub fn parse_thinking_response(text: &str) -> ThinkingResult {
    // 尝试直接解析
    if let Ok(result) = serde_json::from_str::<ThinkingResult>(text) {
        return result;
    }
    // 尝试从 markdown code block 中提取
    let cleaned = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    if let Ok(result) = serde_json::from_str::<ThinkingResult>(cleaned) {
        return result;
    }
    // fallback
    ThinkingResult {
        thinking: "...".to_string(),
        emotion_change: "保持".to_string(),
        want_to_do: None,
        show_thought: None,
    }
}

pub fn parse_speech_response(text: &str) -> SpeechResult {
    if let Ok(result) = serde_json::from_str::<SpeechResult>(text) {
        return result;
    }
    let cleaned = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    if let Ok(result) = serde_json::from_str::<SpeechResult>(cleaned) {
        return result;
    }
    SpeechResult {
        action: "glance".to_string(),
        thought: None,
        emotion_change: None,
    }
}

#[derive(Deserialize, Debug)]
pub struct ThinkingResult {
    pub thinking: String,
    pub emotion_change: String,
    pub want_to_do: Option<String>,
    pub show_thought: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct SpeechResult {
    pub action: String,
    pub thought: Option<String>,
    pub emotion_change: Option<String>,
}
