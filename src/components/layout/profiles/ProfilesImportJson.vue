<script setup lang="ts">
import { ref } from "vue";
import { useProxyStore } from "../../../stores/proxy";
import { Button } from "../../ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "../../ui/card";
import { Textarea } from "../../ui/textarea";

const store = useProxyStore();
const jsonPayload = ref("");

const importJson = async () => {
  const payload = jsonPayload.value.trim();
  if (!payload) return;
  await store.importJson(payload);
};
</script>

<template>
  <Card>
    <CardHeader>
      <CardTitle>Import JSON</CardTitle>
      <p class="text-sm text-muted-foreground">
        Поддерживаются outbounds или отдельный профиль.
      </p>
    </CardHeader>
    <CardContent>
      <Textarea
        v-model="jsonPayload"
        rows="7"
        placeholder='{"type":"shadowsocks", ...} или {"outbounds":[...]}'
      />
      <Button
        variant="outline"
        class="mt-3 w-full"
        :disabled="store.profileBusy"
        @click="importJson"
      >
        Импортировать JSON
      </Button>
    </CardContent>
  </Card>
</template>
