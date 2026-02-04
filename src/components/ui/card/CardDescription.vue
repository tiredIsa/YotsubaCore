<script setup lang="ts">
import { cn } from "../../../lib/utils";

interface CardDescriptionProps {
  class?: string;
}

const props = defineProps<CardDescriptionProps>();
</script>

<template>
  <p :class="cn('text-sm text-muted-foreground', props.class)">
    <slot />
  </p>
</template>
