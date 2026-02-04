<script setup lang="ts">
import { onBeforeUnmount, onMounted } from "vue";
import { RouterView } from "vue-router";
import { useProxyStore } from "../../stores/proxy";
import AppTopbar from "./AppTopbar.vue";
import AppTabs from "./AppTabs.vue";
import AppNotice from "./AppNotice.vue";

const store = useProxyStore();

onMounted(async () => {
  await store.init();
});

onBeforeUnmount(() => {
  store.stopProcessPolling();
  store.cancelScheduledApply();
});
</script>

<template>
  <main class="h-full bg-background text-foreground transition-colors">
    <div class="mx-auto flex w-full max-w-6xl flex-col gap-6 px-6 pb-16 pt-10 md:px-10 lg:px-16 h-full">
      <AppTopbar />
      <AppTabs />
      <AppNotice />
      <RouterView />
    </div>
  </main>
</template>
