<script setup lang="ts">
import { cn } from "../../../lib/utils";

defineOptions({
  inheritAttrs: false,
});

interface SeparatorProps {
  orientation?: "horizontal" | "vertical";
  class?: string;
}

const props = withDefaults(defineProps<SeparatorProps>(), {
  orientation: "horizontal",
});
</script>

<template>
  <div
    v-bind="$attrs"
    :class="cn(
      'shrink-0 bg-border',
      props.orientation === 'horizontal' ? 'h-px w-full' : 'h-full w-px',
      props.class,
    )"
    role="separator"
    :aria-orientation="props.orientation"
  />
</template>
