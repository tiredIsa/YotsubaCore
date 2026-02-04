<script setup lang="ts">
import { useRoute } from "vue-router";
import { cn } from "../../lib/utils";

const route = useRoute();

const tabs = [
  { label: "Dashboard", to: "/dashboard", name: "dashboard" },
  { label: "Apps", to: "/apps", name: "apps" },
  { label: "Profiles", to: "/profiles", name: "profiles" },
  { label: "Logs", to: "/logs", name: "logs" },
];

const isActive = (tab: (typeof tabs)[number]) => route.name === tab.name;
</script>

<template>
  <nav class="w-full">
    <div
      class="flex w-full items-center gap-1 overflow-x-auto rounded-[var(--radius)] bg-muted p-1 text-muted-foreground"
    >
      <RouterLink
        v-for="tab in tabs"
        :key="tab.name"
        :to="tab.to"
        :class="
          cn(
            'inline-flex min-w-[7.5rem] items-center justify-center whitespace-nowrap rounded-[calc(var(--radius)-2px)] px-3 py-1.5 text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 ring-offset-background',
            isActive(tab)
              ? 'bg-background text-foreground shadow-sm'
              : 'text-muted-foreground hover:text-foreground',
          )
        "
        :aria-current="isActive(tab) ? 'page' : undefined"
      >
        {{ tab.label }}
      </RouterLink>
    </div>
  </nav>
</template>
