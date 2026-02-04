<script setup lang="ts">
import { cn } from "../../../lib/utils";

defineOptions({
  inheritAttrs: false,
});

interface TextareaProps {
  modelValue?: string;
  class?: string;
}

const props = defineProps<TextareaProps>();
const emit = defineEmits<{
  (event: "update:modelValue", value: string): void;
}>();

const onInput = (event: Event) => {
  emit("update:modelValue", (event.target as HTMLTextAreaElement).value);
};
</script>

<template>
  <textarea
    v-bind="$attrs"
    :value="props.modelValue"
    :class="cn(
      'flex min-h-[120px] w-full rounded-[calc(var(--radius)-2px)] border border-input bg-background px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background disabled:cursor-not-allowed disabled:opacity-50',
      props.class,
    )"
    @input="onInput"
  />
</template>
