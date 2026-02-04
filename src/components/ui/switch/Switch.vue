<script setup lang="ts">
import { computed } from "vue";
import { cn } from "../../../lib/utils";

interface SwitchProps {
  modelValue?: boolean;
  disabled?: boolean;
  class?: string;
}

const props = withDefaults(defineProps<SwitchProps>(), {
  modelValue: false,
  disabled: false,
});

const emit = defineEmits<{
  (event: "update:modelValue", value: boolean): void;
}>();

const onToggle = () => {
  if (props.disabled) return;
  emit("update:modelValue", !props.modelValue);
};

const rootClass = computed(() =>
  cn(
    "inline-flex h-6 w-11 items-center rounded-full border border-border transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background disabled:cursor-not-allowed disabled:opacity-50",
    props.modelValue ? "bg-primary" : "bg-input",
    props.class,
  ),
);

const thumbClass = computed(() =>
  cn(
    "pointer-events-none block h-5 w-5 rounded-full bg-background shadow transition-transform",
    props.modelValue ? "translate-x-5" : "translate-x-0.5",
  ),
);
</script>

<template>
  <button
    type="button"
    role="switch"
    :aria-checked="props.modelValue"
    :disabled="props.disabled"
    :class="rootClass"
    @click="onToggle"
  >
    <span :class="thumbClass"></span>
  </button>
</template>