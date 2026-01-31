<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import Sophie from "./components/Sophie.vue";
import ThoughtBubble from "./components/ThoughtBubble.vue";

interface SophieState {
  energy: number;
  hunger: number;
  sleepiness: number;
  emotion: string;
  trust: number;
  intimacy: number;
  understanding: number;
  isSleeping: boolean;
  behavior: string;
  flipDirection: boolean;
  minutesSinceInteraction: number;
}

interface ThoughtEvent {
  text: string;
}

interface SpeechResponseEvent {
  action: string;
  thought: string | null;
  behavior: string;
}

const currentThought = ref<string | null>(null);
const showSpeakInput = ref(false);
const speakText = ref("");
const showDebug = ref(false);
const sophieState = ref<SophieState | null>(null);

let unlistenThought: UnlistenFn | undefined;
let unlistenSpeechResponse: UnlistenFn | undefined;

// ── 想法气泡 ──
function showThought(text: string) {
  currentThought.value = text;
  setTimeout(() => {
    currentThought.value = null;
  }, 4000);
}

// ── 状态更新回调（从 Sophie 组件接收） ──
function onStateUpdate(state: SophieState) {
  sophieState.value = state;
}

// ── "说给 Sophie 听" ──
async function onSpeak() {
  if (!speakText.value.trim()) return;
  try {
    const state = await invoke<SophieState>("speak_to_sophie", {
      message: speakText.value,
    });
    onStateUpdate(state);
    // LLM 响应会通过 sophie-speech-response 和 sophie-thought 事件异步到达
  } catch (_) {
    // fallback
  }
  speakText.value = "";
  showSpeakInput.value = false;
}

// ── 喂食 ──
async function onFeed() {
  try {
    const state = await invoke<SophieState>("feed_sophie");
    onStateUpdate(state);
  } catch (_) {
    // fallback
  }
}

// ── 右键 ──
function onContextMenu(e: MouseEvent) {
  e.preventDefault();
  showSpeakInput.value = !showSpeakInput.value;
}

// ── 监听后端事件 ──
onMounted(async () => {
  try {
    // 监听想法气泡（来自 AI 思考循环 或 规则生成）
    unlistenThought = await listen<ThoughtEvent>("sophie-thought", (event) => {
      showThought(event.payload.text);
    });

    // 监听用户言语的 LLM 响应
    unlistenSpeechResponse = await listen<SpeechResponseEvent>("sophie-speech-response", (event) => {
      const { thought } = event.payload;
      if (thought && thought !== "null") {
        showThought(thought);
      }
    });
  } catch (_) {
    // 非 Tauri 环境
  }
});

onUnmounted(() => {
  unlistenThought?.();
  unlistenSpeechResponse?.();
});
</script>

<template>
  <div class="cyber-cat-container" data-tauri-drag-region @contextmenu="onContextMenu">
    <!-- Sophie 本体 -->
    <Sophie @state-update="onStateUpdate" />

    <!-- 想法气泡 -->
    <ThoughtBubble :text="currentThought" />

    <!-- "说给 Sophie 听" 输入框 -->
    <Transition name="slide">
      <div v-if="showSpeakInput" class="speak-panel" @click.stop>
        <div class="speak-hint">说给 Sophie 听...</div>
        <div class="speak-input-row">
          <input
            v-model="speakText"
            class="speak-input"
            placeholder="她可能不会回应"
            @keydown.enter="onSpeak"
            @keydown.escape="showSpeakInput = false"
            autofocus
          />
          <button class="speak-btn" @click="onSpeak">说</button>
        </div>
      </div>
    </Transition>

    <!-- 快捷操作 -->
    <div class="actions">
      <button class="action-btn" title="喂食" @click="onFeed">🐟</button>
      <button class="action-btn" title="说话" @click="showSpeakInput = !showSpeakInput">💬</button>
      <button class="action-btn" title="状态" @click="showDebug = !showDebug">📊</button>
    </div>

    <!-- 调试面板 -->
    <Transition name="fade">
      <div v-if="showDebug && sophieState" class="debug-panel">
        <div class="debug-row">
          <span>⚡{{ sophieState.energy.toFixed(0) }}</span>
          <span>🍖{{ sophieState.hunger.toFixed(0) }}</span>
          <span>💤{{ sophieState.sleepiness.toFixed(0) }}</span>
        </div>
        <div class="debug-row">
          <span>😺{{ sophieState.emotion }}</span>
          <span>{{ sophieState.behavior }}</span>
        </div>
        <div class="debug-row">
          <span>💛{{ sophieState.trust.toFixed(1) }}</span>
          <span>💕{{ sophieState.intimacy.toFixed(1) }}</span>
          <span>🧠{{ sophieState.understanding.toFixed(1) }}</span>
        </div>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.cyber-cat-container {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  position: relative;
}

.actions {
  position: absolute;
  bottom: 4px;
  display: flex;
  gap: 4px;
  opacity: 0;
  transition: opacity 0.3s;
}

.cyber-cat-container:hover .actions {
  opacity: 1;
}

.action-btn {
  width: 24px;
  height: 24px;
  border: none;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.85);
  cursor: pointer;
  font-size: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
  transition: transform 0.15s;
}

.action-btn:hover {
  transform: scale(1.15);
}

.speak-panel {
  position: absolute;
  bottom: 32px;
  background: rgba(255, 255, 255, 0.95);
  border-radius: 8px;
  padding: 6px 8px;
  box-shadow: 0 2px 10px rgba(0, 0, 0, 0.15);
  width: 180px;
}

.speak-hint {
  font-size: 10px;
  color: #999;
  margin-bottom: 4px;
  font-family: "Noto Sans SC", "PingFang SC", sans-serif;
}

.speak-input-row {
  display: flex;
  gap: 4px;
}

.speak-input {
  flex: 1;
  border: 1px solid #ddd;
  border-radius: 4px;
  padding: 3px 6px;
  font-size: 12px;
  outline: none;
  font-family: "Noto Sans SC", "PingFang SC", sans-serif;
}

.speak-input:focus {
  border-color: #aaa;
}

.speak-btn {
  border: none;
  background: #555;
  color: white;
  border-radius: 4px;
  padding: 3px 8px;
  font-size: 11px;
  cursor: pointer;
  font-family: "Noto Sans SC", "PingFang SC", sans-serif;
}

.speak-btn:hover {
  background: #333;
}

.debug-panel {
  position: absolute;
  top: 2px;
  left: 2px;
  background: rgba(0, 0, 0, 0.7);
  border-radius: 4px;
  padding: 4px 6px;
  pointer-events: none;
}

.debug-row {
  display: flex;
  gap: 6px;
  font-family: monospace;
  font-size: 9px;
  color: #ccc;
  line-height: 1.4;
}

.slide-enter-active,
.slide-leave-active {
  transition: all 0.2s ease;
}
.slide-enter-from,
.slide-leave-to {
  opacity: 0;
  transform: translateY(8px);
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
