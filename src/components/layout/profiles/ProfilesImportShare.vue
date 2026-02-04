<script setup lang="ts">
import { ref } from "vue";
import { useProxyStore } from "../../../stores/proxy";
import { Button } from "../../ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "../../ui/card";
import { Textarea } from "../../ui/textarea";

const store = useProxyStore();
const shareLinks = ref("");

const importShareLinks = async () => {
  const links = shareLinks.value
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  if (links.length === 0) return;
  await store.importShareLinks(links);
};
</script>

<template>
  <Card>
    <CardHeader>
      <CardTitle>Import share links</CardTitle>
      <p class="text-sm text-muted-foreground">
        Вставь несколько ссылок, по одной на строку.
      </p>
    </CardHeader>
    <CardContent>
      <Textarea
        v-model="shareLinks"
        rows="7"
        placeholder="vmess://..., vless://..., ss://..."
      />
      <Button
        class="mt-3 w-full"
        :disabled="store.profileBusy"
        @click="importShareLinks"
      >
        Импортировать
      </Button>
    </CardContent>
  </Card>
</template>
