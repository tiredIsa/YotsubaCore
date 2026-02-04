import { computed, ref, watch } from "vue";

type Theme = "light" | "dark";

const THEME_KEY = "yotsuba-theme";
const theme = ref<Theme>("light");
let initialized = false;

const getPreferredTheme = (): Theme => {
  if (typeof window === "undefined") return "light";
  const stored = localStorage.getItem(THEME_KEY);
  if (stored === "light" || stored === "dark") return stored;
  return window.matchMedia?.("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
};

const applyTheme = (value: Theme) => {
  const root = document.documentElement;
  root.classList.toggle("dark", value === "dark");
  root.dataset.theme = value;
  root.style.colorScheme = value;
};

export const initTheme = () => {
  if (initialized || typeof window === "undefined") return;
  theme.value = getPreferredTheme();
  applyTheme(theme.value);
  watch(theme, (value) => {
    applyTheme(value);
    localStorage.setItem(THEME_KEY, value);
  });
  initialized = true;
};

export const useTheme = () => {
  if (!initialized) initTheme();
  const isDark = computed(() => theme.value === "dark");
  const setTheme = (value: Theme) => {
    theme.value = value;
  };
  const toggleTheme = () => {
    theme.value = theme.value === "dark" ? "light" : "dark";
  };

  return {
    theme,
    isDark,
    setTheme,
    toggleTheme,
  };
};
