<script setup lang="ts">
import { useProxyStore } from "../../stores/proxy";

const store = useProxyStore();
</script>

<template>
  <div
    v-if="store.error || store.autostartError"
    class="rounded-[calc(var(--radius)-2px)] border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive"
  >
    <p v-if="store.error">{{ store.error }}</p>
    <p v-if="store.autostartError">{{ store.autostartError }}</p>
  </div>
</template>
