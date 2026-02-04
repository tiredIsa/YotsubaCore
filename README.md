<table>
<tr>
<td width="70%">

# YotsubaCore

Десктопное приложение для управления прокси на базе [sing-box](https://github.com/SagerNet/sing-box).

Название отсылает к клану [Yotsuba](https://mafumafu.fandom.com/ru/wiki/Клан_Ёцуба) из новеллы *The Irregular at Magic High School*.

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

</td>
<td width="30%" align="center">

<img src="src-tauri/icons/128x128.png" width="128" alt="YotsubaCore">

</td>
</tr>
</table>

---

## Что это?

**YotsubaCore** — десктопное приложение для управления прокси на базе [sing-box](https://github.com/SagerNet/sing-box). Никакого сложного конфигурирования — всё интуитивно.

### Основные возможности

| Режим | Описание |
|-------|----------|
| **Off** | Прокси отключён — весь трафик идёт напрямую |
| **Selected** | Только выбранные приложения идут через прокси |
| **Full** | Весь трафик через прокси (кроме `.ru` доменов) |

- **Импорт профилей** — поддерживаются share-ссылки: `ss://`, `vmess://`, `vless://`, `trojan://`, `hysteria2://`, `tuic://`
- **Split tunneling** — точное управление маршрутизацией приложений
- **Bypass `.ru`** — российские домены всегда идут в обход
- **Автозапуск** — стартует вместе с Windows
- **Логи в реальном времени** — всё под контролем

---

## Быстрый старт

### Установка

1. Скачай последнюю версию из [Releases](../../releases)
2. Запусти `YotsubaCore-setup.exe`
3. Готово!

### Первая настройка

1. **Добавь профиль** — перейди в "Profiles" и вставь share-ссылку или JSON-конфиг
2. **Выбери режим** — OFF, Selected или Full в верхней панели
3. **(Опционально) Настрой приложения** — в режиме "Selected" выбери, какие программы использовать прокси

Вот и всё. Никаких `_config.json`, `_rules.yaml` и прочего.

---

## Скриншоты

<table>
<tr>
<td><img src="docs/screenshots/dashboard.png" alt="Dashboard"></td>
<td><img src="docs/screenshots/profiles.png" alt="Profiles"></td>
</tr>
<tr>
<td><i>Главная панель</i></td>
<td><i>Управление профилями</i></td>
</tr>
</table>

---

## Для контрибьютеров

### Архитектура

```
┌─────────────────────────────────────────────────────┐
│                    Frontend (Vue 3)                 │
│  ┌──────────┐  ┌──────────┐  ┌──────────────────┐  │
│  │ Dashboard│  │  Apps    │  │    Profiles      │  │
│  └──────────┘  └──────────┘  └──────────────────┘  │
└─────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────┐
│              Tauri Commands (Rust)                  │
│  ┌──────────────────────────────────────────────┐  │
│  │  • get_status / set_mode                     │  │
│  │  • list_processes                            │  │
│  │  • get_profiles / set_active_profile         │  │
│  │  • import_share_links / import_outbound_json │  │
│  └──────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────┐
│              sing-box (Core Proxy)                   │
└─────────────────────────────────────────────────────┘
```

### Стек

| Frontend | Backend |
|----------|---------|
| Vue 3.5 + TypeScript | Rust + Tauri 2 |
| Pinia (State) | sysinfo (processes) |
| Vue Router | windows-sys (Job Objects) |
| Tailwind CSS | serde/serde_json |

### Разработка

```bash
# Установка
bun install

# Dev-сервер (только frontend)
bun run dev

# Полная dev-сборка Tauri
bun run tauri dev

# Production сборка
bun run tauri build
```

### Структура проекта

```
src/                      # Frontend (Vue 3)
├── main.ts              # Entry point
├── App.vue              # Root component
├── router/              # Vue Router
├── stores/              # Pinia stores
├── view/                # Page components
└── components/          # Reusable components

src-tauri/
├── src/
│   └── lib.rs           # Core Rust logic (2173 LOC)
├── resources/
│   └── sing-box.exe     # sing-box binary (NOT in git)
└── tauri.conf.json      # Tauri config
```

### Tauri Commands

Основные команды в `src-tauri/src/lib.rs`:

| Command | Описание |
|---------|----------|
| `get_status()` | Текущий статус прокси |
| `set_mode(mode, rules)` | Применение режима и правил |
| `list_processes()` | Список запущенных процессов |
| `get_profiles()` | Получение всех профилей |
| `set_active_profile(id)` | Выбор активного профиля |
| `import_share_links(links)` | Парсинг share-ссылок |
| `read_log_tail()` | Чтение логов sing-box |

### Важные файлы

- `src-tauri/src/lib.rs:516` — `build_config()` — генерация конфига sing-box
- `src-tauri/src/lib.rs:1400` — Парсинг share-ссылок
- `src/stores/proxy.ts` — Pinia store для состояния прокси

---

## Лицензия

MIT
