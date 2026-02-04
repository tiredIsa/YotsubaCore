import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  disable as disableAutostart,
  enable as enableAutostart,
  isEnabled as isAutostartEnabled,
} from "@tauri-apps/plugin-autostart";

export type ProxyMode = "off" | "selected" | "full";
export type AppRuleMode = "proxy" | "direct";

export interface AppRule {
  path: string;
  mode: AppRuleMode;
  name?: string;
}

export interface RunningProcess {
  name: string;
  path: string;
  count: number;
  pids: number[];
}

export interface AppListItem {
  name: string;
  path: string;
  running: boolean;
  count: number;
  mode: AppRuleMode;
}

export interface ProxyStatus {
  running: boolean;
  mode: ProxyMode;
  pid: number | null;
  lastExit: number | null;
  lastError: string | null;
  configPath: string | null;
  profilePath: string;
  logPath: string | null;
}

export interface ProfileData {
  outbounds: Record<string, unknown>[];
  activeTag: string | null;
}

export interface ImportResult {
  profile: ProfileData;
  added: number;
  errors: string[];
}

interface SavedState {
  lastMode: ProxyMode;
  appRules: AppRule[];
}

export interface ProfileItem {
  tag: string;
  type: string;
  server?: string;
  serverPort?: number;
  raw: Record<string, unknown>;
}

interface ProxyExitPayload {
  code: number | null;
}

interface LogPayload {
  lines: string[];
}

const LOG_LIMIT = 500;

const normalizePath = (value: string) => value.trim().replace(/^"|"$/g, "");

const isProcessName = (value: string) => {
  const trimmed = normalizePath(value);
  if (!trimmed) return false;
  if (/[\\/]/.test(trimmed)) return false;
  if (trimmed.includes(":")) return false;
  return true;
};

const fileName = (path: string) => {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts.length ? parts[parts.length - 1] : path;
};

const toProfileItem = (raw: Record<string, unknown>): ProfileItem => {
  const tag = String(raw.tag ?? "untagged");
  const type = String(raw.type ?? "unknown");
  const server =
    (raw.server as string | undefined) ??
    (raw.server_address as string | undefined) ??
    (raw.address as string | undefined);
  const serverPortRaw =
    (raw.server_port as number | string | undefined) ??
    (raw.port as number | string | undefined);
  const serverPort =
    typeof serverPortRaw === "number"
      ? serverPortRaw
      : serverPortRaw
        ? Number(serverPortRaw)
        : undefined;
  return { tag, type, server, serverPort, raw };
};

let processTimer: number | null = null;
let unlistenExit: (() => void) | null = null;
let unlistenLog: (() => void) | null = null;
let applyTimer: number | null = null;

const rulesSignature = (rules: AppRule[]) =>
  rules
    .slice()
    .sort((a, b) => a.path.localeCompare(b.path, "en", { sensitivity: "base" }))
    .map((rule) => `${rule.path}|${rule.mode}|${rule.name ?? ""}`)
    .join(";;");

