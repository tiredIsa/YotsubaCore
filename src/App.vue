<script setup lang="ts">
import { watch } from "vue";
import AppShell from "./components/layout/AppShell.vue";

import { useWindowSize } from "@vueuse/core";

const { height } = useWindowSize();

watch(
  height,
  (newHeight) => {
    document.body.style.height = `${newHeight}px`;
  },
  { immediate: true },
);
</script>

<template class="h-full">
  <AppShell />
</template>
