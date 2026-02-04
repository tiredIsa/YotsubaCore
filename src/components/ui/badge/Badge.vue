<script setup lang="ts">
import { computed } from "vue";
import { cn } from "../../../lib/utils";

export type BadgeVariant = "default" | "secondary" | "outline" | "destructive";

interface BadgeProps {
  variant?: BadgeVariant;
  class?: string;
}

const props = withDefaults(defineProps<BadgeProps>(), {
  variant: "default",
});

const base =
  "inline-flex items-center rounded-full border border-transparent px-2.5 py-0.5 text-[0.65rem] font-semibold uppercase tracking-[0.2em]";

const variantClasses: Record<BadgeVariant, string> = {
  default: "bg-primary text-primary-foreground",
  secondary: "bg-secondary text-secondary-foreground",
  outline: "border-border text-foreground",
  destructive: "bg-destructive text-destructive-foreground",
};

const classes = computed(() =>
  cn(base, variantClasses[props.variant], props.class),
);
</script>

<template>
  <span :class="classes">
    <slot />
  </span>
</template>
