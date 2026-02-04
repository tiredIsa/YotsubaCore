<script setup lang="ts">
import { cn } from "../../../lib/utils";

interface CardHeaderProps {
  class?: string;
}

const props = defineProps<CardHeaderProps>();
</script>

<template>
  <div :class="cn('flex flex-col gap-2 p-5 sm:p-6', props.class)">
    <slot />
  </div>
</template>