<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import { useProxyStore } from "../../../stores/proxy";
import { Button } from "../../ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "../../ui/card";

const store = useProxyStore();
const logBox = ref<HTMLDivElement | null>(null);

watch(
  () => store.logs.length,
  async () => {
    await nextTick();
    if (logBox.value) {
      logBox.value.scrollTop = logBox.value.scrollHeight;
    }
  },
);
</script>

<template>
  <Card>
    <CardHeader class="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
      <div>
        <CardTitle>Logs</CardTitle>
        <p class="text-sm text-muted-foreground">Последние строки из sing-box.</p>
      </div>
      <div class="flex flex-wrap gap-2">
        <Button variant="ghost" size="sm" @click="store.loadLogTail()"> Обновить </Button>
        <Button variant="outline" size="sm" @click="store.clearLogs()"> Очистить UI </Button>
      </div>
    </CardHeader>
    <CardContent>
      <div ref="logBox" class="overflow-auto rounded-[calc(var(--radius)-2px)] border border-border bg-muted/40 p-3 font-mono text-xs text-foreground">
        <div v-if="store.logs.length === 0" class="text-muted-foreground">Логов пока нет. Запусти режим, чтобы увидеть вывод sing-box.</div>
        <div v-else class="space-y-1">
          <div v-for="(line, idx) in store.logs" :key="idx">
            {{ line }}
          </div>
        </div>
      </div>
    </CardContent>
  </Card>
</template>
