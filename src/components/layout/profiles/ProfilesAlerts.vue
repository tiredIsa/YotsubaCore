<script setup lang="ts">
import { useProxyStore } from "../../../stores/proxy";

const store = useProxyStore();
</script>

<template>
  <div class="space-y-3">
    <div
      v-if="store.profileError"
      class="rounded-[calc(var(--radius)-2px)] border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive"
    >
      {{ store.profileError }}
    </div>
    <ul
      v-if="store.profileWarnings.length"
      class="space-y-1 text-sm text-muted-foreground"
    >
      <li v-for="warning in store.profileWarnings" :key="warning">
        {{ warning }}
      </li>
    </ul>
  </div>
</template>
