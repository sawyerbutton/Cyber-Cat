mod state;
mod behavior;
mod memory;
mod llm;

use std::sync::Mutex;
use std::time::Duration;
use std::path::PathBuf;

use serde::Serialize;
use tauri::{
    Emitter, Manager, State,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};

use state::SophieState;
use behavior::{Behavior, decide_behavior};
use memory::MemoryStore;
use llm::LlmClient;

// ── 共享状态 ─────────────────────────────────────────────────

struct AppState {
    sophie: Mutex<SophieState>,
    memory: Mutex<MemoryStore>,
    llm: LlmClient,
    tokio_rt: tokio::runtime::Runtime,
}

// ── 前端事件数据 ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SophieSnapshot {
    energy: f32,
    hunger: f32,
    sleepiness: f32,
    emotion: String,
    trust: f32,
    intimacy: f32,
    understanding: f32,
    is_sleeping: bool,
    behavior: Behavior,
    flip_direction: bool,
    minutes_since_interaction: u32,
}

/// 想法气泡事件
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ThoughtEvent {
    text: String,
}

/// 用户言语响应事件
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SpeechResponseEvent {
    action: String,
    thought: Option<String>,
    behavior: Behavior,
}

fn make_snapshot(sophie: &SophieState) -> SophieSnapshot {
    let hour = chrono_hour();
    let beh = decide_behavior(sophie, hour);
    let flip = beh.may_change_direction() && (chrono_nanos() % 2 == 0);
    SophieSnapshot {
        energy: sophie.physiological.energy,
        hunger: sophie.physiological.hunger,
        sleepiness: sophie.physiological.sleepiness,
        emotion: format!("{:?}", sophie.emotion),
        trust: sophie.relationship.trust,
        intimacy: sophie.relationship.intimacy,
        understanding: sophie.relationship.understanding,
        is_sleeping: sophie.is_sleeping,
        behavior: beh,
        flip_direction: flip,
        minutes_since_interaction: sophie.minutes_since_interaction(),
    }
}

// ── Tauri Commands ──────────────────────────────────────────

#[tauri::command]
fn get_sophie_state(app_state: State<AppState>) -> SophieSnapshot {
    let sophie = app_state.sophie.lock().unwrap();
    make_snapshot(&sophie)
}

#[tauri::command]
fn click_sophie(app_state: State<AppState>) -> SophieSnapshot {
    let mut sophie = app_state.sophie.lock().unwrap();
    sophie.record_interaction();
    sophie.relationship.on_positive_interaction();

    if let Ok(mem) = app_state.memory.lock() {
        let _ = mem.add("interaction", "主人点了我", 0.3);
    }

    make_snapshot(&sophie)
}

#[tauri::command]
fn feed_sophie(app_state: State<AppState>) -> SophieSnapshot {
    let mut sophie = app_state.sophie.lock().unwrap();
    sophie.record_interaction();
    sophie.physiological.feed();
    sophie.relationship.on_positive_interaction();

    if let Ok(mem) = app_state.memory.lock() {
        let _ = mem.add("interaction", "主人给我喂食了", 0.6);
    }

    make_snapshot(&sophie)
}

