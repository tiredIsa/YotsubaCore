<script setup lang="ts">
import { computed, ref } from "vue";
import AppsHeader from "../components/layout/apps/AppsHeader.vue";
import AppsProxyList from "../components/layout/apps/AppsProxyList.vue";
import AppsRunningList from "../components/layout/apps/AppsRunningList.vue";

const search = ref("");
const normalizedSearch = computed(() => search.value.trim().toLowerCase());
</script>

<template>
  <section class="flex flex-col gap-6">
    <AppsHeader v-model="search" />
    <div class="grid gap-6 lg:grid-cols-2">
      <AppsProxyList :search="normalizedSearch" />
      <AppsRunningList :search="normalizedSearch" />
    </div>
  </section>
</template>
