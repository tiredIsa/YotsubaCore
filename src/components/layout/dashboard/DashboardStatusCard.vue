<script setup lang="ts">
import { computed } from "vue";
import { useProxyStore, type ProxyMode } from "../../../stores/proxy";
import { Badge } from "../../ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "../../ui/card";

const store = useProxyStore();

const modeLabel = (mode: ProxyMode) => {
  if (mode === "off") return "Off";
  if (mode === "selected") return "Selected apps";
  return "Full PC";
};

const statusBadge = computed(() =>
  store.status.running ? "default" : "secondary",
);
</script>

<template>
  <Card>
    <CardHeader class="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
      <div>
        <CardTitle>Status</CardTitle>
        <p class="text-sm text-muted-foreground">Applied state from sing-box.</p>
      </div>
      <Badge :variant="statusBadge">
        {{ store.status.running ? "RUNNING" : "IDLE" }}
      </Badge>
    </CardHeader>
    <CardContent>
      <div class="grid gap-4 sm:grid-cols-2">
        <div>
          <div class="text-[0.65rem] uppercase tracking-[0.2em] text-muted-foreground">
            Applied mode
          </div>
          <div class="mt-1 text-sm font-medium text-foreground">
            {{ modeLabel(store.status.mode) }}
          </div>
        </div>
        <div>
          <div class="text-[0.65rem] uppercase tracking-[0.2em] text-muted-foreground">
            Active profile
          </div>
          <div class="mt-1 break-all text-sm font-medium text-foreground">
            {{ store.activeTag ?? "proxy" }}
          </div>
        </div>
        <div>
          <div class="text-[0.65rem] uppercase tracking-[0.2em] text-muted-foreground">
            PID
          </div>
          <div class="mt-1 text-sm font-medium text-foreground">
            {{ store.status.pid ?? "—" }}
          </div>
        </div>
        <div>
          <div class="text-[0.65rem] uppercase tracking-[0.2em] text-muted-foreground">
            Last exit
          </div>
          <div class="mt-1 text-sm font-medium text-foreground">
            {{ store.status.lastExit ?? "—" }}
          </div>
        </div>
      </div>
      <div
        v-if="store.status.lastError"
        class="mt-4 rounded-[calc(var(--radius)-2px)] border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive"
      >
        {{ store.status.lastError }}
      </div>
    </CardContent>
  </Card>
</template>
