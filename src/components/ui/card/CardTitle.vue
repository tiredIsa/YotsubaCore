<script setup lang="ts">
import { cn } from "../../../lib/utils";

interface CardTitleProps {
  class?: string;
}

const props = defineProps<CardTitleProps>();
</script>

<template>
  <h3 :class="cn('text-lg font-semibold leading-none', props.class)">
    <slot />
  </h3>
</template>