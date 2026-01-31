<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import catSheet from "../assets/sprites/cat-sheet.png";

// ── Sprite Sheet 参数 ──
const FRAME_SIZE = 16;
const SCALE = 4;
const DISPLAY_SIZE = FRAME_SIZE * SCALE;
const SHEET_COLS = 8;

// 动画定义：每个动画对应 sprite sheet 中的行和帧数
const ANIMATIONS: Record<string, { row: number; frames: number; interval: number }> = {
  idle:  { row: 0, frames: 4, interval: 300 },
  walk:  { row: 1, frames: 8, interval: 150 },
  sleep: { row: 2, frames: 8, interval: 400 },
  alert: { row: 3, frames: 5, interval: 250 },
  sit:   { row: 4, frames: 3, interval: 500 },
  run:   { row: 5, frames: 4, interval: 120 },
};

type AnimState = keyof typeof ANIMATIONS;

// ── 后端状态类型 ──
interface SophieState {
  energy: number;
  hunger: number;
  sleepiness: number;
  emotion: string;
  trust: number;
  intimacy: number;
  understanding: number;
  isSleeping: boolean;
  behavior: AnimState;
  flipDirection: boolean;
  minutesSinceInteraction: number;
}

// ── 响应式状态 ──
const animationState = ref<AnimState>("idle");
const frame = ref(0);
const flipped = ref(false);
const sophieData = ref<SophieState | null>(null);

const emit = defineEmits<{
  stateUpdate: [state: SophieState];
}>();

let frameTimer: number | undefined;
let unlisten: UnlistenFn | undefined;

// ── Sprite 渲染 ──
const totalFrames = computed(() => ANIMATIONS[animationState.value]?.frames ?? 4);

const spriteStyle = computed(() => {
  const anim = ANIMATIONS[animationState.value];
  if (!anim) return {};
  return {
    width: `${DISPLAY_SIZE}px`,
    height: `${DISPLAY_SIZE}px`,
    backgroundImage: `url(${catSheet})`,
    backgroundSize: `${SHEET_COLS * DISPLAY_SIZE}px auto`,
    backgroundPosition: `-${frame.value * DISPLAY_SIZE}px -${anim.row * DISPLAY_SIZE}px`,
    backgroundRepeat: "no-repeat",
    imageRendering: "pixelated" as const,
  };
});

// ── 帧循环 ──
function startFrameLoop() {
  stopFrameLoop();
  const interval = ANIMATIONS[animationState.value]?.interval ?? 300;
  frameTimer = window.setInterval(() => {
    frame.value = (frame.value + 1) % totalFrames.value;
  }, interval);
}

function stopFrameLoop() {
  if (frameTimer !== undefined) {
    clearInterval(frameTimer);
    frameTimer = undefined;
  }
}

function switchAnimation(state: AnimState) {
  if (state === animationState.value) return;
  animationState.value = state;
  frame.value = 0;
  startFrameLoop();
}

// ── 处理后端状态更新 ──
function handleStateUpdate(state: SophieState) {
  sophieData.value = state;
  emit("stateUpdate", state);

  // 行为 → 动画映射
  const behavior = state.behavior;
  if (behavior && ANIMATIONS[behavior]) {
    switchAnimation(behavior);
  }

  // 方向
  if (state.flipDirection !== undefined) {
    flipped.value = state.flipDirection;
  }
}

// ── 用户交互 ──
async function onSophieClick() {
  try {
    const state = await invoke<SophieState>("click_sophie");
    handleStateUpdate(state);
  } catch (e) {
    // fallback: 本地切换到 alert
    switchAnimation("alert");
    setTimeout(() => switchAnimation("idle"), 3000);
  }
}

// ── 生命周期 ──
onMounted(async () => {
  startFrameLoop();

  // 初始获取状态
  try {
    const state = await invoke<SophieState>("get_sophie_state");
    handleStateUpdate(state);
  } catch (_) {
    // Tauri 未就绪时用默认状态
  }

  // 监听后端广播的状态更新
  try {
    unlisten = await listen<SophieState>("sophie-update", (event) => {
      handleStateUpdate(event.payload);
    });
  } catch (_) {
    // 非 Tauri 环境（如浏览器预览）时忽略
  }
});

onUnmounted(() => {
  stopFrameLoop();
  if (unlisten) {
    unlisten();
  }
});

// 暴露给父组件
defineExpose({ sophieData });
</script>

<template>
  <div class="sophie" :class="{ flipped }" @click="onSophieClick">
    <div class="sprite" :style="spriteStyle"></div>
  </div>
</template>

<style scoped>
.sophie {
  cursor: pointer;
  image-rendering: pixelated;
}

.sophie.flipped {
  transform: scaleX(-1);
}

.sprite {
  image-rendering: pixelated;
  -webkit-image-rendering: pixelated;
}
</style>
