<script setup lang="ts">
defineProps<{
  text: string | null;
}>();
</script>

<template>
  <Transition name="thought">
    <div v-if="text" class="thought-bubble">
      <span class="thought-icon">💭</span>
      <span class="thought-text">{{ text }}</span>
    </div>
  </Transition>
</template>

<style scoped>
.thought-bubble {
  position: absolute;
  top: 0;
  right: -8px;
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 10px;
  background: rgba(255, 255, 255, 0.9);
  border-radius: 12px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
  white-space: nowrap;
  pointer-events: none;
}

.thought-icon {
  font-size: 14px;
}

.thought-text {
  font-family: "Noto Sans SC", "PingFang SC", sans-serif;
  font-size: 13px;
  color: #444;
}

/* 进入/离开动画 */
.thought-enter-active {
  animation: bubble-in 0.4s ease-out;
}

.thought-leave-active {
  animation: bubble-out 0.3s ease-in;
}

@keyframes bubble-in {
  0% {
    opacity: 0;
    transform: translateY(8px) scale(0.8);
  }
  100% {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}

@keyframes bubble-out {
  0% {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
  100% {
    opacity: 0;
    transform: translateY(-4px) scale(0.9);
  }
}
</style>