/// 用户"说给 Sophie 听" — 使用 LLM 生成响应
#[tauri::command]
fn speak_to_sophie(app_state: State<AppState>, app_handle: tauri::AppHandle, message: String) -> SophieSnapshot {
    let mut sophie = app_state.sophie.lock().unwrap();
    sophie.record_interaction();
    sophie.relationship.on_conversation();

    // 保存记忆
    let recent_memories = if let Ok(mem) = app_state.memory.lock() {
        let _ = mem.add("user_speech", &format!("主人说：{}", message), 0.7);
        mem.recent_as_text(5)
    } else {
        vec![]
    };

    // 异步调用 LLM
    let emotion_str = format!("{:?}", sophie.emotion);
    let intimacy = sophie.relationship.intimacy;
    let trust = sophie.relationship.trust;
    let behavior_str = format!("{:?}", decide_behavior(&sophie, chrono_hour()));
    let snapshot = make_snapshot(&sophie);
    drop(sophie); // 释放锁

    let llm = &app_state.llm;
    let messages = llm::build_speech_response_prompt(
        &message,
        &emotion_str,
        intimacy,
        trust,
        &behavior_str,
        &recent_memories,
    );

    let handle = app_handle.clone();
    let llm_client = LlmClient::new(llm.api_key().to_string());

    app_state.tokio_rt.spawn(async move {
        match llm_client.chat(messages, 200, 0.9).await {
            Ok(text) => {
                log::info!("LLM speech response: {}", text);
                let result = llm::parse_speech_response(&text);

                // 映射 LLM action 到 Behavior
                let behavior = match result.action.as_str() {
                    "ignore" => Behavior::Idle,
                    "glance" | "alert" => Behavior::Alert,
                    "approach" | "walk" => Behavior::Walk,
                    "walk_away" | "run" => Behavior::Run,
                    "sit" => Behavior::Sit,
                    "sleep" => Behavior::Sleep,
                    _ => Behavior::Idle,
                };

                let event = SpeechResponseEvent {
                    action: result.action,
                    thought: result.thought.clone(),
                    behavior,
                };
                let _ = handle.emit("sophie-speech-response", &event);

                // 如果有想法，单独发送想法事件
                if let Some(thought) = result.thought {
                    if !thought.is_empty() && thought != "null" {
                        let _ = handle.emit("sophie-thought", &ThoughtEvent { text: thought });
                    }
                }
            }
            Err(e) => {
                log::error!("LLM speech error: {}", e);
            }
        }
    });

    snapshot
}

// ── Helpers ─────────────────────────────────────────────────

fn chrono_hour() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    ((secs / 3600 + 8) % 24) as u32
}

fn chrono_nanos() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos()
}

fn db_path() -> PathBuf {
    let mut path = dirs_for_db();
    std::fs::create_dir_all(&path).ok();
    path.push("sophie.db");
    path
}

fn dirs_for_db() -> PathBuf {
    if let Some(data_dir) = dirs::data_local_dir() {
        data_dir.join("cyber-cat")
    } else {
        PathBuf::from(".")
    }
}