export const useProxyStore = defineStore("proxy", {
  state: () => ({
    mode: "off" as ProxyMode,
    appRules: [] as AppRule[],
    processes: [] as RunningProcess[],
    logs: [] as string[],
    profiles: [] as ProfileItem[],
    activeTag: null as string | null,
    status: {
      running: false,
      mode: "off",
      pid: null,
      lastExit: null,
      lastError: null,
      configPath: null,
      profilePath: "",
      logPath: null,
    } as ProxyStatus,
    busy: false,
    error: null as string | null,
    profileBusy: false,
    profileError: null as string | null,
    profileWarnings: [] as string[],
    autostartEnabled: false,
    autostartBusy: false,
    autostartError: null as string | null,
    ready: false,
    lastAppliedMode: "off" as ProxyMode,
    lastAppliedRulesSignature: "",
  }),
  getters: {
    appList(state): AppListItem[] {
      const runningMap = new Map(
        state.processes.map((proc) => [proc.path, proc]),
      );
      const runningNameMap = new Map<string, { name: string; count: number }>();
      state.processes.forEach((proc) => {
        const key = proc.name.toLowerCase();
        const existing = runningNameMap.get(key);
        if (existing) {
          existing.count += proc.count;
          return;
        }
        runningNameMap.set(key, { name: proc.name, count: proc.count });
      });

      const proxyPaths = new Set<string>();
      const proxyNames = new Set<string>();
      state.appRules.forEach((rule) => {
        if (isProcessName(rule.path)) {
          proxyNames.add(rule.path.toLowerCase());
        } else {
          proxyPaths.add(rule.path);
        }
      });
      const items: AppListItem[] = [];

      state.appRules.forEach((rule) => {
        const isNameRule = isProcessName(rule.path);
        const running = isNameRule
          ? runningNameMap.get(rule.path.toLowerCase())
          : runningMap.get(rule.path);
        items.push({
          name: rule.name || running?.name || fileName(rule.path),
          path: rule.path,
          running: Boolean(running),
          count: running?.count ?? 0,
          mode: "proxy",
        });
      });

      state.processes.forEach((proc) => {
        if (proxyPaths.has(proc.path) || proxyNames.has(proc.name.toLowerCase())) {
          return;
        }
        items.push({
          name: proc.name || fileName(proc.path),
          path: proc.path,
          running: true,
          count: proc.count,
          mode: "direct",
        });
      });

      items.sort((a, b) =>
        a.name.localeCompare(b.name, "ru", { sensitivity: "base" }),
      );
      return items;
    },
  },
  actions: {
    setMode(mode: ProxyMode) {
      if (this.mode === mode) return;
      this.mode = mode;
      this.scheduleApply();
    },
    async init() {
      if (this.ready) return;
      this.ready = true;
      await this.loadSavedState();
      await Promise.all([
        this.refreshStatus(),
        this.loadProfiles(),
        this.refreshProcesses(),
        this.loadLogTail(),
        this.refreshAutostart(),
      ]);
      this.snapshotAppliedState();
      this.startProcessPolling();
      if (!unlistenExit) {
        unlistenExit = await listen<ProxyExitPayload>("proxy-exited", () => {
          this.refreshStatus();
        });
      }
      if (!unlistenLog) {
        unlistenLog = await listen<LogPayload>("proxy-log-batch", (event) => {
          this.appendLogs(event.payload.lines);
        });
      }
    },
    async refreshStatus() {
      const status = await invoke<ProxyStatus>("get_status");
      this.status = status;
      if (!this.busy) {
        this.mode = status.mode;
      }
    },
    async loadSavedState() {
      try {
        const saved = await invoke<SavedState>("get_saved_state");
        this.appRules = saved.appRules ?? [];
        if (!this.busy) {
          this.mode = saved.lastMode ?? "off";
        }
      } catch (err) {
        this.error = String(err ?? "Не удалось загрузить сохранённые настройки.");
      }
    },
    async refreshAutostart() {
      try {
        this.autostartEnabled = await isAutostartEnabled();
        this.autostartError = null;
      } catch (err) {
        this.autostartError = String(err ?? "Не удалось проверить автозапуск.");
      }
    },
    async setAutostart(enabled: boolean) {
      if (this.autostartBusy) return;
      if (enabled === this.autostartEnabled) return;
      this.autostartBusy = true;
      this.autostartError = null;
      try {
        if (enabled) {
          await enableAutostart();
        } else {
          await disableAutostart();
        }
      } catch (err) {
        this.autostartError = String(err ?? "Не удалось обновить автозапуск.");
      } finally {
        this.autostartBusy = false;
        await this.refreshAutostart();
      }
    },
    async refreshProcesses() {
      try {
        this.processes = await invoke<RunningProcess[]>("list_processes");
      } catch (err) {
        this.error = String(err ?? "Не удалось получить процессы.");
      }
    },
    startProcessPolling(intervalMs = 4000) {
      if (processTimer) return;
      processTimer = window.setInterval(() => {
        this.refreshProcesses();
      }, intervalMs);
    },
    stopProcessPolling() {
      if (processTimer) {
        window.clearInterval(processTimer);
        processTimer = null;
      }
    },
    appendLog(line: string) {
      if (!line) return;
      this.appendLogs([line]);
    },
    appendLogs(lines: string[]) {
      if (!lines?.length) return;
      for (const line of lines) {
        if (!line) continue;
        this.logs.push(line);
      }
      if (this.logs.length > LOG_LIMIT) {
        this.logs.splice(0, this.logs.length - LOG_LIMIT);
      }
    },
    clearLogs() {
      this.logs = [];
    },
    snapshotAppliedState() {
      this.lastAppliedMode = this.mode;
      this.lastAppliedRulesSignature = rulesSignature(this.appRules);
    },
    scheduleApply(delayMs = 350) {
      if (!this.ready) return;
      if (applyTimer) {
        window.clearTimeout(applyTimer);
      }
      applyTimer = window.setTimeout(async () => {
        applyTimer = null;
        const signature = rulesSignature(this.appRules);
        const modeChanged = this.mode !== this.lastAppliedMode;
        const rulesChanged = signature !== this.lastAppliedRulesSignature;
        if (!modeChanged && !rulesChanged) return;
        if (this.busy) {
          this.scheduleApply(500);
          return;
        }
        await this.applyMode();
      }, delayMs);
    },
    cancelScheduledApply() {
      if (!applyTimer) return;
      window.clearTimeout(applyTimer);
      applyTimer = null;
    },
    async loadLogTail(limit = 200) {
      try {
        const lines = await invoke<string[]>("read_log_tail", { limit });
        this.logs = lines.slice(-LOG_LIMIT);
      } catch (err) {
        this.error = String(err ?? "Не удалось прочитать лог.");
      }
    },
    setProxy(path: string, name?: string) {
      const normalized = normalizePath(path);
      if (!normalized) return;
      const existing = this.appRules.find((item) => item.path === normalized);
      if (existing) {
        existing.mode = "proxy";
        if (name) existing.name = name;
        this.scheduleApply();
        return;
      }
      this.appRules.push({ path: normalized, mode: "proxy", name });
      this.scheduleApply();
    },
    setDirect(path: string) {
      const normalized = normalizePath(path);
      if (!normalized) return;
      const nextRules = this.appRules.filter((item) => item.path !== normalized);
      if (nextRules.length === this.appRules.length) return;
      this.appRules = nextRules;
      this.scheduleApply();
    },
    clearAppRules() {
      if (this.appRules.length === 0) return;
      this.appRules = [];
      this.scheduleApply();
    },
    async applyMode() {
      const signature = rulesSignature(this.appRules);
      if (this.busy) return;
      if (
        this.mode === this.lastAppliedMode &&
        signature === this.lastAppliedRulesSignature
      ) {
        return;
      }
      this.busy = true;
      this.error = null;
      try {
        const status = await invoke<ProxyStatus>("set_mode", {
          mode: this.mode,
          appRules: this.appRules,
        });
        this.status = status;
        this.lastAppliedMode = this.mode;
        this.lastAppliedRulesSignature = signature;
      } catch (err) {
        const message = String(err ?? "");
        if (message.startsWith("PROFILE_MISSING|")) {
          const path = message.split("|")[1] ?? "";
          this.error = `Создан шаблон профиля: ${path}. Заполни его и повтори.`;
        } else if (message.startsWith("PROFILE_INVALID|")) {
          this.error = `Профиль некорректен: ${message.split("|")[1] ?? ""}`;
        } else if (message.startsWith("PROFILE_PROXY_TAG_MISSING|")) {
          this.error =
            "В профиле нет outbound с tag=proxy и нет активного профиля.";
        } else if (message.startsWith("PROFILE_OUTBOUNDS_MISSING|")) {
          this.error = "В профиле нет outbounds. Добавь профиль и повтори.";
        } else if (message.startsWith("SINGBOX_MISSING|")) {
          const path = message.split("|")[1] ?? "";
          this.error = `Не найден sing-box.exe: ${path}`;
        } else {
          this.error = message || "Не удалось применить режим.";
        }
      } finally {
        this.busy = false;
        await this.refreshStatus();
        await this.loadLogTail();
      }
    },
    async loadProfiles() {
      const profile = await invoke<ProfileData>("get_profiles");
      this.activeTag = profile.activeTag;
      this.profiles = (profile.outbounds ?? [])
        .filter((item) => item && typeof item === "object")
        .map((item) => toProfileItem(item));
    },
    async setActiveProfile(tag: string) {
      this.profileError = null;
      try {
        const profile = await invoke<ProfileData>("set_active_profile", { tag });
        this.activeTag = profile.activeTag;
        this.profiles = (profile.outbounds ?? [])
          .filter((item) => item && typeof item === "object")
          .map((item) => toProfileItem(item));
        this.scheduleApply();
      } catch (err) {
        this.profileError = String(err ?? "Не удалось выбрать профиль.");
      }
    },
    async removeProfile(tag: string) {
      this.profileError = null;
      try {
        const profile = await invoke<ProfileData>("remove_outbound", { tag });
        this.activeTag = profile.activeTag;
        this.profiles = (profile.outbounds ?? [])
          .filter((item) => item && typeof item === "object")
          .map((item) => toProfileItem(item));
        this.scheduleApply();
      } catch (err) {
        this.profileError = String(err ?? "Не удалось удалить профиль.");
      }
    },
    async importShareLinks(links: string[]) {
      this.profileBusy = true;
      this.profileError = null;
      this.profileWarnings = [];
      try {
        const result = await invoke<ImportResult>("import_share_links", {
          links,
        });
        this.activeTag = result.profile.activeTag;
        this.profiles = (result.profile.outbounds ?? [])
          .filter((item) => item && typeof item === "object")
          .map((item) => toProfileItem(item));
        this.profileWarnings = result.errors ?? [];
        this.scheduleApply();
      } catch (err) {
        this.profileError = String(err ?? "Не удалось импортировать ссылки.");
      } finally {
        this.profileBusy = false;
      }
    },
    async importJson(payload: string) {
      this.profileBusy = true;
      this.profileError = null;
      this.profileWarnings = [];
      try {
        const result = await invoke<ImportResult>("import_outbound_json", {
          payload,
        });
        this.activeTag = result.profile.activeTag;
        this.profiles = (result.profile.outbounds ?? [])
          .filter((item) => item && typeof item === "object")
          .map((item) => toProfileItem(item));
        this.profileWarnings = result.errors ?? [];
        this.scheduleApply();
      } catch (err) {
        this.profileError = String(err ?? "Не удалось импортировать JSON.");
      } finally {
        this.profileBusy = false;
      }
    },
  },
});
