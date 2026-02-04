<script setup lang="ts">
import { cn } from "../../../lib/utils";

interface CardFooterProps {
  class?: string;
}

const props = defineProps<CardFooterProps>();
</script>

<template>
  <div :class="cn('flex items-center justify-end gap-2 px-5 pb-5 sm:px-6 sm:pb-6', props.class)">
    <slot />
  </div>
</template>