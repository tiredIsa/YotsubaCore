<script setup lang="ts">
import { computed } from "vue";
import { useProxyStore } from "../../../stores/proxy";
import { Badge } from "../../ui/badge";
import { Button } from "../../ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "../../ui/card";

const store = useProxyStore();

interface AppsListProps {
  search?: string;
}

const props = withDefaults(defineProps<AppsListProps>(), {
  search: "",
});

const proxyApps = computed(() => store.appList.filter((item) => item.mode === "proxy"));

const hasQuery = computed(() => props.search.trim().length > 0);

const filteredApps = computed(() => {
  const query = props.search.trim().toLowerCase();
  if (!query) return proxyApps.value;
  return proxyApps.value.filter((item) => item.name.toLowerCase().includes(query) || item.path.toLowerCase().includes(query));
});
</script>

<template>
  <Card>
    <CardHeader class="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
      <div>
        <CardTitle>Proxy pinned</CardTitle>
        <p class="text-xs text-muted-foreground">
          <span v-if="hasQuery"> Найдено {{ filteredApps.length }} из {{ proxyApps.length }} </span>
          <span v-else>Показываются всегда</span>
        </p>
      </div>
      <Badge variant="secondary">
        {{ filteredApps.length }}<span v-if="hasQuery"> / {{ proxyApps.length }}</span>
      </Badge>
    </CardHeader>
    <CardContent>
      <div v-if="proxyApps.length === 0" class="text-sm text-muted-foreground">Никто не закреплен в Proxy.</div>
      <div v-else-if="filteredApps.length === 0" class="text-sm text-muted-foreground">Ничего не найдено по запросу.</div>
      <div v-else class="max-h-[32rem] overflow-auto pr-1 lg:max-h-[36rem]">
        <ul class="space-y-3">
          <li v-for="app in filteredApps" :key="app.path" class="rounded-[calc(var(--radius)-2px)] border border-border bg-background/80 p-3">
            <div class="flex flex-col gap-3">
              <div class="flex flex-col gap-2">
                <div class="flex items-center gap-2">
                  <span class="font-medium text-foreground">{{ app.name }}</span>
                  <Badge :variant="app.running ? 'default' : 'secondary'">
                    {{ app.running ? `RUNNING${app.count > 1 ? ` ×${app.count}` : ""}` : "OFF" }}
                  </Badge>
                </div>
                <span class="break-all font-mono text-xs text-muted-foreground">
                  {{ app.path }}
                </span>
              </div>
              <div class="flex flex-wrap gap-2">
                <Button variant="outline" size="sm" @click="store.setDirect(app.path)"> Direct </Button>
              </div>
            </div>
          </li>
        </ul>
      </div>
    </CardContent>
  </Card>
</template>
