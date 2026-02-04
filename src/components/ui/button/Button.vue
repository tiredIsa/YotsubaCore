<script setup lang="ts">
import { computed } from "vue";
import { Primitive } from "../primitive";
import { cn } from "../../../lib/utils";

export type ButtonVariant =
  | "default"
  | "secondary"
  | "outline"
  | "ghost"
  | "destructive"
  | "link";
export type ButtonSize = "default" | "sm" | "lg" | "icon";

interface ButtonProps {
  variant?: ButtonVariant;
  size?: ButtonSize;
  asChild?: boolean;
  type?: "button" | "submit" | "reset";
  class?: string;
}

const props = withDefaults(defineProps<ButtonProps>(), {
  variant: "default",
  size: "default",
  asChild: false,
  type: "button",
});

const base =
  "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-[calc(var(--radius)-2px)] text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background disabled:pointer-events-none disabled:opacity-50";

const variantClasses: Record<ButtonVariant, string> = {
  default: "bg-primary text-primary-foreground hover:bg-primary/90",
  secondary: "bg-secondary text-secondary-foreground hover:bg-secondary/80",
  outline:
    "border border-input bg-background hover:bg-accent hover:text-accent-foreground",
  ghost: "hover:bg-accent hover:text-accent-foreground",
  destructive: "bg-destructive text-destructive-foreground hover:bg-destructive/90",
  link: "text-primary underline-offset-4 hover:underline",
};

const sizeClasses: Record<ButtonSize, string> = {
  default: "h-10 px-4",
  sm: "h-9 px-3 text-xs",
  lg: "h-11 px-6 text-base",
  icon: "h-10 w-10",
};

const classes = computed(() =>
  cn(base, variantClasses[props.variant], sizeClasses[props.size], props.class),
);
</script>

<template>
  <Primitive
    as="button"
    :as-child="props.asChild"
    :type="props.asChild ? undefined : props.type"
    :class="classes"
  >
    <slot />
  </Primitive>
</template>
