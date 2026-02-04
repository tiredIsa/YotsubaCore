<script setup lang="ts">
import { computed } from "vue";
import { useProxyStore } from "../../../stores/proxy";
import { Button } from "../../ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "../../ui/card";

const store = useProxyStore();
const activeProfileLabel = computed(() => store.activeTag ?? "proxy");
</script>

<template>
  <Card>
    <CardHeader class="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
      <div>
        <CardTitle>Profiles</CardTitle>
        <p class="text-sm text-muted-foreground">
          Active: {{ activeProfileLabel }}
        </p>
      </div>
    </CardHeader>
    <CardContent>
      <div v-if="store.profiles.length === 0" class="text-sm text-muted-foreground">
        Пока нет профилей. Импортируй share-ссылку или JSON.
      </div>
      <div v-else class="grid gap-4 md:grid-cols-2">
        <article
          v-for="profile in store.profiles"
          :key="profile.tag"
          class="flex h-full flex-col justify-between gap-4 rounded-[calc(var(--radius)-2px)] border border-border bg-card/80 p-4"
          :class="{
            'border-primary/60 shadow-sm': store.activeTag === profile.tag,
          }"
        >
          <div>
            <h3 class="text-base font-semibold text-foreground">
              {{ profile.tag }}
            </h3>
            <p class="mt-1 text-sm text-muted-foreground">
              {{ profile.type }}
              <span v-if="profile.server">
                · {{ profile.server }}{{
                  profile.serverPort ? `:${profile.serverPort}` : ""
                }}
              </span>
            </p>
          </div>
          <div class="flex flex-wrap gap-2">
            <Button
              variant="outline"
              size="sm"
              :disabled="store.activeTag === profile.tag"
              @click="store.setActiveProfile(profile.tag)"
            >
              Активировать
            </Button>
            <Button variant="ghost" size="sm" @click="store.removeProfile(profile.tag)">
              Удалить
            </Button>
          </div>
        </article>
      </div>
    </CardContent>
  </Card>
</template>
