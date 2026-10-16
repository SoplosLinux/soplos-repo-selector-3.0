<script lang="ts">
  import type { ComponentType } from "svelte";
  import { getContext } from "svelte";
  import { theme, toggleTheme } from "../lib/theme";
  import { Sun, Moon, Globe } from "lucide-svelte";
  import { _, locale } from "svelte-i18n";
  import { SUPPORTED_LOCALES, LOCALE_NAMES, setLocale, type SupportedLocale } from "../i18n";

  function onLanguageChange(e: Event) {
    setLocale((e.currentTarget as HTMLSelectElement).value as SupportedLocale);
  }

  export let tabs: { id: string; icon: ComponentType; label: string }[];
  export let active: string;
  export let onSelect: (id: string) => void;
  export let appVersion: string;

  const openAbout = getContext<() => void>("openAbout");

  // "3.0.0-1-beta" → "3.0.0-beta" — the footer shows the app version, not
  // the Debian package revision (same rule as Welcome's and Sys Cleaner's
  // Sidebar).
  $: cleanVersion = appVersion.replace(/-\d+-/, "-");
</script>

<aside class="w-[230px] shrink-0 flex flex-col pt-[18px] px-[10px] pb-2 bg-panel border-r border-border">
  <div class="flex items-center gap-2 px-3 pb-[22px] font-bold text-[0.98rem] text-text-bright">
    <img src="/assets/icons/org.soplos.reposelector.png" alt="Soplos" class="w-[28px] h-[28px] object-contain shrink-0" />
    <span class="whitespace-nowrap">Repo Selector</span>
  </div>

  <nav class="flex flex-col gap-[3px] flex-1">
    {#each tabs as tab}
      <button
        class="flex items-center gap-3 px-3.5 py-2.5 rounded text-[0.92rem] text-left transition-colors
          {active === tab.id
          ? 'bg-primary/15 text-primary font-semibold'
          : 'text-text-muted hover:bg-surface hover:text-text-bright'}"
        on:click={() => onSelect(tab.id)}
      >
        <svelte:component this={tab.icon} size={18} class="shrink-0" />
        <span>{tab.label}</span>
      </button>
    {/each}
  </nav>

  <div class="mt-3 flex flex-col gap-2.5">
    <div class="flex items-center gap-1.5 px-1">
      <Globe size={13} class="text-text-muted shrink-0" />
      <select
        class="appearance-none flex-1 min-w-0 bg-surface-2 border border-border rounded px-1.5 py-1 text-[0.68rem] text-text-bright outline-none focus-visible:ring-2 focus-visible:ring-primary/60"
        value={$locale}
        title={$_("sidebar.language")}
        on:change={onLanguageChange}
      >
        {#each SUPPORTED_LOCALES as code}
          <option value={code}>{LOCALE_NAMES[code]}</option>
        {/each}
      </select>
    </div>
    <div class="pt-2.5 border-t border-border flex items-center gap-2 px-1">
      <button
        class="p-1 rounded text-text-muted hover:bg-surface hover:text-text-bright shrink-0"
        on:click={toggleTheme}
        title={$theme === "dark" ? $_("sidebar.switchToLight") : $_("sidebar.switchToDark")}
      >
        <svelte:component this={$theme === "dark" ? Moon : Sun} size={14} />
      </button>
      <button
        class="text-[0.62rem] text-text-muted hover:text-text-bright hover:underline"
        on:click={openAbout}
        title={$_("sidebar.aboutTooltip")}
      >
        {cleanVersion}
      </button>
    </div>
  </div>
</aside>
