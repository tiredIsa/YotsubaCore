<script setup lang="ts">
import { computed } from "vue";
import { useProxyStore, type ProxyMode } from "../../stores/proxy";
import { useTheme } from "../../lib/theme";
import { cn } from "../../lib/utils";
import { Badge } from "../ui/badge";
import { Switch } from "../ui/switch";

const store = useProxyStore();
const { isDark, setTheme } = useTheme();

const statusBadge = computed(() => (store.status.running ? "default" : "secondary"));

const localProxyActive = computed(() => store.status.running && store.status.mode !== "off");
const localProxyBadge = computed(() => (localProxyActive.value ? "default" : "secondary"));

const profileLabel = computed(() => store.activeTag ?? "proxy");

const modeOptions: Array<{ value: ProxyMode; label: string }> = [
  { value: "off", label: "Off" },
  { value: "selected", label: "Selected" },
  { value: "full", label: "Full" },
];

const modeButtonClass = (value: ProxyMode) =>
  cn(
    "rounded-[calc(var(--radius)-2px)] px-3 py-1.5 text-xs font-semibold transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
    store.mode === value ? "bg-primary text-primary-foreground" : "text-muted-foreground hover:text-foreground",
  );

const onThemeChange = (value: boolean) => {
  setTheme(value ? "dark" : "light");
};

const onAutostartChange = (value: boolean) => {
  store.setAutostart(value);
};
</script>

<template>
  <header class="flex flex-col gap-3">
    <div class="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
      <div class="flex flex-wrap items-center gap-3 h-full">
        <div class="flex flex-col items-start justify-between h-full py-1">
          <Badge :variant="statusBadge">
            {{ store.status.running ? "RUNNING" : "DISABLED" }}
          </Badge>
          <Badge :variant="localProxyBadge">
            {{ "****:2080" }}
          </Badge>
        </div>
        <div class="rounded-[calc(var(--radius)-2px)] border border-border bg-card/60 px-3 py-2">
          <p class="text-[0.65rem] uppercase tracking-[0.2em] text-muted-foreground">Profile</p>
          <p class="text-sm font-semibold text-foreground">{{ profileLabel }}</p>
        </div>
        <div v-if="store.status.pid" class="rounded-[calc(var(--radius)-2px)] border border-border bg-card/60 px-3 py-2">
          <p class="text-[0.65rem] uppercase tracking-[0.2em] text-muted-foreground">PID</p>
          <p class="text-sm font-semibold text-foreground">
            {{ store.status.pid }}
          </p>
        </div>
      </div>
      <div class="flex flex-wrap items-center gap-3">
        <div class="inline-flex items-center gap-1 rounded-[var(--radius)] border border-border bg-muted p-1" role="group" aria-label="Mode">
          <button
            v-for="option in modeOptions"
            :key="option.value"
            type="button"
            :class="modeButtonClass(option.value)"
            :aria-pressed="store.mode === option.value"
            @click="store.setMode(option.value)"
          >
            {{ option.label }}
          </button>
        </div>
        <div class="flex items-center gap-3 rounded-[calc(var(--radius)-2px)] border border-border bg-card/60 px-3 py-2">
          <div>
            <p class="text-[0.65rem] uppercase tracking-[0.2em] text-muted-foreground">Theme</p>
            <p class="text-xs font-medium text-foreground">Light / Dark</p>
          </div>
          <Switch aria-label="Toggle theme" :model-value="isDark" @update:model-value="onThemeChange" />
        </div>
        <div class="flex items-center gap-3 rounded-[calc(var(--radius)-2px)] border border-border bg-card/60 px-3 py-2">
          <div>
            <p class="text-[0.65rem] uppercase tracking-[0.2em] text-muted-foreground">Startup</p>
            <p class="text-xs font-medium text-foreground">Старт с Windows</p>
          </div>
          <Switch
            aria-label="Toggle autostart"
            :model-value="store.autostartEnabled"
            :disabled="store.autostartBusy"
            @update:model-value="onAutostartChange"
          />
        </div>
        <span v-if="store.busy" class="text-xs text-muted-foreground"> Saving… </span>
      </div>
    </div>
  </header>
</template>
