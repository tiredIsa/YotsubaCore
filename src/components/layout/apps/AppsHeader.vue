<script setup lang="ts">
import { computed, ref } from "vue";
import { useProxyStore } from "../../../stores/proxy";
import { Button } from "../../ui/button";
import { Input } from "../../ui/input";

const store = useProxyStore();

const hintText = computed(() => {
  if (store.mode === "selected") {
    return "В режиме Selected по умолчанию всё direct. Включи Proxy для нужных процессов.";
  }
  if (store.mode === "full") {
    return "Full PC проксирует всё. Список Proxy пригодится для будущих исключений.";
  }
  return "В режиме Off процессы отображаются без применения правил.";
});

interface AppsHeaderProps {
  modelValue?: string;
}

const props = withDefaults(defineProps<AppsHeaderProps>(), {
  modelValue: "",
});

const emit = defineEmits<{
  (event: "update:modelValue", value: string): void;
}>();

const manualExe = ref("");

const canAddManualExe = computed(() => manualExe.value.trim().length > 0);

const addManualExe = () => {
  const value = manualExe.value.trim();
  if (!value) return;
  store.setProxy(value);
  manualExe.value = "";
};

const clearSearch = () => {
  emit("update:modelValue", "");
};
</script>

<template>
  <div class="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
    <div>
      <h2 class="text-2xl font-semibold text-foreground">Apps</h2>
      <p class="mt-1 text-sm text-muted-foreground">
        {{ hintText }}
      </p>
    </div>
    <div class="flex flex-wrap items-center gap-2">
      <Input
        v-model="manualExe"
        class="w-full min-w-[14rem] md:w-56"
        placeholder="Добавить .exe (например Discord.exe)"
        @keyup.enter="addManualExe"
      />
      <Button
        variant="outline"
        size="sm"
        :disabled="!canAddManualExe"
        @click="addManualExe"
      >
        В Proxy
      </Button>
      <Input
        :model-value="props.modelValue"
        class="w-full min-w-[14rem] md:w-64"
        placeholder="Search by name or path"
        @update:model-value="emit('update:modelValue', $event)"
      />
      <Button
        v-if="props.modelValue"
        variant="ghost"
        size="sm"
        @click="clearSearch"
      >
        Очистить
      </Button>
      <Button variant="ghost" size="sm" @click="store.refreshProcesses()">
        Обновить
      </Button>
      <Button variant="outline" size="sm" @click="store.clearAppRules()">
        Сбросить Proxy
      </Button>
    </div>
  </div>
</template>