// ── App Entry ───────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 加载 .env
    dotenvy::dotenv().ok();

    let api_key = std::env::var("MINIMAX_API_KEY")
        .unwrap_or_default();

    if api_key.is_empty() {
        log::warn!("MINIMAX_API_KEY not set, LLM features will be disabled");
    }

    // 打开记忆数据库
    let memory_store = MemoryStore::open(&db_path())
        .expect("Failed to open memory database");

    // 尝试恢复 Sophie 状态
    let sophie = if let Some(state_json) = memory_store.load_state("sophie") {
        serde_json::from_str::<SophieState>(&state_json).unwrap_or_else(|_| SophieState::new())
    } else {
        SophieState::new()
    };

    let tokio_rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

    let llm_client = LlmClient::new(api_key.clone());

    tauri::Builder::default()
        .manage(AppState {
            sophie: Mutex::new(sophie),
            memory: Mutex::new(memory_store),
            llm: llm_client,
            tokio_rt,
        })
        .invoke_handler(tauri::generate_handler![
            get_sophie_state,
            click_sophie,
            feed_sophie,
            speak_to_sophie,
        ])
        .setup(move |app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // ── 系统托盘 ──
            let show = MenuItem::with_id(app, "show", "显示 Sophie", true, None::<&str>)?;
            let hide = MenuItem::with_id(app, "hide", "隐藏 Sophie", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &hide, &quit])?;

            TrayIconBuilder::new()
                .tooltip("Cyber Cat - Sophie")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("sophie") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "hide" => {
                        if let Some(w) = app.get_webview_window("sophie") {
                            let _ = w.hide();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            // ── 后台生命循环 + AI 思考 ──
            let handle = app.handle().clone();
            let api_key_clone = api_key.clone();

            std::thread::spawn(move || {
                let mut tick_counter: u64 = 0;
                let rt = tokio::runtime::Runtime::new().unwrap();

                loop {
                    std::thread::sleep(Duration::from_secs(10));
                    tick_counter += 1;

                    let state_ref = handle.state::<AppState>();

                    // ── 每 30 秒 tick 生理/情绪/关系 ──
                    if tick_counter % 3 == 0 {
                        let mut sophie = state_ref.sophie.lock().unwrap();
                        sophie.tick();

                        // 持久化状态
                        if tick_counter % 6 == 0 {
                            if let Ok(json) = serde_json::to_string(&*sophie) {
                                if let Ok(mem) = state_ref.memory.lock() {
                                    let _ = mem.save_state("sophie", &json);
                                }
                            }
                        }
                    }

                    // ── 每轮广播行为快照 ──
                    {
                        let sophie = state_ref.sophie.lock().unwrap();
                        let snapshot = make_snapshot(&sophie);
                        drop(sophie);
                        let _ = handle.emit("sophie-update", &snapshot);
                    }

                    // ── AI 自主思考：每 180 轮（~30 分钟） ──
                    if !api_key_clone.is_empty() && tick_counter % 180 == 0 {
                        let sophie = state_ref.sophie.lock().unwrap();
                        let recent = if let Ok(mem) = state_ref.memory.lock() {
                            mem.recent_as_text(5)
                        } else {
                            vec![]
                        };

                        let messages = llm::build_thinking_prompt(
                            sophie.physiological.energy,
                            sophie.physiological.hunger,
                            sophie.physiological.sleepiness,
                            &format!("{:?}", sophie.emotion),
                            sophie.relationship.intimacy,
                            sophie.relationship.trust,
                            sophie.minutes_since_interaction(),
                            chrono_hour(),
                            &recent,
                        );
                        drop(sophie);

                        let handle2 = handle.clone();
                        let key = api_key_clone.clone();

                        rt.spawn(async move {
                            let client = LlmClient::new(key);
                            match client.chat(messages, 300, 0.9).await {
                                Ok(text) => {
                                    log::info!("Sophie thinking: {}", text);
                                    let result = llm::parse_thinking_response(&text);

                                    // 记录思考
                                    if let Some(state_ref) = handle2.try_state::<AppState>() {
                                        if let Ok(mem) = state_ref.memory.lock() {
                                            let _ = mem.add("thought", &result.thinking, 0.5);
                                        }
                                    }

                                    // 显示想法气泡
                                    if let Some(thought) = result.show_thought {
                                        if !thought.is_empty() && thought != "null" {
                                            let _ = handle2.emit("sophie-thought", &ThoughtEvent { text: thought });
                                        }
                                    }
                                }
                                Err(e) => {
                                    log::error!("Thinking error: {}", e);
                                }
                            }
                        });
                    }

                    // ── 随机想法气泡（无 LLM，基于规则）：每 60-90 秒 ──
                    if tick_counter % 7 == 0 {
                        let sophie = state_ref.sophie.lock().unwrap();
                        let thought = rule_based_thought(&sophie);
                        drop(sophie);

                        if let Some(text) = thought {
                            let _ = handle.emit("sophie-thought", &ThoughtEvent { text });
                        }
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// 基于规则的想法生成（不依赖 LLM，保底方案）
fn rule_based_thought(sophie: &SophieState) -> Option<String> {
    use state::emotion::Emotion;

    if sophie.physiological.hunger > 80.0 {
        return Some("饿...".to_string());
    }
    if sophie.physiological.sleepiness > 75.0 && !sophie.is_sleeping {
        return Some("困...".to_string());
    }
    if sophie.is_sleeping {
        if chrono_nanos() % 5 == 0 {
            return Some("zzz".to_string());
        }
        return None;
    }

    let r = chrono_nanos() % 100;
    match sophie.emotion {
        Emotion::Happy => {
            if r < 20 { Some("嗯~".into()) }
            else if r < 35 { Some("舒服".into()) }
            else { None }
        }
        Emotion::Bored => {
            if r < 25 { Some("无聊".into()) }
            else if r < 40 { Some("...".into()) }
            else { None }
        }
        Emotion::Irritated => {
            if r < 20 { Some("烦".into()) }
            else { None }
        }
        Emotion::Down => {
            if r < 15 { Some("...".into()) }
            else { None }
        }
        Emotion::Curious => {
            if r < 30 { Some("嗯？".into()) }
            else { None }
        }
        Emotion::Playful => {
            if r < 25 { Some("来玩！".into()) }
            else { None }
        }
        Emotion::Calm => {
            if r < 10 { Some("嗯。".into()) }
            else if r < 15 { Some("暖和".into()) }
            else { None }
        }
    }
}
